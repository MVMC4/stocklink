use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct PublishCatalogItemReq {
    #[validate(length(min = 1, max = 64))]
    pub sku: String,
    #[validate(length(min = 1, max = 200))]
    pub name: String,
    pub description: Option<String>,
    #[validate(length(equal = 3))]
    pub currency: String,
    #[validate(length(min = 1, max = 32))]
    pub unit_label: String,
    /// Server-validated for non-negativity in `CatalogService::publish_item`
    /// (rust_decimal doesn't implement `validator`'s numeric-range trait).
    pub unit_price: Decimal,
    pub case_size: Option<i32>,
    pub case_price: Option<Decimal>,
    pub pallet_size: Option<i32>,
    pub pallet_price: Option<Decimal>,
    pub stock_qty_units: Decimal,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CatalogItemRes {
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
}

impl From<crate::models::catalog::CatalogItem> for CatalogItemRes {
    fn from(item: crate::models::catalog::CatalogItem) -> Self {
        Self {
            id: item.id,
            warehouse_id: item.warehouse_id,
            sku: item.sku,
            name: item.name,
            description: item.description,
            currency: item.currency,
            unit_label: item.unit_label,
            unit_price: item.unit_price,
            case_size: item.case_size,
            case_price: item.case_price,
            pallet_size: item.pallet_size,
            pallet_price: item.pallet_price,
            stock_qty_units: item.stock_qty_units,
            active: item.active,
        }
    }
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct AddToCartReq {
    pub catalog_item_id: Uuid,
    #[validate(length(min = 1))]
    pub tier: String,
    /// Server-validated as `> 0` in `CatalogService::add_to_cart`.
    pub quantity: Decimal,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CartItemRes {
    pub id: Uuid,
    pub catalog_item_id: Uuid,
    pub tier: String,
    pub quantity: Decimal,
}

impl From<crate::models::catalog::CartItem> for CartItemRes {
    fn from(item: crate::models::catalog::CartItem) -> Self {
        Self {
            id: item.id,
            catalog_item_id: item.catalog_item_id,
            tier: item.tier,
            quantity: item.quantity,
        }
    }
}
