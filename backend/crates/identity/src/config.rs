//! Identity service configuration — accounts, OTP auth, onboarding
//! (warehouse/store/carrier), admin console.

use stocklink_shared::config::{
    enforce_https_from_env, required, trust_proxy_headers_from_env, AfricaTalkingConfig,
    ConfigError, CorsConfig, DatabaseConfig, EmailConfig, Environment, JwtConfig,
    MaintenanceConfig, ObservabilityConfig, OtpConfig, RateLimitConfig, RedisConfig, ServerConfig,
};

use crate::admin_links::{load_admin_links_config, AdminLinksConfig};

#[derive(Debug, Clone)]
pub struct Config {
    pub env: Environment,
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub jwt: JwtConfig,
    pub otp: OtpConfig,
    pub email: EmailConfig,
    pub africas_talking: Option<AfricaTalkingConfig>,
    pub cors: CorsConfig,
    pub rate_limit: RateLimitConfig,
    pub admin_links: AdminLinksConfig,
    pub observability: ObservabilityConfig,
    pub maintenance: MaintenanceConfig,
    pub enforce_https: bool,
    pub trust_proxy_headers: bool,
    /// Shared secret every service presents (`X-Internal-Token`) when
    /// calling another service's `/internal/*` routes — see
    /// `docs/STATUS.md`'s microservices entry for why a shared secret is
    /// enough here (the gateway never routes `/internal/*` to the public
    /// internet at all; this is defense in depth on top of that, not the
    /// only layer).
    pub internal_token: String,
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        let env = Environment::from_env()?;
        Ok(Self {
            env,
            server: ServerConfig::from_env(8081)?,
            database: DatabaseConfig::from_env()?,
            redis: RedisConfig::from_env()?,
            jwt: JwtConfig::from_env(env)?,
            otp: OtpConfig::from_env(env)?,
            email: EmailConfig::from_env()?,
            africas_talking: AfricaTalkingConfig::from_env()?,
            cors: CorsConfig::from_env(env)?,
            rate_limit: RateLimitConfig::from_env(env)?,
            admin_links: load_admin_links_config(),
            observability: ObservabilityConfig::from_env(env, "stocklink-identity")?,
            maintenance: MaintenanceConfig::from_env()?,
            enforce_https: enforce_https_from_env(env)?,
            trust_proxy_headers: trust_proxy_headers_from_env()?,
            internal_token: required("INTERNAL_SERVICE_TOKEN")?,
        })
    }
}
