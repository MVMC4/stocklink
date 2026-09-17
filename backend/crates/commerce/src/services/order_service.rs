//! Checkout, bulk consolidation and settlement.

use std::collections::HashMap;

use rust_decimal::Decimal;
use uuid::Uuid;

use crate::clients::IdentityClient;
use crate::models::order::{BulkOrder, Order};
use crate::repositories::order_repository::NewOrderItem;
use crate::repositories::{CatalogRepository, OrderRepository};
use stocklink_shared::errors::{AppError, AppResult};

#[derive(Clone)]
pub struct OrderService {
    orders: OrderRepository,
    catalog: CatalogRepository,
    identity: IdentityClient,
}

/// One priced cart line, grouped by warehouse during checkout.
struct CheckoutLine {
    catalog_item_id: Uuid,
    tier: String,
    quantity: Decimal,
    unit_price: Decimal,
    line_total: Decimal,
}

impl OrderService {
    pub fn new(
        orders: OrderRepository,
        catalog: CatalogRepository,
        identity: IdentityClient,
    ) -> Self {
        Self {
            orders,
            catalog,
            identity,
        }
    }

    /// Checks out everything currently in `store_id`'s cart, grouped into one
    /// order per warehouse (a cart can span many warehouses — see
    /// `docs/VISION.md`). Idempotent: calling again with the same
    /// `idempotency_key` before the cart is re-populated returns the
    /// previously created orders rather than double-charging.
    ///
    /// Pricing is always computed here, server-side, from the catalogue's
    /// current price — never trusted from the client (rule 7).
    pub async fn checkout(
        &self,
        store_id: Uuid,
        idempotency_key: Option<&str>,
    ) -> AppResult<Vec<Order>> {
        let cart_items = self.catalog.list_cart_items(store_id).await?;
        if cart_items.is_empty() {
            return Err(AppError::Validation {
                field: "cart".into(),
                message: "cart is empty".into(),
            });
        }

        // Group cart lines by warehouse — one order per warehouse.
        let mut by_warehouse: HashMap<Uuid, Vec<CheckoutLine>> = HashMap::new();
        for cart_item in &cart_items {
            let item = self
                .catalog
                .find_item(cart_item.catalog_item_id)
                .await?
                .ok_or(AppError::NotFound("catalog item"))?;
            let unit_price = item
                .price_for_tier(&cart_item.tier)
                .ok_or(AppError::Conflict(format!(
                    "{} is no longer available in that tier",
                    item.name
                )))?;
            let line_total = unit_price * cart_item.quantity;
            by_warehouse
                .entry(item.warehouse_id)
                .or_default()
                .push(CheckoutLine {
                    catalog_item_id: cart_item.catalog_item_id,
                    tier: cart_item.tier.clone(),
                    quantity: cart_item.quantity,
                    unit_price,
                    line_total,
                });
        }

        let mut created = Vec::with_capacity(by_warehouse.len());
        for (warehouse_id, lines) in by_warehouse {
            let mut new_items = Vec::with_capacity(lines.len());
            let mut stock_units = Vec::with_capacity(lines.len());
            for line in &lines {
                let item = self
                    .catalog
                    .find_item(line.catalog_item_id)
                    .await?
                    .ok_or(AppError::NotFound("catalog item"))?;
                let units_per_tier = item.units_per_tier(&line.tier).unwrap_or(1) as i64;
                stock_units.push((
                    line.catalog_item_id,
                    line.quantity * Decimal::from(units_per_tier),
                ));
                new_items.push(NewOrderItem {
                    catalog_item_id: line.catalog_item_id,
                    tier: line.tier.clone(),
                    quantity: line.quantity,
                    unit_price: line.unit_price,
                    line_total: line.line_total,
                });
            }

            let per_warehouse_key = idempotency_key.map(|k| format!("{k}:{warehouse_id}"));
            if let Some(key) = &per_warehouse_key {
                if let Some(existing) = self.orders.find_by_idempotency_key(store_id, key).await? {
                    created.push(existing);
                    continue;
                }
            }

            let result = self
                .orders
                .create_order_with_stock_check(
                    store_id,
                    warehouse_id,
                    "BWP",
                    per_warehouse_key.as_deref(),
                    &new_items,
                    &stock_units,
                )
                .await?;
            let Some((order, _items)) = result else {
                return Err(AppError::Conflict(
                    "not enough stock for one or more items".into(),
                ));
            };

            for line in &lines {
                self.pool_into_bulk_order(
                    warehouse_id,
                    line.catalog_item_id,
                    &line.tier,
                    order.id,
                    store_id,
                    line.quantity,
                )
                .await?;
            }

            created.push(order);
        }

        self.catalog.clear_cart(store_id).await?;
        Ok(created)
    }

    /// Joins (or opens) the current open bulk order for this exact
    /// (warehouse, item, tier) and records this order's allocation into it.
    async fn pool_into_bulk_order(
        &self,
        warehouse_id: Uuid,
        catalog_item_id: Uuid,
        tier: &str,
        order_id: Uuid,
        store_id: Uuid,
        quantity: Decimal,
    ) -> AppResult<BulkOrder> {
        let bulk_order = match self
            .orders
            .find_open_bulk_order(warehouse_id, catalog_item_id, tier)
            .await?
        {
            Some(existing) => existing,
            None => {
                self.orders
                    .create_bulk_order(warehouse_id, catalog_item_id, tier)
                    .await?
            }
        };
        self.orders
            .add_allocation(bulk_order.id, order_id, store_id, quantity)
            .await?;
        Ok(bulk_order)
    }

    pub async fn get(&self, order_id: Uuid) -> AppResult<Order> {
        self.orders
            .find_order(order_id)
            .await?
            .ok_or(AppError::NotFound("order"))
    }

    /// Ownership check: a store may only see its own orders. Returns
    /// `NotFound` (not `Forbidden`) for someone else's order, so existence
    /// isn't leaked either.
    pub async fn get_for_store(&self, store_id: Uuid, order_id: Uuid) -> AppResult<Order> {
        let order = self.get(order_id).await?;
        if order.store_id != store_id {
            return Err(AppError::NotFound("order"));
        }
        Ok(order)
    }

    pub async fn get_for_warehouse(&self, warehouse_id: Uuid, order_id: Uuid) -> AppResult<Order> {
        let order = self.get(order_id).await?;
        if order.warehouse_id != warehouse_id {
            return Err(AppError::NotFound("order"));
        }
        Ok(order)
    }

    pub async fn list_for_store(&self, store_id: Uuid) -> AppResult<Vec<Order>> {
        self.orders.list_orders_for_store(store_id).await
    }

    pub async fn list_for_warehouse(&self, warehouse_id: Uuid) -> AppResult<Vec<Order>> {
        self.orders.list_orders_for_warehouse(warehouse_id).await
    }

    /// Advance an order's status. Only the owning warehouse may do this
    /// (enforced by the controller, which loads via `get_for_warehouse`
    /// first). Settling into the ledger happens on transition to
    /// `delivered`.
    pub async fn advance_status(
        &self,
        warehouse_id: Uuid,
        order_id: Uuid,
        status: &str,
    ) -> AppResult<Order> {
        const VALID: &[&str] = &[
            "confirmed",
            "packed",
            "in_transit",
            "delivered",
            "cancelled",
        ];
        if !VALID.contains(&status) {
            return Err(AppError::Validation {
                field: "status".into(),
                message: format!("must be one of {VALID:?}"),
            });
        }
        let order = self.get_for_warehouse(warehouse_id, order_id).await?;
        self.orders.set_status(order.id, status).await?;

        if status == "delivered" {
            self.settle(&order).await?;
        }
        self.orders
            .find_order(order.id)
            .await?
            .ok_or(AppError::NotFound("order"))
    }

    async fn settle(&self, order: &Order) -> AppResult<()> {
        if self
            .orders
            .find_settlement_for_order(order.id)
            .await?
            .is_some()
        {
            return Ok(()); // already settled — idempotent
        }
        let warehouse = self.identity.get_warehouse(order.warehouse_id).await?;
        let store = self.identity.get_store(order.store_id).await?;
        self.orders
            .settle_order(
                order.id,
                store.account_id,
                warehouse.account_id,
                order.total,
                &order.currency,
            )
            .await?;
        Ok(())
    }
}
