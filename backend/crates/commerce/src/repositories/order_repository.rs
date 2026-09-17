//! Data access for orders, bulk consolidation, settlements and the ledger.

use rust_decimal::Decimal;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::models::order::{BulkOrder, BulkOrderAllocation, Order, OrderItem};
use crate::models::settlement::{LedgerEntry, Settlement};
use stocklink_shared::database::DbPool;
use stocklink_shared::errors::AppResult;

pub struct NewOrderItem {
    pub catalog_item_id: Uuid,
    pub tier: String,
    pub quantity: Decimal,
    pub unit_price: Decimal,
    pub line_total: Decimal,
}

#[derive(Clone)]
pub struct OrderRepository {
    db: DbPool,
}

impl OrderRepository {
    pub fn new(db: DbPool) -> Self {
        Self { db }
    }

    pub async fn find_by_idempotency_key(
        &self,
        store_id: Uuid,
        key: &str,
    ) -> AppResult<Option<Order>> {
        let row = sqlx::query_as::<_, Order>(
            "SELECT * FROM orders WHERE store_id = $1 AND idempotency_key = $2",
        )
        .bind(store_id)
        .bind(key)
        .fetch_optional(self.db.read())
        .await?;
        Ok(row)
    }

    /// One order for one warehouse, atomically: decrements stock for every
    /// line item (failing the whole transaction — order and all — if any
    /// item doesn't have enough), inserts the order, its items, and a
    /// `placed` event. `stock_units` is the base-unit quantity to decrement
    /// per catalog item (quantity × tier size), which may differ from the
    /// order line's own `quantity` (that's in tier units, e.g. 3 cases).
    ///
    /// Returns `Ok(None)` — not an error — when stock was insufficient for
    /// any item, so the caller can report exactly which item ran out
    /// (`AppError::Conflict`) without this repository knowing about HTTP
    /// error shapes.
    pub async fn create_order_with_stock_check(
        &self,
        store_id: Uuid,
        warehouse_id: Uuid,
        currency: &str,
        idempotency_key: Option<&str>,
        items: &[NewOrderItem],
        stock_units: &[(Uuid, Decimal)],
    ) -> AppResult<Option<(Order, Vec<OrderItem>)>> {
        let subtotal: Decimal = items.iter().map(|i| i.line_total).sum();
        let mut tx = self.db.write().begin().await?;

        for (catalog_item_id, qty_units) in stock_units {
            let ok = self
                .try_decrement_stock_on(&mut tx, *catalog_item_id, *qty_units)
                .await?;
            if !ok {
                tx.rollback().await?;
                return Ok(None);
            }
        }

        let order = sqlx::query_as::<_, Order>(
            "INSERT INTO orders (store_id, warehouse_id, currency, subtotal, total, idempotency_key) \
             VALUES ($1,$2,$3,$4,$4,$5) RETURNING *",
        )
        .bind(store_id)
        .bind(warehouse_id)
        .bind(currency)
        .bind(subtotal)
        .bind(idempotency_key)
        .fetch_one(&mut *tx)
        .await?;

        let mut order_items = Vec::with_capacity(items.len());
        for item in items {
            let row = sqlx::query_as::<_, OrderItem>(
                "INSERT INTO order_items (order_id, catalog_item_id, tier, quantity, unit_price, line_total) \
                 VALUES ($1,$2,$3,$4,$5,$6) RETURNING *",
            )
            .bind(order.id)
            .bind(item.catalog_item_id)
            .bind(&item.tier)
            .bind(item.quantity)
            .bind(item.unit_price)
            .bind(item.line_total)
            .fetch_one(&mut *tx)
            .await?;
            order_items.push(row);
        }

        sqlx::query("INSERT INTO order_events (order_id, kind, detail) VALUES ($1, 'placed', $2)")
            .bind(order.id)
            .bind(format!("{} item(s)", order_items.len()))
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
        Ok(Some((order, order_items)))
    }

    /// Atomic, race-safe stock decrement inside the caller's transaction.
    async fn try_decrement_stock_on(
        &self,
        conn: &mut PgConnection,
        catalog_item_id: Uuid,
        qty_units: Decimal,
    ) -> AppResult<bool> {
        let result = sqlx::query(
            "UPDATE catalog_items SET stock_qty_units = stock_qty_units - $2, updated_at = now() \
             WHERE id = $1 AND stock_qty_units >= $2",
        )
        .bind(catalog_item_id)
        .bind(qty_units)
        .execute(conn)
        .await?;
        Ok(result.rows_affected() == 1)
    }

    pub async fn find_order(&self, id: Uuid) -> AppResult<Option<Order>> {
        let row = sqlx::query_as::<_, Order>("SELECT * FROM orders WHERE id = $1")
            .bind(id)
            .fetch_optional(self.db.read())
            .await?;
        Ok(row)
    }

    pub async fn list_items(&self, order_id: Uuid) -> AppResult<Vec<OrderItem>> {
        let rows = sqlx::query_as::<_, OrderItem>("SELECT * FROM order_items WHERE order_id = $1")
            .bind(order_id)
            .fetch_all(self.db.read())
            .await?;
        Ok(rows)
    }

    pub async fn list_orders_for_store(&self, store_id: Uuid) -> AppResult<Vec<Order>> {
        let rows = sqlx::query_as::<_, Order>(
            "SELECT * FROM orders WHERE store_id = $1 ORDER BY placed_at DESC",
        )
        .bind(store_id)
        .fetch_all(self.db.read())
        .await?;
        Ok(rows)
    }

    pub async fn list_orders_for_warehouse(&self, warehouse_id: Uuid) -> AppResult<Vec<Order>> {
        let rows = sqlx::query_as::<_, Order>(
            "SELECT * FROM orders WHERE warehouse_id = $1 ORDER BY placed_at DESC",
        )
        .bind(warehouse_id)
        .fetch_all(self.db.read())
        .await?;
        Ok(rows)
    }

    pub async fn set_status(&self, id: Uuid, status: &str) -> AppResult<()> {
        sqlx::query("UPDATE orders SET status = $2, updated_at = now() WHERE id = $1")
            .bind(id)
            .bind(status)
            .execute(self.db.write())
            .await?;
        sqlx::query(
            "INSERT INTO order_events (order_id, kind, detail) VALUES ($1, 'status_changed', $2)",
        )
        .bind(id)
        .bind(status)
        .execute(self.db.write())
        .await?;
        Ok(())
    }

    // ── bulk consolidation ──────────────────────────────────────────────

    /// The current open bulk order for this exact (warehouse, item, tier,
    /// window), if one exists — compatible demand pools into it rather than
    /// opening a duplicate.
    pub async fn find_open_bulk_order(
        &self,
        warehouse_id: Uuid,
        catalog_item_id: Uuid,
        tier: &str,
    ) -> AppResult<Option<BulkOrder>> {
        let row = sqlx::query_as::<_, BulkOrder>(
            "SELECT * FROM bulk_orders \
             WHERE warehouse_id = $1 AND catalog_item_id = $2 AND tier = $3 AND status = 'open' \
             ORDER BY created_at DESC LIMIT 1",
        )
        .bind(warehouse_id)
        .bind(catalog_item_id)
        .bind(tier)
        .fetch_optional(self.db.write())
        .await?;
        Ok(row)
    }

    pub async fn create_bulk_order(
        &self,
        warehouse_id: Uuid,
        catalog_item_id: Uuid,
        tier: &str,
    ) -> AppResult<BulkOrder> {
        let row = sqlx::query_as::<_, BulkOrder>(
            "INSERT INTO bulk_orders (warehouse_id, catalog_item_id, tier) VALUES ($1,$2,$3) RETURNING *",
        )
        .bind(warehouse_id)
        .bind(catalog_item_id)
        .bind(tier)
        .fetch_one(self.db.write())
        .await?;
        Ok(row)
    }

    pub async fn add_allocation(
        &self,
        bulk_order_id: Uuid,
        order_id: Uuid,
        store_id: Uuid,
        quantity: Decimal,
    ) -> AppResult<BulkOrderAllocation> {
        let row = sqlx::query_as::<_, BulkOrderAllocation>(
            "INSERT INTO bulk_order_allocations (bulk_order_id, order_id, store_id, quantity) \
             VALUES ($1,$2,$3,$4) RETURNING *",
        )
        .bind(bulk_order_id)
        .bind(order_id)
        .bind(store_id)
        .bind(quantity)
        .fetch_one(self.db.write())
        .await?;
        Ok(row)
    }

    pub async fn list_allocations(
        &self,
        bulk_order_id: Uuid,
    ) -> AppResult<Vec<BulkOrderAllocation>> {
        let rows = sqlx::query_as::<_, BulkOrderAllocation>(
            "SELECT * FROM bulk_order_allocations WHERE bulk_order_id = $1",
        )
        .bind(bulk_order_id)
        .fetch_all(self.db.read())
        .await?;
        Ok(rows)
    }

    pub async fn find_bulk_order(&self, id: Uuid) -> AppResult<Option<BulkOrder>> {
        let row = sqlx::query_as::<_, BulkOrder>("SELECT * FROM bulk_orders WHERE id = $1")
            .bind(id)
            .fetch_optional(self.db.read())
            .await?;
        Ok(row)
    }

    pub async fn list_bulk_orders_for_warehouse(
        &self,
        warehouse_id: Uuid,
    ) -> AppResult<Vec<BulkOrder>> {
        let rows = sqlx::query_as::<_, BulkOrder>(
            "SELECT * FROM bulk_orders WHERE warehouse_id = $1 ORDER BY created_at DESC",
        )
        .bind(warehouse_id)
        .fetch_all(self.db.read())
        .await?;
        Ok(rows)
    }

    // ── settlements and ledger ──────────────────────────────────────────

    /// Settle an order: one settlement row plus a balanced double-entry
    /// journal (debit the store, credit the warehouse), all in one
    /// transaction. `store_account_id`/`warehouse_account_id` are the
    /// account that owns each side (see `warehouses.account_id` /
    /// `stores.account_id`).
    pub async fn settle_order(
        &self,
        order_id: Uuid,
        store_account_id: Uuid,
        warehouse_account_id: Uuid,
        amount: Decimal,
        currency: &str,
    ) -> AppResult<Settlement> {
        let mut tx = self.db.write().begin().await?;

        let settlement = sqlx::query_as::<_, Settlement>(
            "INSERT INTO settlements (order_id, status, total_amount, currency) \
             VALUES ($1, 'completed', $2, $3) RETURNING *",
        )
        .bind(order_id)
        .bind(amount)
        .bind(currency)
        .fetch_one(&mut *tx)
        .await?;

        let journal_id: Uuid = sqlx::query_scalar(
            "INSERT INTO ledger_journals (kind, currency, order_id, settlement_id, memo) \
             VALUES ('order_settlement', $1, $2, $3, 'Order settlement') RETURNING id",
        )
        .bind(currency)
        .bind(order_id)
        .bind(settlement.id)
        .fetch_one(&mut *tx)
        .await?;

        sqlx::query(
            "INSERT INTO ledger_entries (journal_id, account_id, direction, amount, currency, memo) \
             VALUES ($1, $2, 'debit', $4, $3, 'Order settlement'), \
                    ($1, $5, 'credit', $4, $3, 'Order settlement')",
        )
        .bind(journal_id)
        .bind(store_account_id)
        .bind(currency)
        .bind(amount)
        .bind(warehouse_account_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(settlement)
    }

    pub async fn find_settlement_for_order(&self, order_id: Uuid) -> AppResult<Option<Settlement>> {
        let row = sqlx::query_as::<_, Settlement>("SELECT * FROM settlements WHERE order_id = $1")
            .bind(order_id)
            .fetch_optional(self.db.read())
            .await?;
        Ok(row)
    }

    pub async fn list_ledger_entries_for_account(
        &self,
        account_id: Uuid,
    ) -> AppResult<Vec<LedgerEntry>> {
        let rows = sqlx::query_as::<_, LedgerEntry>(
            "SELECT * FROM ledger_entries WHERE account_id = $1 ORDER BY created_at DESC",
        )
        .bind(account_id)
        .fetch_all(self.db.read())
        .await?;
        Ok(rows)
    }
}
