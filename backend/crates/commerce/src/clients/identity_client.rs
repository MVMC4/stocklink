//! HTTP client for identity's `/internal/*` API. Commerce only ever holds
//! `warehouse_id`/`store_id`/`account_id` as opaque UUIDs (see the migration
//! file's header comment) — every question about who owns what, or what a
//! warehouse/store is even called, goes through here instead of a database
//! join, because that data lives in identity's own database now.

use serde::Deserialize;
use uuid::Uuid;

use stocklink_shared::errors::{AppError, AppResult};

#[derive(Debug, Clone)]
pub struct IdentityClient {
    http: reqwest::Client,
    base_url: String,
    internal_token: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WarehouseInfo {
    pub id: Uuid,
    pub account_id: Uuid,
    pub name: String,
    pub region: String,
    pub address: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StoreInfo {
    pub id: Uuid,
    pub account_id: Uuid,
    pub name: String,
    pub region: String,
}

/// Mirrors `stocklink_shared::errors::ErrorBody`'s wire shape — just enough
/// to lift the human-readable message out of a 4xx response.
#[derive(Debug, Deserialize)]
struct IdentityErrorEnvelope {
    error: IdentityErrorDetail,
}

#[derive(Debug, Deserialize)]
struct IdentityErrorDetail {
    message: String,
}

impl IdentityClient {
    pub fn new(base_url: String, internal_token: String) -> Self {
        Self {
            http: reqwest::Client::new(),
            base_url,
            internal_token,
        }
    }

    fn request(&self, path: &str) -> reqwest::RequestBuilder {
        self.http
            .get(format!("{}{path}", self.base_url))
            .header("X-Internal-Token", &self.internal_token)
    }

    async fn get_json<T: for<'de> Deserialize<'de>>(
        &self,
        path: &str,
        not_found: &'static str,
    ) -> AppResult<T> {
        let response = self.request(path).send().await.map_err(|err| {
            AppError::Internal(anyhow::Error::new(err).context("identity service call"))
        })?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(AppError::NotFound(not_found));
        }
        // A 4xx here means identity rejected the request as invalid (e.g.
        // "no store registered for this account") — that's the caller's
        // fault, not identity being down, so it must not surface as a 503.
        // Only a 5xx (or the network-level failure above) is really
        // "identity service unavailable".
        if response.status().is_client_error() {
            let message = response
                .json::<IdentityErrorEnvelope>()
                .await
                .ok()
                .map(|e| e.error.message)
                .unwrap_or_else(|| "request rejected by identity service".to_string());
            return Err(AppError::BadRequest(message));
        }
        if !response.status().is_success() {
            return Err(AppError::ServiceUnavailable("identity service"));
        }
        response.json::<T>().await.map_err(|err| {
            AppError::Internal(anyhow::Error::new(err).context("identity service response"))
        })
    }

    pub async fn get_warehouse(&self, id: Uuid) -> AppResult<WarehouseInfo> {
        self.get_json(&format!("/internal/warehouses/{id}"), "warehouse")
            .await
    }

    pub async fn get_store(&self, id: Uuid) -> AppResult<StoreInfo> {
        self.get_json(&format!("/internal/stores/{id}"), "store")
            .await
    }

    pub async fn resolve_single_store(&self, account_id: Uuid) -> AppResult<StoreInfo> {
        self.get_json(
            &format!("/internal/stores/resolve-single?account_id={account_id}"),
            "store",
        )
        .await
    }

    pub async fn assert_owns_warehouse(
        &self,
        account_id: Uuid,
        warehouse_id: Uuid,
    ) -> AppResult<WarehouseInfo> {
        let owns: bool = self
            .get_json(
                &format!("/internal/warehouses/{warehouse_id}/owned-by?account_id={account_id}"),
                "warehouse",
            )
            .await?;
        if !owns {
            return Err(AppError::NotFound("warehouse"));
        }
        self.get_warehouse(warehouse_id).await
    }

    pub async fn assert_owns_store(
        &self,
        account_id: Uuid,
        store_id: Uuid,
    ) -> AppResult<StoreInfo> {
        let owns: bool = self
            .get_json(
                &format!("/internal/stores/{store_id}/owned-by?account_id={account_id}"),
                "store",
            )
            .await?;
        if !owns {
            return Err(AppError::NotFound("store"));
        }
        self.get_store(store_id).await
    }
}
