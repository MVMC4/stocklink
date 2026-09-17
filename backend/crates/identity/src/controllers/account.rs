//! Onboarding: register the caller's account as a warehouse, store or
//! carrier (granting the matching role), and list the caller's own profiles.
//!
//! Registering grants a role the caller's *current* session predates — it
//! was issued at sign-in, before this role existed — so every `register_*`
//! handler below reissues the session (same cookies `auth::verify_otp`
//! sets) carrying the new role, instead of leaving the caller stuck with a
//! token that can't pass that role's guard until it happens to refresh.

use axum::extract::{ConnectInfo, State};
use axum::http::HeaderMap;
use axum::Json;
use axum_extra::extract::CookieJar;
use std::net::SocketAddr;

use crate::controllers::auth::{refresh_cookie, session_cookie};
use crate::schemas::account::{
    CarrierRes, RegisterCarrierReq, RegisterStoreReq, RegisterWarehouseReq, StoreRes, WarehouseRes,
};
use crate::state::AppState;
use stocklink_shared::auth::AuthUser;
use stocklink_shared::errors::AppResult;
use stocklink_shared::utils::validation::ValidatedJson;

async fn reissue_session(
    state: &AppState,
    jar: CookieJar,
    account_id: uuid::Uuid,
    headers: &HeaderMap,
    peer: SocketAddr,
) -> AppResult<CookieJar> {
    let user_agent = headers.get("user-agent").and_then(|v| v.to_str().ok());
    let pair = state
        .auth
        .issue_tokens(account_id, user_agent, Some(&peer.ip().to_string()))
        .await?;
    Ok(jar
        .add(session_cookie(&pair.access_token, state))
        .add(refresh_cookie(&pair.refresh_token, state)))
}

#[utoipa::path(
    post,
    path = "/v1/onboarding/warehouses",
    tag = "onboarding",
    request_body = RegisterWarehouseReq,
    responses((status = 201, body = WarehouseRes)),
)]
pub async fn register_warehouse(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    jar: CookieJar,
    headers: HeaderMap,
    user: AuthUser,
    ValidatedJson(req): ValidatedJson<RegisterWarehouseReq>,
) -> AppResult<(CookieJar, Json<WarehouseRes>)> {
    let w = state
        .onboarding
        .register_warehouse(
            user.account_id,
            &req.name,
            &req.region,
            req.address.as_deref(),
        )
        .await?;
    let jar = reissue_session(&state, jar, user.account_id, &headers, peer).await?;
    Ok((
        jar,
        Json(WarehouseRes {
            id: w.id,
            name: w.name,
            region: w.region,
            address: w.address,
        }),
    ))
}

pub async fn list_my_warehouses(
    State(state): State<AppState>,
    user: AuthUser,
) -> AppResult<Json<Vec<WarehouseRes>>> {
    let rows = state
        .onboarding
        .list_warehouses_for_account(user.account_id)
        .await?;
    Ok(Json(
        rows.into_iter()
            .map(|w| WarehouseRes {
                id: w.id,
                name: w.name,
                region: w.region,
                address: w.address,
            })
            .collect(),
    ))
}

#[utoipa::path(
    post,
    path = "/v1/onboarding/stores",
    tag = "onboarding",
    request_body = RegisterStoreReq,
    responses((status = 201, body = StoreRes)),
)]
pub async fn register_store(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    jar: CookieJar,
    headers: HeaderMap,
    user: AuthUser,
    ValidatedJson(req): ValidatedJson<RegisterStoreReq>,
) -> AppResult<(CookieJar, Json<StoreRes>)> {
    let s = state
        .onboarding
        .register_store(
            user.account_id,
            &req.name,
            &req.region,
            req.address.as_deref(),
        )
        .await?;
    let jar = reissue_session(&state, jar, user.account_id, &headers, peer).await?;
    Ok((
        jar,
        Json(StoreRes {
            id: s.id,
            name: s.name,
            region: s.region,
            address: s.address,
        }),
    ))
}

pub async fn list_my_stores(
    State(state): State<AppState>,
    user: AuthUser,
) -> AppResult<Json<Vec<StoreRes>>> {
    let rows = state
        .onboarding
        .list_stores_for_account(user.account_id)
        .await?;
    Ok(Json(
        rows.into_iter()
            .map(|s| StoreRes {
                id: s.id,
                name: s.name,
                region: s.region,
                address: s.address,
            })
            .collect(),
    ))
}

#[utoipa::path(
    post,
    path = "/v1/onboarding/carriers",
    tag = "onboarding",
    request_body = RegisterCarrierReq,
    responses((status = 201, body = CarrierRes)),
)]
pub async fn register_carrier(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    jar: CookieJar,
    headers: HeaderMap,
    user: AuthUser,
    ValidatedJson(req): ValidatedJson<RegisterCarrierReq>,
) -> AppResult<(CookieJar, Json<CarrierRes>)> {
    let c = state
        .onboarding
        .register_carrier(user.account_id, &req.name)
        .await?;
    let jar = reissue_session(&state, jar, user.account_id, &headers, peer).await?;
    Ok((
        jar,
        Json(CarrierRes {
            id: c.id,
            name: c.name,
        }),
    ))
}

pub async fn list_my_carriers(
    State(state): State<AppState>,
    user: AuthUser,
) -> AppResult<Json<Vec<CarrierRes>>> {
    let rows = state
        .onboarding
        .list_carriers_for_account(user.account_id)
        .await?;
    Ok(Json(
        rows.into_iter()
            .map(|c| CarrierRes {
                id: c.id,
                name: c.name,
            })
            .collect(),
    ))
}
