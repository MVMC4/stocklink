//! Router assembly for the identity service.

use axum::routing::get;
use axum::{middleware, Router};
use stocklink_shared::middleware as shared_mw;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::controllers::{account, admin, auth, health, internal};
use crate::openapi::ApiDoc;
use crate::state::AppState;

pub fn build_router(state: AppState) -> Router {
    let internal_routes = Router::new()
        .route("/internal/warehouses/:id", get(internal::get_warehouse))
        .route(
            "/internal/warehouses/:id/owned-by",
            get(internal::assert_owns_warehouse),
        )
        .route("/internal/stores/:id", get(internal::get_store))
        .route(
            "/internal/stores/:id/owned-by",
            get(internal::assert_owns_store),
        )
        .route(
            "/internal/stores/resolve-single",
            get(internal::resolve_single_store),
        )
        .layer(middleware::from_fn_with_state(
            state.clone(),
            shared_mw::internal_auth_mw::<AppState>,
        ));

    let mut router: Router<AppState> = Router::new()
        .route("/health/live", get(health::live))
        .route("/health/ready", get(health::ready))
        .route("/metrics", get(health::metrics))
        .route(
            "/v1/auth/otp/request",
            axum::routing::post(auth::request_otp),
        )
        .route("/v1/auth/otp/verify", axum::routing::post(auth::verify_otp))
        .route("/v1/auth/refresh", axum::routing::post(auth::refresh))
        .route("/v1/auth/logout", axum::routing::post(auth::logout))
        .route(
            "/v1/onboarding/warehouses",
            axum::routing::post(account::register_warehouse).get(account::list_my_warehouses),
        )
        .route(
            "/v1/onboarding/stores",
            axum::routing::post(account::register_store).get(account::list_my_stores),
        )
        .route(
            "/v1/onboarding/carriers",
            axum::routing::post(account::register_carrier).get(account::list_my_carriers),
        )
        .route("/v1/admin/audit-log", get(admin::audit_log))
        .route("/v1/admin/links", get(admin::links))
        .merge(internal_routes);

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
