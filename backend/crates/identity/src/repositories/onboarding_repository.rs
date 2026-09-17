//! Data access for warehouse/store/carrier profiles.

use uuid::Uuid;

use crate::models::onboarding::{Carrier, Store, Warehouse};
use stocklink_shared::database::DbPool;
use stocklink_shared::errors::AppResult;

#[derive(Clone)]
pub struct OnboardingRepository {
    db: DbPool,
}

impl OnboardingRepository {
    pub fn new(db: DbPool) -> Self {
        Self { db }
    }

    pub async fn create_warehouse(
        &self,
        account_id: Uuid,
        name: &str,
        region: &str,
        address: Option<&str>,
    ) -> AppResult<Warehouse> {
        let row = sqlx::query_as::<_, Warehouse>(
            "INSERT INTO warehouses (account_id, name, region, address) VALUES ($1, $2, $3, $4) RETURNING *",
        )
        .bind(account_id)
        .bind(name)
        .bind(region)
        .bind(address)
        .fetch_one(self.db.write())
        .await?;
        Ok(row)
    }

    pub async fn list_warehouses_for_account(&self, account_id: Uuid) -> AppResult<Vec<Warehouse>> {
        let rows = sqlx::query_as::<_, Warehouse>(
            "SELECT * FROM warehouses WHERE account_id = $1 ORDER BY created_at",
        )
        .bind(account_id)
        .fetch_all(self.db.read())
        .await?;
        Ok(rows)
    }

    pub async fn find_warehouse(&self, id: Uuid) -> AppResult<Option<Warehouse>> {
        let row = sqlx::query_as::<_, Warehouse>("SELECT * FROM warehouses WHERE id = $1")
            .bind(id)
            .fetch_optional(self.db.read())
            .await?;
        Ok(row)
    }

    pub async fn list_warehouses(&self, region: Option<&str>) -> AppResult<Vec<Warehouse>> {
        let rows = match region {
            Some(region) => {
                sqlx::query_as::<_, Warehouse>(
                    "SELECT * FROM warehouses WHERE region = $1 ORDER BY name",
                )
                .bind(region)
                .fetch_all(self.db.read())
                .await?
            }
            None => {
                sqlx::query_as::<_, Warehouse>("SELECT * FROM warehouses ORDER BY name")
                    .fetch_all(self.db.read())
                    .await?
            }
        };
        Ok(rows)
    }

    pub async fn create_store(
        &self,
        account_id: Uuid,
        name: &str,
        region: &str,
        address: Option<&str>,
    ) -> AppResult<Store> {
        let row = sqlx::query_as::<_, Store>(
            "INSERT INTO stores (account_id, name, region, address) VALUES ($1, $2, $3, $4) RETURNING *",
        )
        .bind(account_id)
        .bind(name)
        .bind(region)
        .bind(address)
        .fetch_one(self.db.write())
        .await?;
        Ok(row)
    }

    pub async fn list_stores_for_account(&self, account_id: Uuid) -> AppResult<Vec<Store>> {
        let rows = sqlx::query_as::<_, Store>(
            "SELECT * FROM stores WHERE account_id = $1 ORDER BY created_at",
        )
        .bind(account_id)
        .fetch_all(self.db.read())
        .await?;
        Ok(rows)
    }

    pub async fn find_store(&self, id: Uuid) -> AppResult<Option<Store>> {
        let row = sqlx::query_as::<_, Store>("SELECT * FROM stores WHERE id = $1")
            .bind(id)
            .fetch_optional(self.db.read())
            .await?;
        Ok(row)
    }

    pub async fn create_carrier(&self, account_id: Uuid, name: &str) -> AppResult<Carrier> {
        let row = sqlx::query_as::<_, Carrier>(
            "INSERT INTO carriers (account_id, name) VALUES ($1, $2) RETURNING *",
        )
        .bind(account_id)
        .bind(name)
        .fetch_one(self.db.write())
        .await?;
        Ok(row)
    }

    pub async fn list_carriers_for_account(&self, account_id: Uuid) -> AppResult<Vec<Carrier>> {
        let rows = sqlx::query_as::<_, Carrier>(
            "SELECT * FROM carriers WHERE account_id = $1 ORDER BY created_at",
        )
        .bind(account_id)
        .fetch_all(self.db.read())
        .await?;
        Ok(rows)
    }
}
