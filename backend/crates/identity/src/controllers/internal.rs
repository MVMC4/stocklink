//! Service-to-service endpoints (never reachable through the public
//! gateway — see `docker/nginx/gateway.conf` — and guarded again by
//! `stocklink_shared::middleware::internal_auth_mw`). Commerce calls these
//! to resolve a warehouse/store id it only holds as an opaque UUID.

use axum::extract::State;
use axum::Json;
use serde::Serialize;
use stocklink_shared::errors::AppResult;
use uuid::Uuid;

use crate::state::AppState;
use stocklink_shared::utils::validation::UuidPath;

#[derive(Debug, Serialize)]
pub struct WarehouseInternalRes {
    pub id: Uuid,
    pub account_id: Uuid,
    pub name: String,
    pub region: String,
    pub address: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct StoreInternalRes {
    pub id: Uuid,
    pub account_id: Uuid,
    pub name: String,
    pub region: String,
}

pub async fn get_warehouse(
    State(state): State<AppState>,
    UuidPath(id): UuidPath,
) -> AppResult<Json<WarehouseInternalRes>> {
    let w = state.onboarding.get_warehouse(id).await?;
    Ok(Json(WarehouseInternalRes {
        id: w.id,
        account_id: w.account_id,
        name: w.name,
        region: w.region,
        address: w.address,
    }))
}

pub async fn get_store(
    State(state): State<AppState>,
    UuidPath(id): UuidPath,
) -> AppResult<Json<StoreInternalRes>> {
    let s = state.onboarding.get_store(id).await?;
    Ok(Json(StoreInternalRes {
        id: s.id,
        account_id: s.account_id,
        name: s.name,
        region: s.region,
    }))
}

/// `store_id`/`warehouse_id` ownership check, used by Commerce before
/// mutating an order on behalf of the caller's account.
pub async fn assert_owns_warehouse(
    State(state): State<AppState>,
    axum::extract::Query(q): axum::extract::Query<OwnershipQuery>,
    UuidPath(id): UuidPath,
) -> AppResult<Json<bool>> {
    Ok(Json(
        state
            .onboarding
            .assert_owns_warehouse(q.account_id, id)
            .await
            .is_ok(),
    ))
}

pub async fn assert_owns_store(
    State(state): State<AppState>,
    axum::extract::Query(q): axum::extract::Query<OwnershipQuery>,
    UuidPath(id): UuidPath,
) -> AppResult<Json<bool>> {
    Ok(Json(
        state
            .onboarding
            .assert_owns_store(q.account_id, id)
            .await
            .is_ok(),
    ))
}

#[derive(Debug, serde::Deserialize)]
pub struct OwnershipQuery {
    pub account_id: Uuid,
}

/// The caller's single store (see `OnboardingService::resolve_single_store`).
pub async fn resolve_single_store(
    State(state): State<AppState>,
    axum::extract::Query(q): axum::extract::Query<OwnershipQuery>,
) -> AppResult<Json<StoreInternalRes>> {
    let s = state.onboarding.resolve_single_store(q.account_id).await?;
    Ok(Json(StoreInternalRes {
        id: s.id,
        account_id: s.account_id,
        name: s.name,
        region: s.region,
    }))
}
