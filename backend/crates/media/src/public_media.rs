//! Public marketplace media storage configuration (contract 10).
//!
//! `PUBLIC_MEDIA_BACKEND` unset means uploads answer 503; listings can still
//! use bundled images. `s3` publishes to an S3-compatible bucket whose objects
//! are publicly readable (bucket policy or CDN). `local` writes to a directory
//! the API serves itself and is refused outside development.

use std::path::PathBuf;
use std::time::Duration;

use stocklink_shared::config::{non_empty_env, parse, ConfigError, Environment, StorageConfig};

#[derive(Debug, Clone)]
pub enum PublicMediaStore {
    S3(StorageConfig),
    Local { dir: PathBuf },
}

#[derive(Debug, Clone)]
pub struct PublicMediaConfig {
    pub store: PublicMediaStore,
    /// Prefix every published URL starts with, without a trailing slash.
    pub base_url: String,
    pub max_upload_bytes: usize,
    pub max_pixels: u64,
    pub max_edge_px: u32,
    pub jpeg_quality: u8,
}

/// The local-disk store has no CDN, no redundancy and no access policy of its
/// own, so only development may use it.
pub fn local_backend_allowed(env: Environment) -> bool {
    env == Environment::Development
}

pub fn load_public_media_config(
    env: Environment,
) -> Result<Option<PublicMediaConfig>, ConfigError> {
    let Some(backend) = non_empty_env("PUBLIC_MEDIA_BACKEND") else {
        return Ok(None);
    };
    let required = |key: &'static str| non_empty_env(key).ok_or(ConfigError::Missing(key));
    let base_url = required("PUBLIC_MEDIA_BASE_URL")?
        .trim_end_matches('/')
        .to_string();

    let store = match backend.as_str() {
        "s3" => {
            if !base_url.starts_with("https://") {
                return Err(ConfigError::Invalid {
                    key: "PUBLIC_MEDIA_BASE_URL",
                    message: "must be an https URL when PUBLIC_MEDIA_BACKEND=s3".into(),
                });
            }
            let endpoint_url = required("PUBLIC_MEDIA_ENDPOINT_URL")?;
            if !endpoint_url.starts_with("https://") && !endpoint_url.starts_with("http://") {
                return Err(ConfigError::Invalid {
                    key: "PUBLIC_MEDIA_ENDPOINT_URL",
                    message: "must be an http or https URL".into(),
                });
            }
            PublicMediaStore::S3(StorageConfig {
                endpoint_url,
                bucket: required("PUBLIC_MEDIA_BUCKET")?,
                region: required("PUBLIC_MEDIA_REGION")?,
                access_key_id: required("PUBLIC_MEDIA_ACCESS_KEY_ID")?,
                secret_access_key: required("PUBLIC_MEDIA_SECRET_ACCESS_KEY")?,
                // The API signs and sends each PUT/DELETE immediately.
                presign_ttl: Duration::from_secs(300),
            })
        }
        "local" => {
            if !local_backend_allowed(env) {
                return Err(ConfigError::Invalid {
                    key: "PUBLIC_MEDIA_BACKEND",
                    message: "`local` is for development only; use `s3`".into(),
                });
            }
            PublicMediaStore::Local {
                dir: PathBuf::from(required("PUBLIC_MEDIA_LOCAL_DIR")?),
            }
        }
        _ => {
            return Err(ConfigError::Invalid {
                key: "PUBLIC_MEDIA_BACKEND",
                message: "must be `s3` or `local`".into(),
            })
        }
    };

    let max_upload_bytes: usize = parse("PUBLIC_MEDIA_MAX_UPLOAD_BYTES", 10 * 1024 * 1024)?;
    if max_upload_bytes == 0 || max_upload_bytes > 25 * 1024 * 1024 {
        return Err(ConfigError::Invalid {
            key: "PUBLIC_MEDIA_MAX_UPLOAD_BYTES",
            message: "must be between 1 byte and 25 MiB".into(),
        });
    }
    let max_pixels: u64 = parse("PUBLIC_MEDIA_MAX_PIXELS", 40_000_000)?;
    if max_pixels == 0 {
        return Err(ConfigError::Invalid {
            key: "PUBLIC_MEDIA_MAX_PIXELS",
            message: "must be greater than zero".into(),
        });
    }
    let max_edge_px: u32 = parse("PUBLIC_MEDIA_MAX_EDGE_PX", 1600)?;
    if !(320..=4096).contains(&max_edge_px) {
        return Err(ConfigError::Invalid {
            key: "PUBLIC_MEDIA_MAX_EDGE_PX",
            message: "must be between 320 and 4096".into(),
        });
    }
    let jpeg_quality: u8 = parse("PUBLIC_MEDIA_JPEG_QUALITY", 82)?;
    if !(50..=95).contains(&jpeg_quality) {
        return Err(ConfigError::Invalid {
            key: "PUBLIC_MEDIA_JPEG_QUALITY",
            message: "must be between 50 and 95".into(),
        });
    }

    Ok(Some(PublicMediaConfig {
        store,
        base_url,
        max_upload_bytes,
        max_pixels,
        max_edge_px,
        jpeg_quality,
    }))
}

#[cfg(test)]
mod tests {
    use super::{local_backend_allowed, Environment};

    #[test]
    fn local_disk_media_is_development_only() {
        assert!(local_backend_allowed(Environment::Development));
        assert!(!local_backend_allowed(Environment::Staging));
        assert!(!local_backend_allowed(Environment::Production));
        assert!(!local_backend_allowed(Environment::Demo));
    }
}
