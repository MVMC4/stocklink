//! Warehouse catalogue and store cart.

use rust_decimal::Decimal;
use uuid::Uuid;

use crate::models::catalog::{CartItem, CatalogItem, CatalogItemImage};
use crate::repositories::CatalogRepository;
use stocklink_shared::errors::{AppError, AppResult};

/// Storefront listings need room for a few angles of one product, but not a
/// full gallery — five keeps upload time and review effort small for a
/// warehouse publishing many SKUs.
const MAX_IMAGES_PER_ITEM: usize = 5;

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

    // ── catalogue images ─────────────────────────────────────────────────

    pub async fn images_for_item(&self, item_id: Uuid) -> AppResult<Vec<CatalogItemImage>> {
        self.repo.list_images_for_item(item_id).await
    }

    pub async fn images_for_items(&self, item_ids: &[Uuid]) -> AppResult<Vec<CatalogItemImage>> {
        if item_ids.is_empty() {
            return Ok(Vec::new());
        }
        self.repo.list_images_for_items(item_ids).await
    }

    /// Ownership is checked here (not just at the controller, which already
    /// confirms the caller owns `warehouse_id`) because the item itself must
    /// also belong to that warehouse — otherwise a warehouse owner could
    /// attach, reorder or delete photos on another warehouse's listing by
    /// guessing its id.
    async fn owned_item(&self, warehouse_id: Uuid, item_id: Uuid) -> AppResult<CatalogItem> {
        let item = self.get(item_id).await?;
        if item.warehouse_id != warehouse_id {
            return Err(AppError::NotFound("catalog item"));
        }
        Ok(item)
    }

    pub async fn attach_image(
        &self,
        warehouse_id: Uuid,
        item_id: Uuid,
        media_asset_id: Uuid,
        url: &str,
    ) -> AppResult<CatalogItemImage> {
        self.owned_item(warehouse_id, item_id).await?;
        if !(url.starts_with("http://") || url.starts_with("https://")) {
            return Err(AppError::Validation {
                field: "url".into(),
                message: "must be an http(s) URL".into(),
            });
        }
        let existing = self.repo.list_images_for_item(item_id).await?;
        if existing.len() >= MAX_IMAGES_PER_ITEM {
            return Err(AppError::Conflict(format!(
                "an item can have at most {MAX_IMAGES_PER_ITEM} images"
            )));
        }
        let is_first = existing.is_empty();
        self.repo
            .add_image(item_id, media_asset_id, url, existing.len() as i16, is_first)
            .await
    }

    pub async fn remove_image(
        &self,
        warehouse_id: Uuid,
        item_id: Uuid,
        image_id: Uuid,
    ) -> AppResult<()> {
        self.owned_item(warehouse_id, item_id).await?;
        let image = self
            .repo
            .find_image(image_id)
            .await?
            .filter(|i| i.catalog_item_id == item_id)
            .ok_or(AppError::NotFound("image"))?;

        if !self.repo.delete_image(image_id).await? {
            return Err(AppError::NotFound("image"));
        }
        // Keep a listing with remaining photos from going thumbnail-less.
        if image.is_thumbnail {
            let remaining = self.repo.list_images_for_item(item_id).await?;
            if let Some(next) = remaining.into_iter().min_by_key(|i| i.sort_order) {
                self.repo.set_thumbnail(next.id).await?;
            }
        }
        Ok(())
    }

    pub async fn reorder_images(
        &self,
        warehouse_id: Uuid,
        item_id: Uuid,
        ordered_ids: &[Uuid],
    ) -> AppResult<()> {
        self.owned_item(warehouse_id, item_id).await?;
        let existing = self.repo.list_images_for_item(item_id).await?;

        let mut existing_ids: Vec<Uuid> = existing.iter().map(|i| i.id).collect();
        existing_ids.sort();
        let mut given_ids = ordered_ids.to_vec();
        given_ids.sort();
        if existing_ids != given_ids {
            return Err(AppError::Validation {
                field: "image_ids".into(),
                message: "must list exactly this item's current images, once each".into(),
            });
        }

        for (index, id) in ordered_ids.iter().enumerate() {
            self.repo.set_sort_order(*id, index as i16).await?;
        }
        Ok(())
    }

    pub async fn set_thumbnail(
        &self,
        warehouse_id: Uuid,
        item_id: Uuid,
        image_id: Uuid,
    ) -> AppResult<()> {
        self.owned_item(warehouse_id, item_id).await?;
        self.repo
            .find_image(image_id)
            .await?
            .filter(|i| i.catalog_item_id == item_id)
            .ok_or(AppError::NotFound("image"))?;

        self.repo.clear_thumbnail(item_id).await?;
        self.repo.set_thumbnail(image_id).await?;
        Ok(())
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
