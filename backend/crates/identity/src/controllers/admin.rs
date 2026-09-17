//! Admin console: audit log and deep-link tiles. Every route here requires
//! the `admin` role — there's no separate `AdminUser` extractor (unlike
//! `WarehouseUser`) since this is the only controller that needs it; a
//! second admin-scoped controller should factor this check out the same
//! way `WarehouseUser` did.

use axum::extract::{Query, State};
use axum::Json;
use serde::Deserialize;
use stocklink_shared::auth::claims::ROLE_ADMIN;
use stocklink_shared::auth::AuthUser;
use stocklink_shared::errors::{AppError, AppResult};

use crate::schemas::admin::{AdminAuditEntryRes, AdminLinksRes};
use crate::state::AppState;

fn require_admin(user: &AuthUser) -> AppResult<()> {
    if user.has_role(ROLE_ADMIN) {
        Ok(())
    } else {
        Err(AppError::Unauthorized)
    }
}

#[derive(Debug, Deserialize)]
pub struct AuditLogQuery {
    limit: Option<i64>,
}

#[utoipa::path(
    get,
    path = "/v1/admin/audit-log",
    tag = "admin",
    params(("limit" = Option<i64>, Query, description = "Max entries, default 100")),
    responses((status = 200, body = [AdminAuditEntryRes])),
)]
pub async fn audit_log(
    State(state): State<AppState>,
    user: AuthUser,
    Query(query): Query<AuditLogQuery>,
) -> AppResult<Json<Vec<AdminAuditEntryRes>>> {
    require_admin(&user)?;
    let entries = state.admin.audit_log(query.limit.unwrap_or(100)).await?;
    Ok(Json(entries.into_iter().map(Into::into).collect()))
}

#[utoipa::path(
    get,
    path = "/v1/admin/links",
    tag = "admin",
    responses((status = 200, body = AdminLinksRes)),
)]
pub async fn links(
    State(state): State<AppState>,
    user: AuthUser,
) -> AppResult<Json<AdminLinksRes>> {
    require_admin(&user)?;
    Ok(Json(AdminLinksRes::from(state.admin.links())))
}
