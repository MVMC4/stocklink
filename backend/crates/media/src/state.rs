use std::sync::Arc;

use stocklink_shared::auth::{AccessTokenDenylist, AuthState, JwtKeys};
use stocklink_shared::database::redis::RedisPool;
use stocklink_shared::database::DbPool;
use stocklink_shared::middleware::InternalAuthState;
use stocklink_shared::utils::metrics::Metrics;

use crate::config::Config;
use crate::repositories::MediaRepository;
use crate::services::MediaService;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub db: DbPool,
    pub redis: RedisPool,
    pub jwt: Arc<JwtKeys>,
    pub metrics: Arc<Metrics>,
    pub denylist: AccessTokenDenylist,

    pub media: MediaService,
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

        let media_repo = MediaRepository::new(db.clone());
        let media = MediaService::new(media_repo, config.public_media.clone());

        Self {
            config: Arc::new(config),
            db,
            redis,
            jwt,
            metrics,
            denylist,
            media,
        }
    }
}
