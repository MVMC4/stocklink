//! End-to-end commerce contract tests, now spanning two services: identity
//! (accounts/warehouses/stores — seeded directly against its database) and
//! commerce (catalogue/cart/orders — exercised through the real service
//! code, calling out to a *live* identity instance via `IdentityClient` for
//! ownership/lookup, exactly as production does).
//!
//! Exercises the acceptance criteria named in docs/WORK_ORDERS.md WO-06:
//! pricing, idempotency, allocation totals, and ownership scoping.
//!
//! Requires the full local stack up and migrated:
//!   docker compose -f docker/compose.dev.yml up -d identity-db commerce-db redis
//!   cargo run --bin stocklink-identity -- migrate
//!   cargo run --bin stocklink-commerce -- migrate
//!   cargo run --bin stocklink-identity   (leave running in another shell)
//!   cargo test --test commerce_flow -- --ignored

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use sqlx::PgPool;
use std::time::Duration;
use stocklink_commerce::clients::IdentityClient;
use uuid::Uuid;

use stocklink_commerce::repositories::order_repository::NewOrderItem;
use stocklink_commerce::repositories::{CatalogRepository, OrderRepository};
use stocklink_commerce::services::{CatalogService, OrderService};
use stocklink_shared::config::DatabaseConfig;
use stocklink_shared::database::DbPool;

fn commerce_database_url() -> String {
    std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://stocklink:stocklink@localhost:5443/stocklink_commerce".to_string()
    })
}

fn identity_database_url() -> String {
    std::env::var("IDENTITY_DATABASE_URL").unwrap_or_else(|_| {
        "postgres://stocklink:stocklink@localhost:5443/stocklink_identity".to_string()
    })
}

fn identity_service_url() -> String {
    std::env::var("IDENTITY_SERVICE_URL").unwrap_or_else(|_| "http://localhost:8081".to_string())
}

fn internal_token() -> String {
    std::env::var("INTERNAL_SERVICE_TOKEN")
        .unwrap_or_else(|_| "dev-only-internal-token".to_string())
}

async fn commerce_pool() -> DbPool {
    let cfg = DatabaseConfig {
        url: commerce_database_url(),
        read_replica_url: None,
        max_connections: 5,
        min_connections: 1,
        acquire_timeout: Duration::from_secs(5),
    };
    stocklink_shared::database::connect(&cfg)
        .await
        .expect("connect to commerce database (see this file's header for setup)")
}

async fn identity_pool() -> PgPool {
    PgPool::connect(&identity_database_url())
        .await
        .expect("connect to identity database (see this file's header for setup)")
}

fn identity_client() -> IdentityClient {
    IdentityClient::new(identity_service_url(), internal_token())
}

async fn seed_account(pg: &PgPool, display_name: &str) -> Uuid {
    sqlx::query_scalar("INSERT INTO accounts (display_name) VALUES ($1) RETURNING id")
        .bind(display_name)
        .fetch_one(pg)
        .await
        .expect("seed account")
}

async fn seed_warehouse(pg: &PgPool, account_id: Uuid) -> Uuid {
    sqlx::query_scalar(
        "INSERT INTO warehouses (account_id, name, region) VALUES ($1, 'Test Warehouse', 'Gaborone') RETURNING id",
    )
    .bind(account_id)
    .fetch_one(pg)
    .await
    .expect("seed warehouse")
}

async fn seed_store(pg: &PgPool, account_id: Uuid) -> Uuid {
    sqlx::query_scalar(
        "INSERT INTO stores (account_id, name, region) VALUES ($1, 'Test Store', 'Gaborone') RETURNING id",
    )
    .bind(account_id)
    .fetch_one(pg)
    .await
    .expect("seed store")
}

#[tokio::test]
#[ignore = "requires identity + commerce databases and a live identity service"]
async fn checkout_prices_from_the_catalogue_server_side_never_the_client() {
    let db = commerce_pool().await;
    let idpg = identity_pool().await;
    let owner = seed_account(&idpg, "Owner").await;
    let warehouse_id = seed_warehouse(&idpg, owner).await;
    let store_owner = seed_account(&idpg, "Store owner").await;
    let store_id = seed_store(&idpg, store_owner).await;

    let catalog = CatalogService::new(CatalogRepository::new(db.clone()));
    let item = catalog
        .publish_item(
            warehouse_id,
            "SKU-1",
            "Widget",
            None,
            "BWP",
            "unit",
            dec!(10.00),
            Some(10),
            Some(dec!(90.00)),
            None,
            None,
            dec!(1000),
        )
        .await
        .expect("publish item");

    catalog
        .add_to_cart(store_id, item.id, "case", dec!(2))
        .await
        .expect("add to cart");

    let orders = OrderService::new(
        OrderRepository::new(db.clone()),
        CatalogRepository::new(db.clone()),
        identity_client(),
    );
    let created = orders.checkout(store_id, None).await.expect("checkout");

    assert_eq!(created.len(), 1);
    // 2 cases at 90.00/case (NOT 2 * unit_price * case_size = 200) — proves
    // the tier price is used, not a client-suppliable number.
    assert_eq!(created[0].total, dec!(180.00));
}

#[tokio::test]
#[ignore = "requires identity + commerce databases and a live identity service"]
async fn repeated_checkout_with_the_same_idempotency_key_does_not_duplicate_orders() {
    let db = commerce_pool().await;
    let idpg = identity_pool().await;
    let owner = seed_account(&idpg, "Owner").await;
    let warehouse_id = seed_warehouse(&idpg, owner).await;
    let store_owner = seed_account(&idpg, "Store owner").await;
    let store_id = seed_store(&idpg, store_owner).await;

    let order_repo = OrderRepository::new(db.clone());
    let pg = db.write().clone();
    let item_id: Uuid = sqlx::query_scalar(
        "INSERT INTO catalog_items (warehouse_id, sku, name, unit_price, stock_qty_units) \
         VALUES ($1, 'SKU-2', 'Gadget', 5.00, 100) RETURNING id",
    )
    .bind(warehouse_id)
    .fetch_one(&pg)
    .await
    .expect("seed catalog item");

    let idempotency_key = format!("idem-key-{}", Uuid::new_v4());
    let items = vec![NewOrderItem {
        catalog_item_id: item_id,
        tier: "unit".to_string(),
        quantity: dec!(1),
        unit_price: dec!(5.00),
        line_total: dec!(5.00),
    }];
    let stock_units = vec![(item_id, dec!(1))];

    let first = order_repo
        .create_order_with_stock_check(
            store_id,
            warehouse_id,
            "BWP",
            Some(&idempotency_key),
            &items,
            &stock_units,
        )
        .await
        .expect("first checkout")
        .expect("stock available");

    let existing = order_repo
        .find_by_idempotency_key(store_id, &idempotency_key)
        .await
        .expect("lookup")
        .expect("existing order found");
    assert_eq!(existing.id, first.0.id);

    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM orders WHERE idempotency_key = $1")
        .bind(&idempotency_key)
        .fetch_one(&pg)
        .await
        .expect("count orders");
    assert_eq!(count, 1);
}

#[tokio::test]
#[ignore = "requires identity + commerce databases and a live identity service"]
async fn bulk_order_allocation_totals_equal_the_bulk_order() {
    let db = commerce_pool().await;
    let idpg = identity_pool().await;
    let owner = seed_account(&idpg, "Owner").await;
    let warehouse_id = seed_warehouse(&idpg, owner).await;

    let store_a_owner = seed_account(&idpg, "Store A owner").await;
    let store_a = seed_store(&idpg, store_a_owner).await;
    let store_b_owner = seed_account(&idpg, "Store B owner").await;
    let store_b = seed_store(&idpg, store_b_owner).await;

    let pg = db.write().clone();
    let item_id: Uuid = sqlx::query_scalar(
        "INSERT INTO catalog_items (warehouse_id, sku, name, unit_price, case_size, case_price, stock_qty_units) \
         VALUES ($1, 'SKU-3', 'Pooled Item', 1.00, 10, 8.00, 10000) RETURNING id",
    )
    .bind(warehouse_id)
    .fetch_one(&pg)
    .await
    .expect("seed catalog item");

    let catalog = CatalogRepository::new(db.clone());
    catalog
        .upsert_cart_item(store_a, item_id, "case", dec!(3))
        .await
        .unwrap();
    catalog
        .upsert_cart_item(store_b, item_id, "case", dec!(5))
        .await
        .unwrap();

    let orders = OrderService::new(
        OrderRepository::new(db.clone()),
        CatalogRepository::new(db.clone()),
        identity_client(),
    );
    orders
        .checkout(store_a, None)
        .await
        .expect("store A checkout");
    orders
        .checkout(store_b, None)
        .await
        .expect("store B checkout");

    let order_repo = OrderRepository::new(db.clone());
    let bulk = order_repo
        .find_open_bulk_order(warehouse_id, item_id, "case")
        .await
        .expect("find bulk order")
        .expect("a bulk order was formed");
    let allocations = order_repo
        .list_allocations(bulk.id)
        .await
        .expect("list allocations");

    assert_eq!(
        allocations.len(),
        2,
        "both stores' orders pooled into the same bulk order"
    );
    let total: Decimal = allocations.iter().map(|a| a.quantity).sum();
    assert_eq!(
        total,
        dec!(8),
        "allocation totals (3 + 5) equal the pooled demand"
    );
}

#[tokio::test]
#[ignore = "requires identity + commerce databases and a live identity service"]
async fn a_store_cannot_see_another_stores_order() {
    let db = commerce_pool().await;
    let idpg = identity_pool().await;
    let owner = seed_account(&idpg, "Owner").await;
    let warehouse_id = seed_warehouse(&idpg, owner).await;
    let store_a_owner = seed_account(&idpg, "Store A owner").await;
    let store_a = seed_store(&idpg, store_a_owner).await;
    let store_b_owner = seed_account(&idpg, "Store B owner").await;
    let store_b = seed_store(&idpg, store_b_owner).await;

    let pg = db.write().clone();
    let item_id: Uuid = sqlx::query_scalar(
        "INSERT INTO catalog_items (warehouse_id, sku, name, unit_price, stock_qty_units) \
         VALUES ($1, 'SKU-4', 'Scoped Item', 2.00, 100) RETURNING id",
    )
    .bind(warehouse_id)
    .fetch_one(&pg)
    .await
    .expect("seed catalog item");

    let order_repo = OrderRepository::new(db.clone());
    let items = vec![NewOrderItem {
        catalog_item_id: item_id,
        tier: "unit".to_string(),
        quantity: dec!(1),
        unit_price: dec!(2.00),
        line_total: dec!(2.00),
    }];
    let (order_a, _) = order_repo
        .create_order_with_stock_check(
            store_a,
            warehouse_id,
            "BWP",
            None,
            &items,
            &[(item_id, dec!(1))],
        )
        .await
        .expect("create order")
        .expect("stock available");

    let orders = OrderService::new(
        order_repo,
        CatalogRepository::new(db.clone()),
        identity_client(),
    );

    assert!(orders.get_for_store(store_a, order_a.id).await.is_ok());
    let result = orders.get_for_store(store_b, order_a.id).await;
    assert!(
        result.is_err(),
        "store B must not be able to load store A's order"
    );
}
