//! Warehouse catalogue (publish/browse) and store cart.

use std::collections::{HashMap, HashSet};

use axum::extract::{Query, State};
use axum::Json;
use serde::Deserialize;
use uuid::Uuid;

use crate::clients::identity_client::WarehouseInfo;
use crate::models::catalog::{CatalogItem, CatalogItemImage};
use crate::schemas::catalog::{
    AddToCartReq, AttachImageReq, CartItemRes, CatalogItemRes, PublishCatalogItemReq,
    ReorderImagesReq,
};
use crate::state::AppState;
use stocklink_shared::auth::{AuthUser, WarehouseUser};
use stocklink_shared::errors::AppResult;
use stocklink_shared::utils::validation::{UuidPath, UuidPath2, UuidPath3, ValidatedJson};

fn with_warehouse(res: CatalogItemRes, warehouse: &WarehouseInfo) -> CatalogItemRes {
    res.with_warehouse(&warehouse.name, &warehouse.region, warehouse.address.as_deref())
}

/// Zips items with their (already-fetched, batched) images and warehouse
/// info — one query/lookup per distinct warehouse on the page, not one per
/// row (see `browse_catalog`, the one caller that can span several
/// warehouses at once).
fn assemble(
    items: Vec<CatalogItem>,
    images: Vec<CatalogItemImage>,
    warehouses: &HashMap<Uuid, WarehouseInfo>,
) -> Vec<CatalogItemRes> {
    let mut grouped: HashMap<Uuid, Vec<CatalogItemImage>> = HashMap::new();
    for image in images {
        grouped.entry(image.catalog_item_id).or_default().push(image);
    }
    items
        .into_iter()
        .map(|item| {
            let images = grouped.remove(&item.id).unwrap_or_default();
            let warehouse_id = item.warehouse_id;
            let res = CatalogItemRes::from_item_and_images(item, images);
            match warehouses.get(&warehouse_id) {
                Some(w) => with_warehouse(res, w),
                None => res,
            }
        })
        .collect()
}

async fn respond_with_item(
    state: &AppState,
    item_id: Uuid,
    warehouse: &WarehouseInfo,
) -> AppResult<Json<CatalogItemRes>> {
    let item = state.catalog.get(item_id).await?;
    let images = state.catalog.images_for_item(item_id).await?;
    let res = CatalogItemRes::from_item_and_images(item, images);
    Ok(Json(with_warehouse(res, warehouse)))
}

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
    let warehouse = state
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
    let res = with_warehouse(CatalogItemRes::from_item_and_images(item, Vec::new()), &warehouse);
    Ok(Json(res))
}

pub async fn list_warehouse_catalog(
    State(state): State<AppState>,
    user: WarehouseUser,
    UuidPath(warehouse_id): UuidPath,
) -> AppResult<Json<Vec<CatalogItemRes>>> {
    let warehouse = state
        .identity
        .assert_owns_warehouse(user.0.account_id, warehouse_id)
        .await?;
    let items = state.catalog.list_for_warehouse(warehouse_id).await?;
    let ids: Vec<Uuid> = items.iter().map(|i| i.id).collect();
    let images = state.catalog.images_for_items(&ids).await?;
    let warehouses = HashMap::from([(warehouse_id, warehouse)]);
    Ok(Json(assemble(items, images, &warehouses)))
}

#[utoipa::path(
    post,
    path = "/v1/warehouses/{warehouse_id}/catalog/{item_id}/images",
    tag = "catalog",
    request_body = AttachImageReq,
    responses((status = 201, body = CatalogItemRes)),
)]
pub async fn attach_image(
    State(state): State<AppState>,
    user: WarehouseUser,
    UuidPath2(warehouse_id, item_id): UuidPath2,
    ValidatedJson(req): ValidatedJson<AttachImageReq>,
) -> AppResult<Json<CatalogItemRes>> {
    let warehouse = state
        .identity
        .assert_owns_warehouse(user.0.account_id, warehouse_id)
        .await?;
    state
        .catalog
        .attach_image(warehouse_id, item_id, req.media_asset_id, &req.url)
        .await?;
    respond_with_item(&state, item_id, &warehouse).await
}

#[utoipa::path(
    delete,
    path = "/v1/warehouses/{warehouse_id}/catalog/{item_id}/images/{image_id}",
    tag = "catalog",
    responses((status = 200, body = CatalogItemRes)),
)]
pub async fn remove_image(
    State(state): State<AppState>,
    user: WarehouseUser,
    UuidPath3(warehouse_id, item_id, image_id): UuidPath3,
) -> AppResult<Json<CatalogItemRes>> {
    let warehouse = state
        .identity
        .assert_owns_warehouse(user.0.account_id, warehouse_id)
        .await?;
    state
        .catalog
        .remove_image(warehouse_id, item_id, image_id)
        .await?;
    respond_with_item(&state, item_id, &warehouse).await
}

#[utoipa::path(
    put,
    path = "/v1/warehouses/{warehouse_id}/catalog/{item_id}/images/order",
    tag = "catalog",
    request_body = ReorderImagesReq,
    responses((status = 200, body = CatalogItemRes)),
)]
pub async fn reorder_images(
    State(state): State<AppState>,
    user: WarehouseUser,
    UuidPath2(warehouse_id, item_id): UuidPath2,
    ValidatedJson(req): ValidatedJson<ReorderImagesReq>,
) -> AppResult<Json<CatalogItemRes>> {
    let warehouse = state
        .identity
        .assert_owns_warehouse(user.0.account_id, warehouse_id)
        .await?;
    state
        .catalog
        .reorder_images(warehouse_id, item_id, &req.image_ids)
        .await?;
    respond_with_item(&state, item_id, &warehouse).await
}

#[utoipa::path(
    put,
    path = "/v1/warehouses/{warehouse_id}/catalog/{item_id}/images/{image_id}/thumbnail",
    tag = "catalog",
    responses((status = 200, body = CatalogItemRes)),
)]
pub async fn set_thumbnail(
    State(state): State<AppState>,
    user: WarehouseUser,
    UuidPath3(warehouse_id, item_id, image_id): UuidPath3,
) -> AppResult<Json<CatalogItemRes>> {
    let warehouse = state
        .identity
        .assert_owns_warehouse(user.0.account_id, warehouse_id)
        .await?;
    state
        .catalog
        .set_thumbnail(warehouse_id, item_id, image_id)
        .await?;
    respond_with_item(&state, item_id, &warehouse).await
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
    let ids: Vec<Uuid> = items.iter().map(|i| i.id).collect();
    let images = state.catalog.images_for_items(&ids).await?;

    let distinct_warehouse_ids: HashSet<Uuid> = items.iter().map(|i| i.warehouse_id).collect();
    let mut warehouses = HashMap::with_capacity(distinct_warehouse_ids.len());
    for id in distinct_warehouse_ids {
        // Best-effort: a listing whose warehouse lookup fails still shows,
        // just without ship-from details, rather than the whole page 500ing.
        if let Ok(info) = state.identity.get_warehouse(id).await {
            warehouses.insert(id, info);
        }
    }
    Ok(Json(assemble(items, images, &warehouses)))
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
