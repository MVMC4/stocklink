use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use uuid::Uuid;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Settlement {
    pub id: Uuid,
    pub order_id: Uuid,
    pub status: String,
    pub total_amount: Decimal,
    pub currency: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct LedgerEntry {
    pub id: Uuid,
    pub journal_id: Uuid,
    pub account_id: Uuid,
    pub direction: String,
    pub amount: Decimal,
    pub currency: String,
    pub memo: Option<String>,
    pub created_at: DateTime<Utc>,
}
