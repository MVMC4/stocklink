//! Presigned media uploads (catalogue photos, proof of delivery).

use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
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

/// Local dev backend only: `presign_upload` hands the caller this route's
/// own URL as the "presigned" upload target (there's no real presigning
/// concept without S3), so the caller PUTs the file bytes here directly.
/// Auth-gated (unlike `serve_local`) since it's a write.
#[utoipa::path(
    put,
    path = "/v1/media/local/{key}",
    tag = "media",
    responses((status = 204)),
)]
pub async fn upload_local(
    State(state): State<AppState>,
    user: AuthUser,
    Path(key): Path<String>,
    body: Bytes,
) -> AppResult<StatusCode> {
    state.media.store_local(user.account_id, &key, &body).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Local dev backend only: serves back what `upload_local` wrote. No auth —
/// mirrors the S3 backend, where these objects are publicly readable (see
/// `public_media.rs`'s module doc).
#[utoipa::path(
    get,
    path = "/v1/media/local/{key}",
    tag = "media",
    responses((status = 200)),
)]
pub async fn serve_local(
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> AppResult<Response> {
    let (bytes, content_type) = state.media.load_local(&key).await?;
    Ok((StatusCode::OK, [(header::CONTENT_TYPE, content_type)], bytes).into_response())
}
