use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct MediaAsset {
    pub id: Uuid,
    pub account_id: Uuid,
    pub kind: String,
    pub object_key: String,
    pub url: String,
    pub content_type: String,
    pub byte_size: i64,
    pub created_at: DateTime<Utc>,
}
