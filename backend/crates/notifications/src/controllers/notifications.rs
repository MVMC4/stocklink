//! The caller's in-app notification inbox.

use axum::extract::State;
use axum::Json;

use crate::schemas::notifications::NotificationRes;
use crate::state::AppState;
use stocklink_shared::auth::AuthUser;
use stocklink_shared::errors::AppResult;
use stocklink_shared::utils::validation::UuidPath;

pub async fn list_my_notifications(
    State(state): State<AppState>,
    user: AuthUser,
) -> AppResult<Json<Vec<NotificationRes>>> {
    let rows = state
        .notifications
        .list_for_account(user.account_id)
        .await?;
    Ok(Json(rows.into_iter().map(Into::into).collect()))
}

pub async fn mark_notification_read(
    State(state): State<AppState>,
    user: AuthUser,
    UuidPath(id): UuidPath,
) -> AppResult<()> {
    if state.notifications.mark_read(user.account_id, id).await? {
        Ok(())
    } else {
        Err(stocklink_shared::errors::AppError::NotFound("notification"))
    }
}

#[derive(Debug, serde::Deserialize)]
pub struct RegisterDeviceReq {
    pub provider: String,
    pub token: String,
}

pub async fn register_device(
    State(state): State<AppState>,
    user: AuthUser,
    Json(req): Json<RegisterDeviceReq>,
) -> AppResult<()> {
    state
        .devices
        .register(user.account_id, &req.provider, &req.token)
        .await
}
