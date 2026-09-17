use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use uuid::Uuid;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct CatalogItem {
    pub id: Uuid,
    pub warehouse_id: Uuid,
    pub sku: String,
    pub name: String,
    pub description: Option<String>,
    pub currency: String,
    pub unit_label: String,
    pub unit_price: Decimal,
    pub case_size: Option<i32>,
    pub case_price: Option<Decimal>,
    pub pallet_size: Option<i32>,
    pub pallet_price: Option<Decimal>,
    pub stock_qty_units: Decimal,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl CatalogItem {
    /// Server-computed price for one unit of `tier` — the single source of
    /// truth for pricing (rule 7: price is never trusted from the client).
    /// `None` if the tier isn't configured for this item (e.g. no pallet
    /// price set).
    pub fn price_for_tier(&self, tier: &str) -> Option<Decimal> {
        match tier {
            "unit" => Some(self.unit_price),
            "case" => self.case_price,
            "pallet" => self.pallet_price,
            _ => None,
        }
    }

    /// How many base units one `tier` unit represents, for stock accounting.
    pub fn units_per_tier(&self, tier: &str) -> Option<i32> {
        match tier {
            "unit" => Some(1),
            "case" => self.case_size,
            "pallet" => self.pallet_size,
            _ => None,
        }
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct CartItem {
    pub id: Uuid,
    pub store_id: Uuid,
    pub catalog_item_id: Uuid,
    pub tier: String,
    pub quantity: Decimal,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
