use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CheckoutReq {
    /// Client-generated idempotency key — retrying the same checkout with
    /// the same key never creates duplicate orders.
    pub idempotency_key: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct OrderRes {
    pub id: Uuid,
    pub store_id: Uuid,
    pub warehouse_id: Uuid,
    pub status: String,
    pub currency: String,
    pub subtotal: Decimal,
    pub total: Decimal,
    pub placed_at: DateTime<Utc>,
}

impl From<crate::models::order::Order> for OrderRes {
    fn from(o: crate::models::order::Order) -> Self {
        Self {
            id: o.id,
            store_id: o.store_id,
            warehouse_id: o.warehouse_id,
            status: o.status,
            currency: o.currency,
            subtotal: o.subtotal,
            total: o.total,
            placed_at: o.placed_at,
        }
    }
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct AdvanceOrderStatusReq {
    #[validate(length(min = 1))]
    pub status: String,
}
