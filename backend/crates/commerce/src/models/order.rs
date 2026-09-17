use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use uuid::Uuid;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Order {
    pub id: Uuid,
    pub store_id: Uuid,
    pub warehouse_id: Uuid,
    pub status: String,
    pub currency: String,
    pub subtotal: Decimal,
    pub total: Decimal,
    pub idempotency_key: Option<String>,
    pub placed_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct OrderItem {
    pub id: Uuid,
    pub order_id: Uuid,
    pub catalog_item_id: Uuid,
    pub tier: String,
    pub quantity: Decimal,
    pub unit_price: Decimal,
    pub line_total: Decimal,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct OrderEvent {
    pub id: Uuid,
    pub order_id: Uuid,
    pub kind: String,
    pub detail: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct BulkOrder {
    pub id: Uuid,
    pub warehouse_id: Uuid,
    pub catalog_item_id: Uuid,
    pub tier: String,
    pub status: String,
    pub delivery_window_start: Option<DateTime<Utc>>,
    pub delivery_window_end: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct BulkOrderAllocation {
    pub id: Uuid,
    pub bulk_order_id: Uuid,
    pub order_id: Uuid,
    pub store_id: Uuid,
    pub quantity: Decimal,
    pub created_at: DateTime<Utc>,
}
