//! Media upload: signs a presigned PUT (S3 backend) or accepts bytes directly
//! (local backend, development only), then records the asset.

use uuid::Uuid;

use crate::models::media::MediaAsset;
use crate::public_media::{PublicMediaConfig, PublicMediaStore};
use crate::repositories::MediaRepository;
use crate::s3_sigv4;
use stocklink_shared::errors::{AppError, AppResult};

#[derive(Clone)]
pub struct MediaService {
    repo: MediaRepository,
    config: Option<PublicMediaConfig>,
}

impl MediaService {
    pub fn new(repo: MediaRepository, config: Option<PublicMediaConfig>) -> Self {
        Self { repo, config }
    }

    /// Presign a PUT for the caller to upload directly to S3 (never through
    /// this API — media bytes don't touch the app process). Returns the
    /// presigned URL and the public URL the asset will be reachable at once
    /// uploaded.
    pub async fn presign_upload(
        &self,
        account_id: Uuid,
        kind: &str,
        content_type: &str,
    ) -> AppResult<(String, MediaAsset)> {
        let config = self
            .config
            .as_ref()
            .ok_or(AppError::ServiceUnavailable("media upload"))?;
        let object_key = format!("{kind}/{account_id}/{}", Uuid::new_v4());

        let (presigned_url, public_url) = match &config.store {
            PublicMediaStore::S3(storage) => {
                let presigned = s3_sigv4::presign_path_style(storage, "PUT", &object_key, None)?;
                (presigned, format!("{}/{}", config.base_url, object_key))
            }
            PublicMediaStore::Local { .. } => {
                // Local dev: no presigning concept — the caller PUTs to this
                // API's own `store_local` route instead (below).
                let url = format!("{}/{}", config.base_url, object_key);
                (url.clone(), url)
            }
        };

        let asset = self
            .repo
            .create(account_id, kind, &object_key, &public_url, content_type, 0)
            .await?;
        Ok((presigned_url, asset))
    }

    pub async fn get(&self, id: Uuid) -> AppResult<MediaAsset> {
        self.repo
            .find(id)
            .await?
            .ok_or(AppError::NotFound("media asset"))
    }

    /// Local dev backend only: receives the bytes `presign_upload` promised
    /// a `local` caller would PUT here itself (see that method's doc comment)
    /// and writes them under `PUBLIC_MEDIA_LOCAL_DIR`. `object_key` is the
    /// wildcard tail of the request path, so it's untrusted input — checked
    /// against `account_id` (the `kind/account_id/uuid` shape `presign_upload`
    /// generates) before touching the filesystem, both so one account can't
    /// overwrite another's upload and so the key can never escape the
    /// configured directory via `..` segments.
    pub async fn store_local(
        &self,
        account_id: Uuid,
        object_key: &str,
        bytes: &[u8],
    ) -> AppResult<()> {
        let dir = self.local_dir()?;
        self.check_object_key(account_id, object_key)?;

        let max = self
            .config
            .as_ref()
            .map(|c| c.max_upload_bytes)
            .unwrap_or(0);
        if bytes.len() > max {
            return Err(AppError::Validation {
                field: "file".into(),
                message: format!("must be at most {max} bytes"),
            });
        }

        let path = dir.join(object_key);
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|err| AppError::Internal(anyhow::Error::new(err).context("create media dir")))?;
        }
        tokio::fs::write(&path, bytes)
            .await
            .map_err(|err| AppError::Internal(anyhow::Error::new(err).context("write media file")))?;

        if let Some(asset) = self.repo.find_by_object_key(object_key).await? {
            self.repo
                .update_byte_size(asset.id, bytes.len() as i64)
                .await?;
        }
        Ok(())
    }

    /// Local dev backend only: serves back what `store_local` wrote,
    /// alongside the content type recorded at presign time.
    pub async fn load_local(&self, object_key: &str) -> AppResult<(Vec<u8>, String)> {
        let dir = self.local_dir()?;
        let asset = self
            .repo
            .find_by_object_key(object_key)
            .await?
            .ok_or(AppError::NotFound("media asset"))?;
        let path = dir.join(object_key);
        let bytes = tokio::fs::read(&path)
            .await
            .map_err(|_| AppError::NotFound("media asset"))?;
        Ok((bytes, asset.content_type))
    }

    fn local_dir(&self) -> AppResult<&std::path::Path> {
        match self.config.as_ref().map(|c| &c.store) {
            Some(PublicMediaStore::Local { dir }) => Ok(dir.as_path()),
            _ => Err(AppError::ServiceUnavailable("media upload")),
        }
    }

    /// `object_key` must be exactly `{kind}/{account_id}/{uuid}` — the shape
    /// `presign_upload` generates — so a caller can only ever write inside
    /// their own account's prefix, never another account's, and never
    /// outside the media directory via a path-traversal segment.
    fn check_object_key(&self, account_id: Uuid, object_key: &str) -> AppResult<()> {
        let parts: Vec<&str> = object_key.split('/').collect();
        let valid = parts.len() == 3
            && !parts.iter().any(|p| p.is_empty() || *p == "." || *p == "..")
            && parts[1] == account_id.to_string().as_str()
            && Uuid::parse_str(parts[2]).is_ok();
        if valid {
            Ok(())
        } else {
            Err(AppError::BadRequest("invalid media object key".into()))
        }
    }
}
