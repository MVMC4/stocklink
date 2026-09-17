//! Access tokens are stateless JWTs, verified without a database round
//! trip — which means logging out can't just delete a row. Instead logout
//! marks the token's `jti` revoked here, in Redis, with a TTL matching its
//! own remaining lifetime: it only ever needs to be remembered for exactly
//! as long as it would otherwise still verify, never longer.

use uuid::Uuid;

use crate::database::redis::RedisPool;
use crate::errors::AppResult;

#[derive(Clone)]
pub struct AccessTokenDenylist {
    redis: RedisPool,
}

impl AccessTokenDenylist {
    pub fn new(redis: RedisPool) -> Self {
        Self { redis }
    }

    fn key(jti: Uuid) -> String {
        format!("auth:denylist:{jti}")
    }

    /// `remaining_ttl_secs` should be [`TokenClaims::remaining_ttl_secs`] —
    /// an already-expired token needs no entry, since it will fail
    /// verification on its own from the moment it expires.
    ///
    /// [`TokenClaims::remaining_ttl_secs`]: crate::auth::claims::TokenClaims::remaining_ttl_secs
    pub async fn revoke(&self, jti: Uuid, remaining_ttl_secs: i64) -> AppResult<()> {
        if remaining_ttl_secs <= 0 {
            return Ok(());
        }
        let mut conn = self.redis.get().await?;
        let _: () = deadpool_redis::redis::cmd("SET")
            .arg(Self::key(jti))
            .arg(1)
            .arg("EX")
            .arg(remaining_ttl_secs)
            .query_async(&mut conn)
            .await?;
        Ok(())
    }

    pub async fn is_revoked(&self, jti: Uuid) -> AppResult<bool> {
        let mut conn = self.redis.get().await?;
        let exists: bool = deadpool_redis::redis::cmd("EXISTS")
            .arg(Self::key(jti))
            .query_async(&mut conn)
            .await?;
        Ok(exists)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn test_pool() -> RedisPool {
        let url =
            std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string());
        deadpool_redis::Config::from_url(url)
            .create_pool(Some(deadpool_redis::Runtime::Tokio1))
            .expect("create redis pool")
    }

    /// Red test: a `jti` this denylist never saw must read as not revoked —
    /// otherwise every token would be rejected regardless of logout.
    #[tokio::test]
    #[ignore = "requires Redis"]
    async fn unknown_jti_is_not_revoked() {
        let denylist = AccessTokenDenylist::new(test_pool().await);
        assert!(!denylist.is_revoked(Uuid::new_v4()).await.unwrap());
    }

    #[tokio::test]
    #[ignore = "requires Redis"]
    async fn revoked_jti_reads_back_as_revoked() {
        let denylist = AccessTokenDenylist::new(test_pool().await);
        let jti = Uuid::new_v4();
        denylist.revoke(jti, 60).await.unwrap();
        assert!(denylist.is_revoked(jti).await.unwrap());
    }

    #[tokio::test]
    #[ignore = "requires Redis"]
    async fn revoking_with_zero_remaining_ttl_is_a_no_op() {
        let denylist = AccessTokenDenylist::new(test_pool().await);
        let jti = Uuid::new_v4();
        denylist.revoke(jti, 0).await.unwrap();
        assert!(!denylist.is_revoked(jti).await.unwrap());
    }
}
