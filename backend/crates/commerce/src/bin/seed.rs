//! `cargo run --bin seed` — placeholder.
//!
//! StockLink's schema (warehouses, stores, carriers, catalogue, bulk orders)
//! does not exist yet (see `seed.sql`), so this seeds nothing. It still
//! refuses to run in production or staging, and still reports what happened,
//! honestly: no demo data yet.
//!
//! Refuses to run when `STOCKLINK_ENV` is `production` or `staging` — demo
//! data must never touch a real environment.

use sqlx::postgres::PgPoolOptions;
use sqlx::Executor;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let env = std::env::var("STOCKLINK_ENV").unwrap_or_else(|_| "development".into());
    if matches!(env.as_str(), "production" | "staging") {
        eprintln!("refusing to seed demo data in `{env}`");
        std::process::exit(1);
    }
    let url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://stocklink:stocklink@localhost:5432/stocklink".into());
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&url)
        .await?;

    // Postgres' simple-query protocol runs the whole script in one call.
    // Currently a no-op: see seed.sql for why.
    pool.execute(include_str!("seed.sql")).await?;

    println!("no demo data seeded yet — StockLink's schema lands in WO-04 (baseline) and WO-06 (commerce); see seed.sql");
    Ok(())
}
