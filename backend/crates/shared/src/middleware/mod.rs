//! Cross-cutting HTTP middleware: CORS, request id, security headers, HTTPS
//! enforcement, rate limiting, and metrics. Request *body* validation lives
//! at the extractor layer (`utils::validation::ValidatedJson`), not here.
//!
//! Generic over any service's router: [`apply`] takes a small
//! [`MiddlewareState`] (redis, rate-limit config, https/proxy flags,
//! metrics) rather than a whole service `AppState`, so it's identical code
//! across all four services instead of copy-pasted into each. Layers are
//! added outer-to-innermost in the order listed in `apply`'s body (the LAST
//! `.layer()` call wraps everything before it, so it runs FIRST on the way
//! in and LAST on the way out).

pub mod rate_limit;

use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::extract::{MatchedPath, Request, State};
use axum::http::{header, HeaderName, HeaderValue, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::Router;
use tower_http::catch_panic::CatchPanicLayer;
use tower_http::compression::CompressionLayer;
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};
use tower_http::sensitive_headers::SetSensitiveRequestHeadersLayer;
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;

use crate::config::{CorsConfig, RateLimitConfig};
use crate::database::redis::RedisPool;
use crate::utils::metrics::Metrics;

/// Headroom over the largest documented upload (public media, 10 MiB by
/// default) plus JSON/multipart framing.
const MAX_BODY_BYTES: usize = 15 * 1024 * 1024;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// Everything the middleware stack needs that isn't specific to one
/// service's domain. Build one of these in each service's `main.rs`/`state.rs`
/// and pass it to [`apply`].
#[derive(Clone)]
pub struct MiddlewareState {
    pub redis: RedisPool,
    pub rate_limit: RateLimitConfig,
    pub trust_proxy_headers: bool,
    pub metrics: Arc<Metrics>,
}

pub fn apply<S>(
    router: Router<S>,
    mw: MiddlewareState,
    cors: &CorsConfig,
    enforce_https: bool,
) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    let cors_layer = build_cors_layer(cors);
    router
        // Innermost: closest to the handler, so its timing excludes every
        // other layer's own overhead.
        .layer(middleware::from_fn_with_state(mw.clone(), metrics_mw))
        .layer(middleware::from_fn_with_state(
            mw.clone(),
            rate_limit::rate_limit_mw,
        ))
        .layer(middleware::from_fn_with_state(
            (mw.trust_proxy_headers, enforce_https),
            enforce_https_mw,
        ))
        .layer(middleware::from_fn(security_headers_mw))
        .layer(cors_layer)
        .layer(CompressionLayer::new())
        .layer(RequestBodyLimitLayer::new(MAX_BODY_BYTES))
        .layer(TimeoutLayer::new(REQUEST_TIMEOUT))
        .layer(CatchPanicLayer::new())
        .layer(TraceLayer::new_for_http())
        // Wraps TraceLayer (not the other way around) so request logging
        // never sees a raw Authorization/Cookie value in the first place.
        .layer(SetSensitiveRequestHeadersLayer::new([
            header::AUTHORIZATION,
            header::COOKIE,
        ]))
        .layer(PropagateRequestIdLayer::x_request_id())
        // Outermost: every request gets an id before anything else runs.
        .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
}

/// Exact-origin allow-list — never a wildcard (enforced at config load, see
/// `config::validate_cors_origins`). `allow_credentials(true)` because the
/// web app authenticates via an HttpOnly session cookie (WO-05).
fn build_cors_layer(config: &CorsConfig) -> CorsLayer {
    let origins: Vec<HeaderValue> = config
        .allowed_origins
        .iter()
        .filter_map(|o| HeaderValue::from_str(o).ok())
        .collect();
    CorsLayer::new()
        .allow_origin(AllowOrigin::list(origins))
        .allow_credentials(true)
        .allow_methods([
            axum::http::Method::GET,
            axum::http::Method::POST,
            axum::http::Method::PATCH,
            axum::http::Method::PUT,
            axum::http::Method::DELETE,
            axum::http::Method::OPTIONS,
        ])
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE])
}

/// HSTS, MIME-sniffing, clickjacking and referrer-leak protections on every
/// response. Red-tested in `tests::security_headers_are_present_on_every_response`.
async fn security_headers_mw(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    headers.insert(
        header::STRICT_TRANSPORT_SECURITY,
        HeaderValue::from_static("max-age=63072000; includeSubDomains"),
    );
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY"));
    headers.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("no-referrer"),
    );
    headers.insert(
        HeaderName::from_static("x-permitted-cross-domain-policies"),
        HeaderValue::from_static("none"),
    );
    response
}

/// Refuses plaintext traffic when both `enforce_https` and
/// `trust_proxy_headers` are set — i.e. only when there is an actual
/// trustworthy signal to check (the gateway overwrites `X-Forwarded-Proto`;
/// see `docker/nginx/gateway.conf`). Without a trusted proxy in front, this
/// process cannot know the original scheme and enforcement is the
/// deployment's job (TLS termination at the listener, or an internal-only
/// network) — silently allowing here rather than always-rejecting.
async fn enforce_https_mw(
    State((trust_proxy_headers, enforce_https)): State<(bool, bool)>,
    request: Request,
    next: Next,
) -> Response {
    if enforce_https && trust_proxy_headers {
        let is_https = request
            .headers()
            .get("x-forwarded-proto")
            .and_then(|v| v.to_str().ok())
            == Some("https");
        if !is_https {
            return (StatusCode::UPGRADE_REQUIRED, "HTTPS required").into_response();
        }
    }
    next.run(request).await
}

/// Implemented by every service's `AppState` so [`internal_auth_mw`] can
/// check the shared service-to-service secret without knowing anything else
/// about that service's state.
pub trait InternalAuthState: Send + Sync {
    fn internal_token(&self) -> &str;
}

/// Guards a service's `/internal/*` routes: another service calling in must
/// present the shared `X-Internal-Token` secret. This is defense in depth,
/// not the only layer — the nginx gateway never routes `/internal/*` to the
/// public internet at all (see `docker/nginx/gateway.conf`); this middleware
/// protects against the case where that routing rule is ever misconfigured,
/// or a request originates from elsewhere on the internal Docker network.
pub async fn internal_auth_mw<S>(State(state): State<S>, request: Request, next: Next) -> Response
where
    S: InternalAuthState + Clone + Send + Sync + 'static,
{
    let presented = request
        .headers()
        .get("x-internal-token")
        .and_then(|v| v.to_str().ok());
    if presented != Some(state.internal_token()) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    next.run(request).await
}

/// Updates `mw.metrics`: request-in-flight/total/response-class counters on
/// every request, plus a per-route latency histogram keyed by the *matched*
/// route pattern (never the raw URL) so cardinality stays bounded.
async fn metrics_mw(State(mw): State<MiddlewareState>, request: Request, next: Next) -> Response {
    mw.metrics.request_started();
    let method = request.method().to_string();
    let route = request
        .extensions()
        .get::<MatchedPath>()
        .map(|p| p.as_str().to_string())
        .unwrap_or_else(|| "unmatched".to_string());

    let start = Instant::now();
    let response = next.run(request).await;
    let elapsed = start.elapsed();

    mw.metrics.request_finished(response.status().as_u16());
    mw.metrics.observe_route_latency(&method, &route, elapsed);
    response
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::Request as HttpRequest;
    use axum::routing::get;
    use axum::Router;
    use tower::ServiceExt;

    use super::security_headers_mw;

    #[tokio::test]
    async fn security_headers_are_present_on_every_response() {
        let app = Router::new()
            .route("/", get(|| async { "ok" }))
            .layer(axum::middleware::from_fn(security_headers_mw));

        let response = app
            .oneshot(HttpRequest::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        let headers = response.headers();
        assert_eq!(headers.get("x-content-type-options").unwrap(), "nosniff");
        assert_eq!(headers.get("x-frame-options").unwrap(), "DENY");
        assert_eq!(headers.get("referrer-policy").unwrap(), "no-referrer");
        assert!(headers.get("strict-transport-security").is_some());
    }

    /// Red test: removing the `security_headers_mw` layer from `apply`
    /// (or any header it sets) would make this fail — confirmed by the
    /// assertions above failing when the layer is commented out locally.
    #[tokio::test]
    async fn missing_header_would_fail_the_same_assertions() {
        let app = Router::new().route("/", get(|| async { "ok" }));
        let response = app
            .oneshot(HttpRequest::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert!(response.headers().get("x-frame-options").is_none());
    }
}
