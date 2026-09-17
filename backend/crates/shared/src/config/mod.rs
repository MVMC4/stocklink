//! Config primitives shared by every service: the environment loader
//! helpers, `ConfigError`, and every reusable sub-config struct. Each
//! service defines its own top-level `Config` (composing whichever of these
//! it needs) and its own `Config::from_env()` — there is no single
//! service-spanning `Config` type, because no two services need the same
//! set of fields.
//!
//! Every production-safety rule below is a small, independently
//! unit-tested pure function — so each rule has a red test (production +
//! the unsafe value → `Err`; production + the safe value, and development +
//! the "unsafe" value → `Ok`) without touching real process environment
//! variables (which would race across parallel `cargo test` threads).

use std::env;
use std::fmt;
use std::str::FromStr;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Environment {
    Development,
    Demo,
    Staging,
    Production,
}

impl Environment {
    pub fn parse(raw: &str) -> Result<Self, ConfigError> {
        match raw.to_ascii_lowercase().as_str() {
            "development" | "dev" => Ok(Environment::Development),
            "demo" => Ok(Environment::Demo),
            "staging" => Ok(Environment::Staging),
            "production" | "prod" => Ok(Environment::Production),
            _ => Err(ConfigError::Invalid {
                key: "STOCKLINK_ENV",
                message: format!(
                    "must be one of development, demo, staging, production; got `{raw}`"
                ),
            }),
        }
    }

    /// Staging and production get the strict production-safety checks below;
    /// development and demo do not (demo is a public-facing showcase, not a
    /// place real accounts or money move, so it stays permissive like dev).
    pub fn is_production_like(self) -> bool {
        matches!(self, Environment::Staging | Environment::Production)
    }

    /// Reads `STOCKLINK_ENV`, defaulting to `development` when unset.
    pub fn from_env() -> Result<Self, ConfigError> {
        Self::parse(&non_empty_env("STOCKLINK_ENV").unwrap_or_else(|| "development".into()))
    }
}

impl fmt::Display for Environment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Environment::Development => "development",
            Environment::Demo => "demo",
            Environment::Staging => "staging",
            Environment::Production => "production",
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("missing required environment variable `{0}`")]
    Missing(&'static str),
    #[error("invalid value for `{key}`: {message}")]
    Invalid { key: &'static str, message: String },
}

/// `env::var`, trimmed, with a blank value treated as absent — an
/// accidentally-empty `.env` line should behave like the var was never set,
/// not like it was set to `""`.
pub fn non_empty_env(key: &str) -> Option<String> {
    env::var(key)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

/// Parse an optional environment variable, falling back to `default` when
/// absent. `key` is only used in the error message, so a malformed value
/// fails loudly instead of silently falling back.
pub fn parse<T: FromStr>(key: &'static str, default: T) -> Result<T, ConfigError> {
    match non_empty_env(key) {
        None => Ok(default),
        Some(raw) => raw.parse::<T>().map_err(|_| ConfigError::Invalid {
            key,
            message: format!("could not parse `{raw}`"),
        }),
    }
}

pub fn required(key: &'static str) -> Result<String, ConfigError> {
    non_empty_env(key).ok_or(ConfigError::Missing(key))
}

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

impl ServerConfig {
    pub fn from_env(default_port: u16) -> Result<Self, ConfigError> {
        Ok(Self {
            host: non_empty_env("SERVER_HOST").unwrap_or_else(|| "0.0.0.0".into()),
            port: parse("SERVER_PORT", default_port)?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    pub url: String,
    pub read_replica_url: Option<String>,
    pub max_connections: u32,
    pub min_connections: u32,
    pub acquire_timeout: Duration,
}

impl DatabaseConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        Ok(Self {
            url: required("DATABASE_URL")?,
            read_replica_url: non_empty_env("DATABASE_READ_REPLICA_URL"),
            max_connections: parse("DATABASE_MAX_CONNECTIONS", 10u32)?,
            min_connections: parse("DATABASE_MIN_CONNECTIONS", 1u32)?,
            acquire_timeout: Duration::from_secs(parse("DATABASE_ACQUIRE_TIMEOUT_SECS", 10u64)?),
        })
    }
}

#[derive(Debug, Clone)]
pub struct RedisConfig {
    pub url: String,
    pub pool_size: usize,
}

impl RedisConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        Ok(Self {
            url: required("REDIS_URL")?,
            pool_size: parse("REDIS_POOL_SIZE", 10usize)?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct JwtConfig {
    pub access_secret: String,
    pub issuer: String,
    pub audience: String,
    pub access_ttl: Duration,
    pub refresh_ttl: Duration,
}

impl JwtConfig {
    /// Every service loads this the same way — the signing secret is shared
    /// across all of them so any service can verify a token identity issued,
    /// without a network call (see docs/STATUS.md's microservices entry).
    pub fn from_env(env: Environment) -> Result<Self, ConfigError> {
        let access_secret = required("JWT_ACCESS_SECRET")?;
        validate_jwt_secret(env, &access_secret)?;
        Ok(Self {
            access_secret,
            issuer: non_empty_env("JWT_ISSUER").unwrap_or_else(|| "stocklink".into()),
            audience: non_empty_env("JWT_AUDIENCE").unwrap_or_else(|| "stocklink-app".into()),
            access_ttl: Duration::from_secs(parse("JWT_ACCESS_TTL_SECS", 900u64)?),
            refresh_ttl: Duration::from_secs(parse("JWT_REFRESH_TTL_SECS", 2_592_000u64)?),
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub struct OtpConfig {
    pub dev_echo_enabled: bool,
}

impl OtpConfig {
    pub fn from_env(env: Environment) -> Result<Self, ConfigError> {
        let dev_echo_enabled: bool = parse("OTP_DEV_ECHO_ENABLED", false)?;
        validate_otp_dev_echo(env, dev_echo_enabled)?;
        Ok(Self { dev_echo_enabled })
    }
}

#[derive(Debug, Clone)]
pub struct EmailConfig {
    pub api_key: String,
    pub base_url: String,
    pub from_name: String,
    pub from_address: String,
    pub reply_to: Option<String>,
    pub timeout: Duration,
}

impl EmailConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        Ok(Self {
            api_key: required("RESEND_API_KEY")?,
            base_url: non_empty_env("RESEND_BASE_URL")
                .unwrap_or_else(|| "https://api.resend.com".into()),
            from_name: non_empty_env("EMAIL_FROM_NAME").unwrap_or_else(|| "StockLink".into()),
            from_address: required("EMAIL_FROM_ADDRESS")?,
            reply_to: non_empty_env("EMAIL_REPLY_TO"),
            timeout: Duration::from_secs(parse("EMAIL_TIMEOUT_SECS", 8u64)?),
        })
    }
}

/// SMS is a secondary channel (phone OTP is optional for business accounts;
/// email is the required channel — see WO-04). Absent entirely unless
/// `AFRICAS_TALKING_API_KEY` is set.
#[derive(Debug, Clone)]
pub struct AfricaTalkingConfig {
    pub username: String,
    pub api_key: String,
    pub base_url: String,
    pub sender_id: Option<String>,
    pub timeout: Duration,
}

impl AfricaTalkingConfig {
    pub fn from_env() -> Result<Option<Self>, ConfigError> {
        let Some(api_key) = non_empty_env("AFRICAS_TALKING_API_KEY") else {
            return Ok(None);
        };
        Ok(Some(Self {
            username: required("AFRICAS_TALKING_USERNAME")?,
            api_key,
            base_url: non_empty_env("AFRICAS_TALKING_BASE_URL").unwrap_or_else(|| {
                "https://api.sandbox.africastalking.com/version1/messaging".into()
            }),
            sender_id: non_empty_env("AFRICAS_TALKING_SENDER_ID"),
            timeout: Duration::from_secs(parse("AFRICAS_TALKING_TIMEOUT_SECS", 8u64)?),
        }))
    }
}

/// Push notifications are optional; absent entirely unless `FCM_PROJECT_ID`
/// is set.
#[derive(Debug, Clone)]
pub struct FcmConfig {
    pub project_id: String,
    pub client_email: String,
    pub private_key: String,
    pub token_uri: String,
    pub timeout: Duration,
    pub max_retries: u32,
}

impl FcmConfig {
    pub fn from_env() -> Result<Option<Self>, ConfigError> {
        let Some(project_id) = non_empty_env("FCM_PROJECT_ID") else {
            return Ok(None);
        };
        Ok(Some(Self {
            project_id,
            client_email: required("FCM_CLIENT_EMAIL")?,
            private_key: required("FCM_PRIVATE_KEY")?,
            token_uri: non_empty_env("FCM_TOKEN_URI")
                .unwrap_or_else(|| "https://oauth2.googleapis.com/token".into()),
            timeout: Duration::from_secs(parse("FCM_TIMEOUT_SECS", 8u64)?),
            max_retries: parse("FCM_MAX_RETRIES", 2u32)?,
        }))
    }
}

/// S3-compatible object storage credentials, shared by every store that signs
/// presigned URLs (see `media`'s `s3_sigv4` module).
#[derive(Debug, Clone)]
pub struct StorageConfig {
    pub endpoint_url: String,
    pub bucket: String,
    pub region: String,
    pub access_key_id: String,
    pub secret_access_key: String,
    pub presign_ttl: Duration,
}

#[derive(Debug, Clone)]
pub struct CorsConfig {
    pub allowed_origins: Vec<String>,
}

impl CorsConfig {
    pub fn from_env(env: Environment) -> Result<Self, ConfigError> {
        let allowed_origins: Vec<String> = non_empty_env("CORS_ALLOWED_ORIGINS")
            .unwrap_or_default()
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect();
        validate_cors_origins(env, &allowed_origins)?;
        Ok(Self { allowed_origins })
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RateLimitConfig {
    pub enabled: bool,
    pub requests_per_minute: u32,
    pub window_secs: u64,
}

impl RateLimitConfig {
    pub fn from_env(env: Environment) -> Result<Self, ConfigError> {
        let enabled = parse("RATE_LIMIT_ENABLED", true)?;
        validate_rate_limit_enabled(env, enabled)?;
        Ok(Self {
            enabled,
            requests_per_minute: parse("RATE_LIMIT_REQUESTS_PER_MINUTE", 120u32)?,
            window_secs: parse("RATE_LIMIT_WINDOW_SECS", 60u64)?,
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub struct MaintenanceConfig {
    pub enabled: bool,
    pub interval: Duration,
}

impl MaintenanceConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        Ok(Self {
            enabled: parse("MAINTENANCE_ENABLED", true)?,
            interval: Duration::from_secs(parse("MAINTENANCE_INTERVAL_SECS", 300u64)?),
        })
    }
}

#[derive(Debug, Clone)]
pub struct OtelConfig {
    pub enabled: bool,
    pub endpoint: String,
    pub service_name: String,
}

#[derive(Debug, Clone)]
pub struct ObservabilityConfig {
    pub log_filter: String,
    pub json: bool,
    pub otel: OtelConfig,
}

impl ObservabilityConfig {
    pub fn from_env(env: Environment, service_name: &str) -> Result<Self, ConfigError> {
        Ok(Self {
            log_filter: non_empty_env("RUST_LOG").unwrap_or_else(|| "info".into()),
            json: parse("LOG_JSON", env.is_production_like())?,
            otel: OtelConfig {
                enabled: parse("OTEL_ENABLED", false)?,
                endpoint: non_empty_env("OTEL_EXPORTER_OTLP_ENDPOINT")
                    .unwrap_or_else(|| "http://localhost:4317".into()),
                service_name: non_empty_env("OTEL_SERVICE_NAME")
                    .unwrap_or_else(|| service_name.to_string()),
            },
        })
    }
}

/// Whether the HTTPS-enforcement middleware refuses plain HTTP. Always
/// `true` in staging/production (validated here); the gateway terminates
/// real TLS, so this trusts `X-Forwarded-Proto` when `trust_proxy_headers`
/// is also set (see `middleware::enforce_https_mw`).
pub fn enforce_https_from_env(env: Environment) -> Result<bool, ConfigError> {
    let enforce: bool = parse("ENFORCE_HTTPS", env.is_production_like())?;
    validate_enforce_https(env, enforce)?;
    Ok(enforce)
}

/// Whether to trust `X-Forwarded-For`/`X-Forwarded-Proto` from the immediate
/// peer. Only ever `true` behind a gateway that itself overwrites those
/// headers (see `docker/nginx/gateway.conf`) — never when a service is
/// directly internet-facing.
pub fn trust_proxy_headers_from_env() -> Result<bool, ConfigError> {
    parse("TRUST_PROXY_HEADERS", false)
}

// ── production-safety validators (each independently unit-tested below) ────

pub fn validate_jwt_secret(env: Environment, secret: &str) -> Result<(), ConfigError> {
    if secret.len() < 32 {
        return Err(ConfigError::Invalid {
            key: "JWT_ACCESS_SECRET",
            message: "must be at least 32 bytes".into(),
        });
    }
    if env.is_production_like() && secret.contains("dev-only") {
        return Err(ConfigError::Invalid {
            key: "JWT_ACCESS_SECRET",
            message: "refuses to start in staging/production with a development placeholder secret"
                .into(),
        });
    }
    Ok(())
}

pub fn validate_otp_dev_echo(env: Environment, enabled: bool) -> Result<(), ConfigError> {
    if enabled && env.is_production_like() {
        return Err(ConfigError::Invalid {
            key: "OTP_DEV_ECHO_ENABLED",
            message: "must never be enabled in staging or production".into(),
        });
    }
    Ok(())
}

pub fn validate_cors_origins(env: Environment, origins: &[String]) -> Result<(), ConfigError> {
    if origins.iter().any(|o| o == "*") {
        return Err(ConfigError::Invalid {
            key: "CORS_ALLOWED_ORIGINS",
            message: "a wildcard origin is never allowed".into(),
        });
    }
    if env.is_production_like() && origins.is_empty() {
        return Err(ConfigError::Invalid {
            key: "CORS_ALLOWED_ORIGINS",
            message: "must not be empty in staging or production".into(),
        });
    }
    Ok(())
}

pub fn validate_enforce_https(env: Environment, enforce: bool) -> Result<(), ConfigError> {
    if env.is_production_like() && !enforce {
        return Err(ConfigError::Invalid {
            key: "ENFORCE_HTTPS",
            message: "must stay enabled in staging or production".into(),
        });
    }
    Ok(())
}

pub fn validate_rate_limit_enabled(env: Environment, enabled: bool) -> Result<(), ConfigError> {
    if env.is_production_like() && !enabled {
        return Err(ConfigError::Invalid {
            key: "RATE_LIMIT_ENABLED",
            message: "must stay enabled in staging or production".into(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn environment_parses_known_names_case_insensitively() {
        assert_eq!(
            Environment::parse("development").unwrap(),
            Environment::Development
        );
        assert_eq!(Environment::parse("DEV").unwrap(), Environment::Development);
        assert_eq!(Environment::parse("Staging").unwrap(), Environment::Staging);
        assert_eq!(
            Environment::parse("PRODUCTION").unwrap(),
            Environment::Production
        );
        assert!(Environment::parse("nonsense").is_err());
    }

    #[test]
    fn jwt_secret_shorter_than_32_bytes_is_rejected_in_every_environment() {
        assert!(validate_jwt_secret(Environment::Development, "short").is_err());
        assert!(validate_jwt_secret(Environment::Production, "short").is_err());
    }

    #[test]
    fn jwt_dev_placeholder_secret_is_refused_in_production_but_allowed_in_dev() {
        let placeholder = "dev-only-change-me-before-sharing-32-bytes";
        assert!(validate_jwt_secret(Environment::Development, placeholder).is_ok());
        assert!(validate_jwt_secret(Environment::Staging, placeholder).is_err());
        assert!(validate_jwt_secret(Environment::Production, placeholder).is_err());
    }

    #[test]
    fn otp_dev_echo_refuses_to_start_in_production_but_not_development() {
        assert!(validate_otp_dev_echo(Environment::Production, true).is_err());
        assert!(validate_otp_dev_echo(Environment::Staging, true).is_err());
        assert!(validate_otp_dev_echo(Environment::Development, true).is_ok());
        assert!(validate_otp_dev_echo(Environment::Production, false).is_ok());
    }

    #[test]
    fn wildcard_cors_origin_is_rejected_in_every_environment() {
        let wildcard = vec!["*".to_string()];
        assert!(validate_cors_origins(Environment::Development, &wildcard).is_err());
        assert!(validate_cors_origins(Environment::Production, &wildcard).is_err());
    }

    #[test]
    fn empty_cors_origins_are_rejected_in_production_but_allowed_in_dev() {
        assert!(validate_cors_origins(Environment::Development, &[]).is_ok());
        assert!(validate_cors_origins(Environment::Production, &[]).is_err());
        let allowed = vec!["https://app.stocklink.example".to_string()];
        assert!(validate_cors_origins(Environment::Production, &allowed).is_ok());
    }

    #[test]
    fn disabled_https_enforcement_refuses_to_start_in_production() {
        assert!(validate_enforce_https(Environment::Production, false).is_err());
        assert!(validate_enforce_https(Environment::Staging, false).is_err());
        assert!(validate_enforce_https(Environment::Development, false).is_ok());
        assert!(validate_enforce_https(Environment::Production, true).is_ok());
    }

    #[test]
    fn disabled_rate_limiting_refuses_to_start_in_production() {
        assert!(validate_rate_limit_enabled(Environment::Production, false).is_err());
        assert!(validate_rate_limit_enabled(Environment::Development, false).is_ok());
        assert!(validate_rate_limit_enabled(Environment::Production, true).is_ok());
    }
}
