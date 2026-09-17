//! Registers a warehouse/store/carrier profile for an account and grants the
//! matching role in the same operation — an account gains the `warehouse`
//! role by creating a warehouse, not through a separate admin action.

use uuid::Uuid;

use crate::models::onboarding::{Carrier, Store, Warehouse};
use crate::repositories::auth_repository::AuthRepository;
use crate::repositories::onboarding_repository::OnboardingRepository;
use stocklink_shared::auth::claims::{ROLE_CARRIER, ROLE_STORE, ROLE_WAREHOUSE};
use stocklink_shared::errors::{AppError, AppResult};

#[derive(Clone)]
pub struct OnboardingService {
    repo: OnboardingRepository,
    auth_repo: AuthRepository,
}

impl OnboardingService {
    pub fn new(repo: OnboardingRepository, auth_repo: AuthRepository) -> Self {
        Self { repo, auth_repo }
    }

    pub async fn register_warehouse(
        &self,
        account_id: Uuid,
        name: &str,
        region: &str,
        address: Option<&str>,
    ) -> AppResult<Warehouse> {
        let warehouse = self
            .repo
            .create_warehouse(account_id, name, region, address)
            .await?;
        self.auth_repo.add_role(account_id, ROLE_WAREHOUSE).await?;
        Ok(warehouse)
    }

    pub async fn register_store(
        &self,
        account_id: Uuid,
        name: &str,
        region: &str,
        address: Option<&str>,
    ) -> AppResult<Store> {
        let store = self
            .repo
            .create_store(account_id, name, region, address)
            .await?;
        self.auth_repo.add_role(account_id, ROLE_STORE).await?;
        Ok(store)
    }

    pub async fn register_carrier(&self, account_id: Uuid, name: &str) -> AppResult<Carrier> {
        let carrier = self.repo.create_carrier(account_id, name).await?;
        self.auth_repo.add_role(account_id, ROLE_CARRIER).await?;
        Ok(carrier)
    }

    pub async fn list_warehouses_for_account(&self, account_id: Uuid) -> AppResult<Vec<Warehouse>> {
        self.repo.list_warehouses_for_account(account_id).await
    }

    pub async fn list_stores_for_account(&self, account_id: Uuid) -> AppResult<Vec<Store>> {
        self.repo.list_stores_for_account(account_id).await
    }

    pub async fn list_carriers_for_account(&self, account_id: Uuid) -> AppResult<Vec<Carrier>> {
        self.repo.list_carriers_for_account(account_id).await
    }

    /// Ownership check: `NotFound` (not `Forbidden`) so a warehouse's
    /// existence isn't leaked to accounts that don't own it — same pattern
    /// as `OrderService::get_for_store`.
    pub async fn assert_owns_warehouse(
        &self,
        account_id: Uuid,
        warehouse_id: Uuid,
    ) -> AppResult<Warehouse> {
        let warehouse = self
            .repo
            .find_warehouse(warehouse_id)
            .await?
            .ok_or(AppError::NotFound("warehouse"))?;
        if warehouse.account_id != account_id {
            return Err(AppError::NotFound("warehouse"));
        }
        Ok(warehouse)
    }

    /// The caller's one store — StockLink's MVP scope is one store per
    /// account (see `docs/VISION.md`); an account with zero or more than one
    /// store is a state a future multi-store feature would need to resolve
    /// explicitly (e.g. a `store_id` selector), not silently guessed here.
    pub async fn resolve_single_store(&self, account_id: Uuid) -> AppResult<Store> {
        let mut stores = self.repo.list_stores_for_account(account_id).await?;
        match stores.len() {
            1 => Ok(stores.remove(0)),
            0 => Err(AppError::Validation {
                field: "account".into(),
                message: "no store registered for this account — register one first".into(),
            }),
            _ => Err(AppError::NotImplemented("multiple stores per account")),
        }
    }

    pub async fn get_warehouse(&self, warehouse_id: Uuid) -> AppResult<Warehouse> {
        self.repo
            .find_warehouse(warehouse_id)
            .await?
            .ok_or(AppError::NotFound("warehouse"))
    }

    pub async fn get_store(&self, store_id: Uuid) -> AppResult<Store> {
        self.repo
            .find_store(store_id)
            .await?
            .ok_or(AppError::NotFound("store"))
    }

    pub async fn assert_owns_store(&self, account_id: Uuid, store_id: Uuid) -> AppResult<Store> {
        let store = self
            .repo
            .find_store(store_id)
            .await?
            .ok_or(AppError::NotFound("store"))?;
        if store.account_id != account_id {
            return Err(AppError::NotFound("store"));
        }
        Ok(store)
    }
}
