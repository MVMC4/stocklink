//! Notifications service configuration.

use stocklink_shared::config::{
    enforce_https_from_env, required, trust_proxy_headers_from_env, ConfigError, CorsConfig,
    DatabaseConfig, Environment, FcmConfig, JwtConfig, ObservabilityConfig, RateLimitConfig,
    RedisConfig, ServerConfig,
};

#[derive(Debug, Clone)]
pub struct Config {
    pub env: Environment,
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub jwt: JwtConfig,
    pub fcm: Option<FcmConfig>,
    pub cors: CorsConfig,
    pub rate_limit: RateLimitConfig,
    pub observability: ObservabilityConfig,
    pub enforce_https: bool,
    pub trust_proxy_headers: bool,
    pub internal_token: String,
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        let env = Environment::from_env()?;
        Ok(Self {
            env,
            server: ServerConfig::from_env(8083)?,
            database: DatabaseConfig::from_env()?,
            redis: RedisConfig::from_env()?,
            jwt: JwtConfig::from_env(env)?,
            fcm: FcmConfig::from_env()?,
            cors: CorsConfig::from_env(env)?,
            rate_limit: RateLimitConfig::from_env(env)?,
            observability: ObservabilityConfig::from_env(env, "stocklink-notifications")?,
            enforce_https: enforce_https_from_env(env)?,
            trust_proxy_headers: trust_proxy_headers_from_env()?,
            internal_token: required("INTERNAL_SERVICE_TOKEN")?,
        })
    }
}
