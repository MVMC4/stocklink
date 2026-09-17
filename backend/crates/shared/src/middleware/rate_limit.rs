//! Redis-backed fixed-window rate limiting.
//!
//! [`check_and_increment`] is the reusable primitive: a fixed window keyed by
//! whatever the caller wants to throttle (an IP, an OTP contact identifier,
//! an account id). [`rate_limit_mw`] applies it per-IP to every request;
//! identity's OTP flow calls it directly, keyed by contact identifier, for
//! OTP-specific throttling (tighter limits than the general per-IP one).

use std::net::SocketAddr;

use axum::extract::{ConnectInfo, Request, State};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use deadpool_redis::redis::AsyncCommands;

use crate::database::redis::RedisPool;
use crate::errors::{AppError, AppResult};

use super::MiddlewareState;

/// Increment `key`'s counter in a fixed window of `window_secs`, creating it
/// with that expiry on first use. Returns `true` if the caller is still
/// within `limit` (the increment happened either way — a rejected request
/// still counts, so a caller can't reset their budget by retrying).
pub async fn check_and_increment(
    redis: &RedisPool,
    key: &str,
    limit: u32,
    window_secs: u64,
) -> AppResult<bool> {
    let mut conn = redis.get().await?;
    let count: i64 = conn.incr(key, 1).await?;
    if count == 1 {
        // Only set the expiry on the first hit in a window; a repeated EXPIRE
        // on every call would let a steady trickle of requests keep pushing
        // the window forward forever (never resetting).
        let _: () = conn.expire(key, window_secs as i64).await?;
    }
    Ok(count as u32 <= limit)
}

/// Per-IP rate limit applied to every request. Behind a trusted gateway
/// (`trust_proxy_headers`), keys on the first `X-Forwarded-For` entry — the
/// gateway overwrites this header with the real peer address (never appends
/// to a client-supplied value; see `docker/nginx/gateway.conf`), so trusting
/// it here is safe. Otherwise keys on the direct TCP peer address.
pub async fn rate_limit_mw(
    State(mw): State<MiddlewareState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    request: Request,
    next: Next,
) -> Response {
    if !mw.rate_limit.enabled {
        return next.run(request).await;
    }

    let client_key = if mw.trust_proxy_headers {
        request
            .headers()
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.split(',').next())
            .map(str::trim)
            .map(str::to_string)
            .unwrap_or_else(|| peer.ip().to_string())
    } else {
        peer.ip().to_string()
    };

    let redis_key = format!("ratelimit:ip:{client_key}");
    match check_and_increment(
        &mw.redis,
        &redis_key,
        mw.rate_limit.requests_per_minute,
        mw.rate_limit.window_secs,
    )
    .await
    {
        Ok(true) => next.run(request).await,
        Ok(false) => {
            mw.metrics.rate_limited();
            AppError::RateLimited.into_response()
        }
        // Redis being unreachable should not take the whole API down —
        // health/readiness already reports Redis as down separately.
        Err(err) => {
            tracing::warn!(error = ?err, "rate limiter could not reach redis; allowing request");
            next.run(request).await
        }
    }
}

#[cfg(test)]
mod tests {
    use super::check_and_increment;
    use deadpool_redis::{Config as DeadpoolConfig, Runtime};

    async fn test_pool() -> crate::database::redis::RedisPool {
        let url =
            std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string());
        DeadpoolConfig::from_url(url)
            .create_pool(Some(Runtime::Tokio1))
            .expect("create redis pool")
    }

    /// Red-tested per AGENTS.md rule 5 / WO-02's own example ("limiter
    /// returns 429 after budget"): this test fails (stays `true` forever) if
    /// the limit check is removed, and passes once it's restored.
    #[tokio::test]
    #[ignore = "requires Redis"]
    async fn limiter_allows_within_budget_then_rejects_over_budget() {
        let redis = test_pool().await;
        let key = format!("test:ratelimit:{}", uuid::Uuid::new_v4());

        for _ in 0..3 {
            assert!(check_and_increment(&redis, &key, 3, 60).await.unwrap());
        }
        assert!(
            !check_and_increment(&redis, &key, 3, 60).await.unwrap(),
            "the 4th request within the window must be rejected"
        );
    }

    #[tokio::test]
    #[ignore = "requires Redis"]
    async fn distinct_keys_have_independent_budgets() {
        let redis = test_pool().await;
        let key_a = format!("test:ratelimit:{}", uuid::Uuid::new_v4());
        let key_b = format!("test:ratelimit:{}", uuid::Uuid::new_v4());

        assert!(check_and_increment(&redis, &key_a, 1, 60).await.unwrap());
        assert!(!check_and_increment(&redis, &key_a, 1, 60).await.unwrap());
        // key_b's budget is untouched by key_a's use.
        assert!(check_and_increment(&redis, &key_b, 1, 60).await.unwrap());
    }
}
