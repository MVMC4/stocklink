//! Data access for warehouse catalogue items and store cart items.

use rust_decimal::Decimal;
use uuid::Uuid;

use crate::models::catalog::{CartItem, CatalogItem};
use stocklink_shared::database::DbPool;
use stocklink_shared::errors::AppResult;

#[derive(Clone)]
pub struct CatalogRepository {
    db: DbPool,
}

#[allow(clippy::too_many_arguments)]
impl CatalogRepository {
    pub fn new(db: DbPool) -> Self {
        Self { db }
    }

    pub async fn create_item(
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
        let row = sqlx::query_as::<_, CatalogItem>(
            "INSERT INTO catalog_items \
             (warehouse_id, sku, name, description, currency, unit_label, unit_price, \
              case_size, case_price, pallet_size, pallet_price, stock_qty_units) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12) RETURNING *",
        )
        .bind(warehouse_id)
        .bind(sku)
        .bind(name)
        .bind(description)
        .bind(currency)
        .bind(unit_label)
        .bind(unit_price)
        .bind(case_size)
        .bind(case_price)
        .bind(pallet_size)
        .bind(pallet_price)
        .bind(stock_qty_units)
        .fetch_one(self.db.write())
        .await?;
        Ok(row)
    }

    pub async fn find_item(&self, id: Uuid) -> AppResult<Option<CatalogItem>> {
        let row = sqlx::query_as::<_, CatalogItem>("SELECT * FROM catalog_items WHERE id = $1")
            .bind(id)
            .fetch_optional(self.db.read())
            .await?;
        Ok(row)
    }

    /// Active items, optionally scoped to one warehouse — the browsable
    /// catalogue a store sees.
    pub async fn list_active_items(
        &self,
        warehouse_id: Option<Uuid>,
    ) -> AppResult<Vec<CatalogItem>> {
        let rows =
            match warehouse_id {
                Some(id) => sqlx::query_as::<_, CatalogItem>(
                    "SELECT * FROM catalog_items WHERE active AND warehouse_id = $1 ORDER BY name",
                )
                .bind(id)
                .fetch_all(self.db.read())
                .await?,
                None => {
                    sqlx::query_as::<_, CatalogItem>(
                        "SELECT * FROM catalog_items WHERE active ORDER BY name",
                    )
                    .fetch_all(self.db.read())
                    .await?
                }
            };
        Ok(rows)
    }

    pub async fn list_items_for_warehouse(
        &self,
        warehouse_id: Uuid,
    ) -> AppResult<Vec<CatalogItem>> {
        let rows = sqlx::query_as::<_, CatalogItem>(
            "SELECT * FROM catalog_items WHERE warehouse_id = $1 ORDER BY created_at DESC",
        )
        .bind(warehouse_id)
        .fetch_all(self.db.read())
        .await?;
        Ok(rows)
    }

    /// Decrement stock by `qty_units`, but only if enough remains —
    /// atomic and race-safe (never goes negative under concurrent orders).
    /// Returns `false` if there wasn't enough stock.
    pub async fn try_decrement_stock(&self, id: Uuid, qty_units: Decimal) -> AppResult<bool> {
        let result = sqlx::query(
            "UPDATE catalog_items SET stock_qty_units = stock_qty_units - $2, updated_at = now() \
             WHERE id = $1 AND stock_qty_units >= $2",
        )
        .bind(id)
        .bind(qty_units)
        .execute(self.db.write())
        .await?;
        Ok(result.rows_affected() == 1)
    }

    // ── cart ────────────────────────────────────────────────────────────

    pub async fn upsert_cart_item(
        &self,
        store_id: Uuid,
        catalog_item_id: Uuid,
        tier: &str,
        quantity: Decimal,
    ) -> AppResult<CartItem> {
        let row = sqlx::query_as::<_, CartItem>(
            "INSERT INTO cart_items (store_id, catalog_item_id, tier, quantity) VALUES ($1,$2,$3,$4) \
             ON CONFLICT (store_id, catalog_item_id, tier) \
             DO UPDATE SET quantity = EXCLUDED.quantity, updated_at = now() \
             RETURNING *",
        )
        .bind(store_id)
        .bind(catalog_item_id)
        .bind(tier)
        .bind(quantity)
        .fetch_one(self.db.write())
        .await?;
        Ok(row)
    }

    pub async fn list_cart_items(&self, store_id: Uuid) -> AppResult<Vec<CartItem>> {
        let rows = sqlx::query_as::<_, CartItem>(
            "SELECT * FROM cart_items WHERE store_id = $1 ORDER BY created_at",
        )
        .bind(store_id)
        .fetch_all(self.db.read())
        .await?;
        Ok(rows)
    }

    pub async fn remove_cart_item(&self, store_id: Uuid, id: Uuid) -> AppResult<bool> {
        let result = sqlx::query("DELETE FROM cart_items WHERE id = $1 AND store_id = $2")
            .bind(id)
            .bind(store_id)
            .execute(self.db.write())
            .await?;
        Ok(result.rows_affected() == 1)
    }

    pub async fn clear_cart(&self, store_id: Uuid) -> AppResult<()> {
        sqlx::query("DELETE FROM cart_items WHERE store_id = $1")
            .bind(store_id)
            .execute(self.db.write())
            .await?;
        Ok(())
    }
}
