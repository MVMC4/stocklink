//! Service-to-service endpoint: other services (commerce today) POST here
//! to raise a notification for an account. Never reachable through the
//! public gateway; guarded by `stocklink_shared::middleware::internal_auth_mw`.

use axum::extract::State;
use axum::Json;
use serde::Deserialize;
use stocklink_shared::errors::AppResult;
use uuid::Uuid;

use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct RaiseNotificationReq {
    pub account_id: Uuid,
    pub kind: String,
    pub title: String,
    pub body: String,
    #[serde(default)]
    pub data: serde_json::Value,
}

pub async fn raise(
    State(state): State<AppState>,
    Json(req): Json<RaiseNotificationReq>,
) -> AppResult<()> {
    state
        .notifications
        .dispatch(req.account_id, &req.kind, &req.title, &req.body, req.data)
        .await
}
