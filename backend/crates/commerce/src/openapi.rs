//! OpenAPI document assembly: every `#[utoipa::path]`-annotated handler and
//! the schema it references, listed once here rather than discovered by
//! macro magic — the OpenAPI coverage guard (WO-03) diffs this list against
//! the mounted routes, so an entry only mounted here and not in `routes.rs`
//! (or vice versa) fails CI instead of silently drifting.

use utoipa::OpenApi;

use crate::controllers::{catalog, orders};
use crate::schemas::catalog::{AddToCartReq, CartItemRes, CatalogItemRes, PublishCatalogItemReq};
use crate::schemas::common::{DependencyStatus, ErrorBody, ErrorRes, ReadyRes};
use crate::schemas::orders::{AdvanceOrderStatusReq, CheckoutReq, OrderRes};

#[derive(OpenApi)]
#[openapi(
    paths(
        catalog::publish_item,
        catalog::browse_catalog,
        catalog::add_to_cart,
        orders::checkout,
        orders::advance_order_status,
    ),
    components(schemas(
        PublishCatalogItemReq,
        CatalogItemRes,
        AddToCartReq,
        CartItemRes,
        CheckoutReq,
        OrderRes,
        AdvanceOrderStatusReq,
        DependencyStatus,
        ReadyRes,
        ErrorBody,
        ErrorRes,
    )),
    tags(
        (name = "catalog", description = "Warehouse catalogue and cart"),
        (name = "orders", description = "Checkout, order status and settlement"),
    ),
)]
pub struct ApiDoc;
