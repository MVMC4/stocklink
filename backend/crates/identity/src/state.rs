use std::sync::Arc;

use stocklink_shared::auth::{AccessTokenDenylist, AuthState, JwtKeys};
use stocklink_shared::database::redis::RedisPool;
use stocklink_shared::database::DbPool;
use stocklink_shared::middleware::InternalAuthState;
use stocklink_shared::utils::metrics::Metrics;

use crate::auth_service::AuthService;
use crate::config::Config;
use crate::email::gateway::{EmailGateway, LoggingEmailGateway};
use crate::email::resend::ResendEmailGateway;
use crate::repositories::{AdminRepository, AuthRepository, OnboardingRepository};
use crate::services::{AdminService, OnboardingService};
use crate::sms::gateway::{NoopSmsGateway, SmsGateway};
use crate::sms::AfricaTalkingSmsGateway;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub db: DbPool,
    pub redis: RedisPool,
    pub jwt: Arc<JwtKeys>,
    pub metrics: Arc<Metrics>,
    pub denylist: AccessTokenDenylist,

    pub auth: AuthService,
    pub onboarding: OnboardingService,
    pub admin: AdminService,
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

        let email: Arc<dyn EmailGateway> = if config.env.is_production_like() {
            Arc::new(ResendEmailGateway::new(&config.email))
        } else {
            // Local dev/demo without real provider setup still needs to
            // "send" OTP email; the code reaches the caller through
            // `dev_code` instead (see `config::validate_otp_dev_echo`).
            Arc::new(LoggingEmailGateway)
        };
        let sms: Option<Arc<dyn SmsGateway>> = config.africas_talking.as_ref().map(|cfg| {
            let gateway: Arc<dyn SmsGateway> = if config.env.is_production_like() {
                Arc::new(AfricaTalkingSmsGateway::new(cfg))
            } else {
                Arc::new(NoopSmsGateway)
            };
            gateway
        });

        let auth_repo = AuthRepository::new(db.clone());
        let onboarding_repo = OnboardingRepository::new(db.clone());
        let admin_repo = AdminRepository::new(db.clone());

        let auth = AuthService::new(
            auth_repo.clone(),
            email,
            sms,
            jwt.clone(),
            denylist.clone(),
            redis.clone(),
            config.otp.dev_echo_enabled,
            config.jwt.refresh_ttl,
        );
        let onboarding = OnboardingService::new(onboarding_repo, auth_repo);
        let admin = AdminService::new(admin_repo, config.admin_links.clone());

        Self {
            config: Arc::new(config),
            db,
            redis,
            jwt,
            metrics,
            denylist,
            auth,
            onboarding,
            admin,
        }
    }
}
