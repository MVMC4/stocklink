//! Redis connection pool. A thin alias, not a wrapper: every caller already
//! reaches for `deadpool_redis`/`redis` directly for the command they need
//! (`AsyncCommands::incr`, `.expire`, ...) — wrapping that in a bespoke API
//! here would just be a second thing to keep in sync with every new command
//! a caller wants.

use crate::config::RedisConfig;

pub type RedisPool = deadpool_redis::Pool;

pub fn connect(config: &RedisConfig) -> Result<RedisPool, deadpool_redis::CreatePoolError> {
    let cfg = deadpool_redis::Config::from_url(&config.url);
    cfg.builder()
        .map_err(deadpool_redis::CreatePoolError::Config)?
        .max_size(config.pool_size)
        .runtime(deadpool_redis::Runtime::Tokio1)
        .build()
        .map_err(deadpool_redis::CreatePoolError::Build)
}

/// Used by `/health/ready` — a real `PING`, not just "the pool exists".
pub async fn is_healthy(pool: &RedisPool) -> bool {
    let Ok(mut conn) = pool.get().await else {
        return false;
    };
    deadpool_redis::redis::cmd("PING")
        .query_async::<_, String>(&mut conn)
        .await
        .is_ok()
}
