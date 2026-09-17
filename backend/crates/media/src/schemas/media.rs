use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct PresignUploadReq {
    #[validate(length(min = 1, max = 64))]
    pub kind: String,
    #[validate(length(min = 1, max = 100))]
    pub content_type: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PresignUploadRes {
    pub media_asset_id: Uuid,
    pub upload_url: String,
    pub public_url: String,
}
