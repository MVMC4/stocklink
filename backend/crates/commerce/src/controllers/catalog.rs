//! Warehouse catalogue (publish/browse) and store cart.

use axum::extract::{Query, State};
use axum::Json;
use serde::Deserialize;
use uuid::Uuid;

use crate::schemas::catalog::{AddToCartReq, CartItemRes, CatalogItemRes, PublishCatalogItemReq};
use crate::state::AppState;
use stocklink_shared::auth::{AuthUser, WarehouseUser};
use stocklink_shared::errors::AppResult;
use stocklink_shared::utils::validation::{UuidPath, ValidatedJson};

#[utoipa::path(
    post,
    path = "/v1/warehouses/{warehouse_id}/catalog",
    tag = "catalog",
    request_body = PublishCatalogItemReq,
    responses((status = 201, body = CatalogItemRes)),
)]
pub async fn publish_item(
    State(state): State<AppState>,
    user: WarehouseUser,
    UuidPath(warehouse_id): UuidPath,
    ValidatedJson(req): ValidatedJson<PublishCatalogItemReq>,
) -> AppResult<Json<CatalogItemRes>> {
    state
        .identity
        .assert_owns_warehouse(user.0.account_id, warehouse_id)
        .await?;
    let item = state
        .catalog
        .publish_item(
            warehouse_id,
            &req.sku,
            &req.name,
            req.description.as_deref(),
            &req.currency,
            &req.unit_label,
            req.unit_price,
            req.case_size,
            req.case_price,
            req.pallet_size,
            req.pallet_price,
            req.stock_qty_units,
        )
        .await?;
    Ok(Json(item.into()))
}

pub async fn list_warehouse_catalog(
    State(state): State<AppState>,
    user: WarehouseUser,
    UuidPath(warehouse_id): UuidPath,
) -> AppResult<Json<Vec<CatalogItemRes>>> {
    state
        .identity
        .assert_owns_warehouse(user.0.account_id, warehouse_id)
        .await?;
    let items = state.catalog.list_for_warehouse(warehouse_id).await?;
    Ok(Json(items.into_iter().map(Into::into).collect()))
}

#[derive(Debug, Deserialize)]
pub struct BrowseQuery {
    pub warehouse_id: Option<Uuid>,
}

#[utoipa::path(
    get,
    path = "/v1/catalog",
    tag = "catalog",
    responses((status = 200, body = [CatalogItemRes])),
)]
pub async fn browse_catalog(
    State(state): State<AppState>,
    Query(q): Query<BrowseQuery>,
) -> AppResult<Json<Vec<CatalogItemRes>>> {
    let items = state.catalog.browse(q.warehouse_id).await?;
    Ok(Json(items.into_iter().map(Into::into).collect()))
}

#[utoipa::path(
    post,
    path = "/v1/cart/items",
    tag = "cart",
    request_body = AddToCartReq,
    responses((status = 200, body = CartItemRes)),
)]
pub async fn add_to_cart(
    State(state): State<AppState>,
    user: AuthUser,
    ValidatedJson(req): ValidatedJson<AddToCartReq>,
) -> AppResult<Json<CartItemRes>> {
    let store = state.identity.resolve_single_store(user.account_id).await?;
    let item = state
        .catalog
        .add_to_cart(store.id, req.catalog_item_id, &req.tier, req.quantity)
        .await?;
    Ok(Json(item.into()))
}

pub async fn get_cart(
    State(state): State<AppState>,
    user: AuthUser,
) -> AppResult<Json<Vec<CartItemRes>>> {
    let store = state.identity.resolve_single_store(user.account_id).await?;
    let items = state.catalog.cart(store.id).await?;
    Ok(Json(items.into_iter().map(Into::into).collect()))
}

pub async fn remove_from_cart(
    State(state): State<AppState>,
    user: AuthUser,
    UuidPath(cart_item_id): UuidPath,
) -> AppResult<()> {
    let store = state.identity.resolve_single_store(user.account_id).await?;
    state.catalog.remove_from_cart(store.id, cart_item_id).await
}
