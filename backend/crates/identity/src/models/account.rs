use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Account {
    pub id: Uuid,
    pub status: String,
    pub display_name: String,
    pub preferred_lang: String,
    pub primary_contact_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AccountContact {
    pub id: Uuid,
    pub account_id: Uuid,
    pub channel: String,
    pub identifier: String,
    pub verified_at: Option<DateTime<Utc>>,
    pub is_primary: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AccountRole {
    pub account_id: Uuid,
    pub role: String,
    pub created_at: DateTime<Utc>,
}
