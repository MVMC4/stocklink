//! Data access for uploaded media assets (catalogue photos, proof of delivery).

use uuid::Uuid;

use crate::models::media::MediaAsset;
use stocklink_shared::database::DbPool;
use stocklink_shared::errors::AppResult;

#[derive(Clone)]
pub struct MediaRepository {
    db: DbPool,
}

impl MediaRepository {
    pub fn new(db: DbPool) -> Self {
        Self { db }
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn create(
        &self,
        account_id: Uuid,
        kind: &str,
        object_key: &str,
        url: &str,
        content_type: &str,
        byte_size: i64,
    ) -> AppResult<MediaAsset> {
        let row = sqlx::query_as::<_, MediaAsset>(
            "INSERT INTO media_assets (account_id, kind, object_key, url, content_type, byte_size) \
             VALUES ($1,$2,$3,$4,$5,$6) RETURNING *",
        )
        .bind(account_id)
        .bind(kind)
        .bind(object_key)
        .bind(url)
        .bind(content_type)
        .bind(byte_size)
        .fetch_one(self.db.write())
        .await?;
        Ok(row)
    }

    pub async fn find(&self, id: Uuid) -> AppResult<Option<MediaAsset>> {
        let row = sqlx::query_as::<_, MediaAsset>("SELECT * FROM media_assets WHERE id = $1")
            .bind(id)
            .fetch_optional(self.db.read())
            .await?;
        Ok(row)
    }
}
