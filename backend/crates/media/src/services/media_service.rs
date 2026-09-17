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
                // API's own upload route instead (not modelled further here).
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
}
