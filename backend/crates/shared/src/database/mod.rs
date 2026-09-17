//! The Postgres connection pool every service holds one of, split into a
//! write pool (the primary) and a read pool (a replica, when configured).
//! Repositories pick `.write()` for anything inside a transaction or that
//! mutates state, and `.read()` for plain lookups — see
//! `docs/ARCHITECTURE.md`'s "Database" section for why a repository is
//! trusted to make that call itself rather than the pool inferring it from
//! the query.

pub mod redis;

use sqlx::postgres::{PgPool, PgPoolOptions};

use crate::config::DatabaseConfig;

#[derive(Clone)]
pub struct DbPool {
    write: PgPool,
    read: PgPool,
}

/// Connect the write pool (the primary) and, when a read replica is
/// configured, a second pool for reads — otherwise reads share the primary
/// pool rather than opening a second connection to the same database for
/// no benefit.
pub async fn connect(config: &DatabaseConfig) -> Result<DbPool, sqlx::Error> {
    let write = PgPoolOptions::new()
        .max_connections(config.max_connections)
        .min_connections(config.min_connections)
        .acquire_timeout(config.acquire_timeout)
        .connect(&config.url)
        .await?;

    let read = match &config.read_replica_url {
        Some(url) => {
            PgPoolOptions::new()
                .max_connections(config.max_connections)
                .min_connections(config.min_connections)
                .acquire_timeout(config.acquire_timeout)
                .connect(url)
                .await?
        }
        None => write.clone(),
    };

    Ok(DbPool { write, read })
}

impl DbPool {
    pub fn write(&self) -> &PgPool {
        &self.write
    }

    pub fn read(&self) -> &PgPool {
        &self.read
    }

    /// Used by `/health/ready` — a real round trip, not just "the pool
    /// object exists", so a database that's up but unreachable (wrong
    /// network, exhausted connections) is reported honestly.
    pub async fn is_healthy(&self) -> bool {
        sqlx::query("SELECT 1").execute(&self.write).await.is_ok()
    }
}
