//! Router assembly for the media service.

use axum::routing::{get, post};
use axum::Router;
use stocklink_shared::middleware as shared_mw;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::controllers::{health, media};
use crate::openapi::ApiDoc;
use crate::state::AppState;

pub fn build_router(state: AppState) -> Router {
    let mut router: Router<AppState> = Router::new()
        .route("/health/live", get(health::live))
        .route("/health/ready", get(health::ready))
        .route("/metrics", get(health::metrics))
        .route("/v1/media/presign", post(media::presign_upload));

    if !state.config.env.is_production_like() {
        router =
            router.merge(SwaggerUi::new("/docs/swagger").url("/openapi.json", ApiDoc::openapi()));
    }

    let mw = shared_mw::MiddlewareState {
        redis: state.redis.clone(),
        rate_limit: state.config.rate_limit,
        trust_proxy_headers: state.config.trust_proxy_headers,
        metrics: state.metrics.clone(),
    };
    let router = shared_mw::apply(router, mw, &state.config.cors, state.config.enforce_https);
    router.with_state(state)
}
