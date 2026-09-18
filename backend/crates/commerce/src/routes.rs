//! Router assembly for the commerce service.

use axum::routing::{delete, get, patch, post, put};
use axum::Router;
use stocklink_shared::middleware as shared_mw;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::controllers::{catalog, health, orders};
use crate::openapi::ApiDoc;
use crate::state::AppState;

pub fn build_router(state: AppState) -> Router {
    let mut router: Router<AppState> = Router::new()
        .route("/health/live", get(health::live))
        .route("/health/ready", get(health::ready))
        .route("/metrics", get(health::metrics))
        .route("/v1/catalog", get(catalog::browse_catalog))
        .route(
            "/v1/warehouses/:warehouse_id/catalog",
            post(catalog::publish_item).get(catalog::list_warehouse_catalog),
        )
        .route(
            "/v1/warehouses/:warehouse_id/catalog/:item_id/images",
            post(catalog::attach_image),
        )
        .route(
            "/v1/warehouses/:warehouse_id/catalog/:item_id/images/order",
            put(catalog::reorder_images),
        )
        .route(
            "/v1/warehouses/:warehouse_id/catalog/:item_id/images/:image_id",
            delete(catalog::remove_image),
        )
        .route(
            "/v1/warehouses/:warehouse_id/catalog/:item_id/images/:image_id/thumbnail",
            put(catalog::set_thumbnail),
        )
        .route(
            "/v1/warehouses/:warehouse_id/orders",
            get(orders::list_warehouse_orders),
        )
        .route(
            "/v1/warehouses/:warehouse_id/orders/:order_id/status",
            patch(orders::advance_order_status),
        )
        .route(
            "/v1/cart/items",
            post(catalog::add_to_cart).get(catalog::get_cart),
        )
        .route(
            "/v1/cart/items/:cart_item_id",
            axum::routing::delete(catalog::remove_from_cart),
        )
        .route("/v1/orders/checkout", post(orders::checkout))
        .route("/v1/orders", get(orders::list_my_orders))
        .route("/v1/orders/:order_id", get(orders::get_my_order));

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
