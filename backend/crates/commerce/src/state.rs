use std::sync::Arc;

use stocklink_shared::auth::{AccessTokenDenylist, AuthState, JwtKeys};
use stocklink_shared::database::redis::RedisPool;
use stocklink_shared::database::DbPool;
use stocklink_shared::middleware::InternalAuthState;
use stocklink_shared::utils::metrics::Metrics;

use crate::clients::{IdentityClient, NotificationsClient};
use crate::config::Config;
use crate::repositories::{CatalogRepository, OrderRepository};
use crate::services::{CatalogService, OrderService};

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub db: DbPool,
    pub redis: RedisPool,
    pub jwt: Arc<JwtKeys>,
    pub metrics: Arc<Metrics>,
    /// Checks the *same* Redis-backed denylist identity writes to on
    /// logout — revocation is visible to every service immediately (module
    /// docs in `stocklink_shared::auth::denylist` explain the cache/staleness
    /// tradeoff, which applies per-service-instance the same way it applied
    /// per-monolith-instance before the split).
    pub denylist: AccessTokenDenylist,

    pub catalog: CatalogService,
    pub orders: OrderService,
    pub identity: IdentityClient,
    pub notifications: NotificationsClient,
}

impl AuthState for AppState {
    fn jwt(&self) -> &JwtKeys {
        &self.jwt
    }

    fn denylist(&self) -> &AccessTokenDenylist {
        &self.denylist
    }
}

impl InternalAuthState for AppState {
    fn internal_token(&self) -> &str {
        &self.config.internal_token
    }
}

impl AppState {
    pub fn new(config: Config, db: DbPool, redis: RedisPool) -> Self {
        let jwt = Arc::new(JwtKeys::new(&config.jwt));
        let denylist = AccessTokenDenylist::new(redis.clone());
        let metrics = Arc::new(Metrics::default());

        let catalog_repo = CatalogRepository::new(db.clone());
        let order_repo = OrderRepository::new(db.clone());
        let identity = IdentityClient::new(
            config.identity_service_url.clone(),
            config.internal_token.clone(),
        );
        let notifications = NotificationsClient::new(
            config.notifications_service_url.clone(),
            config.internal_token.clone(),
        );

        let catalog = CatalogService::new(catalog_repo.clone());
        let orders = OrderService::new(order_repo, catalog_repo, identity.clone());

        Self {
            config: Arc::new(config),
            db,
            redis,
            jwt,
            metrics,
            denylist,
            catalog,
            orders,
            identity,
            notifications,
        }
    }
}
