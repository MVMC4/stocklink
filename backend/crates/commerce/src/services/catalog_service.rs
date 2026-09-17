//! Warehouse catalogue and store cart.

use rust_decimal::Decimal;
use uuid::Uuid;

use crate::models::catalog::{CartItem, CatalogItem};
use crate::repositories::CatalogRepository;
use stocklink_shared::errors::{AppError, AppResult};

#[derive(Clone)]
pub struct CatalogService {
    repo: CatalogRepository,
}

fn valid_tier(tier: &str) -> AppResult<()> {
    match tier {
        "unit" | "case" | "pallet" => Ok(()),
        _ => Err(AppError::Validation {
            field: "tier".into(),
            message: "must be `unit`, `case` or `pallet`".into(),
        }),
    }
}

impl CatalogService {
    pub fn new(repo: CatalogRepository) -> Self {
        Self { repo }
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn publish_item(
        &self,
        warehouse_id: Uuid,
        sku: &str,
        name: &str,
        description: Option<&str>,
        currency: &str,
        unit_label: &str,
        unit_price: Decimal,
        case_size: Option<i32>,
        case_price: Option<Decimal>,
        pallet_size: Option<i32>,
        pallet_price: Option<Decimal>,
        stock_qty_units: Decimal,
    ) -> AppResult<CatalogItem> {
        if unit_price < Decimal::ZERO {
            return Err(AppError::Validation {
                field: "unit_price".into(),
                message: "must not be negative".into(),
            });
        }
        self.repo
            .create_item(
                warehouse_id,
                sku,
                name,
                description,
                currency,
                unit_label,
                unit_price,
                case_size,
                case_price,
                pallet_size,
                pallet_price,
                stock_qty_units,
            )
            .await
    }

    pub async fn browse(&self, warehouse_id: Option<Uuid>) -> AppResult<Vec<CatalogItem>> {
        self.repo.list_active_items(warehouse_id).await
    }

    pub async fn list_for_warehouse(&self, warehouse_id: Uuid) -> AppResult<Vec<CatalogItem>> {
        self.repo.list_items_for_warehouse(warehouse_id).await
    }

    pub async fn get(&self, id: Uuid) -> AppResult<CatalogItem> {
        self.repo
            .find_item(id)
            .await?
            .ok_or(AppError::NotFound("catalog item"))
    }

    pub async fn add_to_cart(
        &self,
        store_id: Uuid,
        catalog_item_id: Uuid,
        tier: &str,
        quantity: Decimal,
    ) -> AppResult<CartItem> {
        valid_tier(tier)?;
        if quantity <= Decimal::ZERO {
            return Err(AppError::Validation {
                field: "quantity".into(),
                message: "must be greater than zero".into(),
            });
        }
        let item = self.get(catalog_item_id).await?;
        if item.price_for_tier(tier).is_none() {
            return Err(AppError::Validation {
                field: "tier".into(),
                message: format!("`{tier}` is not available for this item"),
            });
        }
        self.repo
            .upsert_cart_item(store_id, catalog_item_id, tier, quantity)
            .await
    }

    pub async fn cart(&self, store_id: Uuid) -> AppResult<Vec<CartItem>> {
        self.repo.list_cart_items(store_id).await
    }

    pub async fn remove_from_cart(&self, store_id: Uuid, cart_item_id: Uuid) -> AppResult<()> {
        if self.repo.remove_cart_item(store_id, cart_item_id).await? {
            Ok(())
        } else {
            Err(AppError::NotFound("cart item"))
        }
    }
}
