//! Presigned media uploads (catalogue photos, proof of delivery).

use axum::extract::State;
use axum::Json;

use crate::schemas::media::{PresignUploadReq, PresignUploadRes};
use crate::state::AppState;
use stocklink_shared::auth::AuthUser;
use stocklink_shared::errors::AppResult;
use stocklink_shared::utils::validation::ValidatedJson;

#[utoipa::path(
    post,
    path = "/v1/media/presign",
    tag = "media",
    request_body = PresignUploadReq,
    responses((status = 200, body = PresignUploadRes)),
)]
pub async fn presign_upload(
    State(state): State<AppState>,
    user: AuthUser,
    ValidatedJson(req): ValidatedJson<PresignUploadReq>,
) -> AppResult<Json<PresignUploadRes>> {
    let (upload_url, asset) = state
        .media
        .presign_upload(user.account_id, &req.kind, &req.content_type)
        .await?;
    Ok(Json(PresignUploadRes {
        media_asset_id: asset.id,
        upload_url,
        public_url: asset.url,
    }))
}
