//! Checkout and order lifecycle.

use axum::extract::State;
use axum::Json;

use crate::schemas::orders::{AdvanceOrderStatusReq, CheckoutReq, OrderRes};
use crate::state::AppState;
use stocklink_shared::auth::{AuthUser, WarehouseUser};
use stocklink_shared::errors::AppResult;
use stocklink_shared::utils::validation::{UuidPath, ValidatedJson};

#[utoipa::path(
    post,
    path = "/v1/orders/checkout",
    tag = "orders",
    request_body = CheckoutReq,
    responses((status = 201, body = [OrderRes])),
)]
pub async fn checkout(
    State(state): State<AppState>,
    user: AuthUser,
    ValidatedJson(req): ValidatedJson<CheckoutReq>,
) -> AppResult<Json<Vec<OrderRes>>> {
    let store = state.identity.resolve_single_store(user.account_id).await?;
    let orders = state
        .orders
        .checkout(store.id, req.idempotency_key.as_deref())
        .await?;
    for order in &orders {
        if let Ok(warehouse) = state.identity.get_warehouse(order.warehouse_id).await {
            let _ = state
                .notifications
                .notify_order_placed(warehouse.account_id, order.id)
                .await;
        }
    }
    Ok(Json(orders.into_iter().map(Into::into).collect()))
}

pub async fn list_my_orders(
    State(state): State<AppState>,
    user: AuthUser,
) -> AppResult<Json<Vec<OrderRes>>> {
    let store = state.identity.resolve_single_store(user.account_id).await?;
    let orders = state.orders.list_for_store(store.id).await?;
    Ok(Json(orders.into_iter().map(Into::into).collect()))
}

pub async fn get_my_order(
    State(state): State<AppState>,
    user: AuthUser,
    UuidPath(order_id): UuidPath,
) -> AppResult<Json<OrderRes>> {
    let store = state.identity.resolve_single_store(user.account_id).await?;
    let order = state.orders.get_for_store(store.id, order_id).await?;
    Ok(Json(order.into()))
}

pub async fn list_warehouse_orders(
    State(state): State<AppState>,
    user: WarehouseUser,
    UuidPath(warehouse_id): UuidPath,
) -> AppResult<Json<Vec<OrderRes>>> {
    state
        .identity
        .assert_owns_warehouse(user.0.account_id, warehouse_id)
        .await?;
    let orders = state.orders.list_for_warehouse(warehouse_id).await?;
    Ok(Json(orders.into_iter().map(Into::into).collect()))
}

#[utoipa::path(
    patch,
    path = "/v1/warehouses/{warehouse_id}/orders/{order_id}/status",
    tag = "orders",
    request_body = AdvanceOrderStatusReq,
    responses((status = 200, body = OrderRes)),
)]
pub async fn advance_order_status(
    State(state): State<AppState>,
    user: WarehouseUser,
    stocklink_shared::utils::validation::UuidPath2(warehouse_id, order_id): stocklink_shared::utils::validation::UuidPath2,
    ValidatedJson(req): ValidatedJson<AdvanceOrderStatusReq>,
) -> AppResult<Json<OrderRes>> {
    state
        .identity
        .assert_owns_warehouse(user.0.account_id, warehouse_id)
        .await?;
    let order = state
        .orders
        .advance_status(warehouse_id, order_id, &req.status)
        .await?;
    if let Ok(store) = state.identity.get_store(order.store_id).await {
        let _ = state
            .notifications
            .notify_order_status_changed(store.account_id, order.id, &order.status)
            .await;
    }
    Ok(Json(order.into()))
}
