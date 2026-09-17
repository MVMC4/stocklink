//! OTP challenges, refresh-token sessions, accounts/contacts, and role
//! grants — everything `AuthService` and `OnboardingService` need to read
//! or write against the `accounts`/`otp_challenges`/`sessions` tables (see
//! `migrations/0001_baseline.sql`).

use chrono::{DateTime, Utc};
use uuid::Uuid;

use stocklink_shared::database::DbPool;
use stocklink_shared::errors::AppResult;

use crate::models::account::Account;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct OtpChallenge {
    pub id: Uuid,
    pub code_hash: String,
    pub attempts: i32,
    pub max_attempts: i32,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Session {
    pub id: Uuid,
    pub account_id: Uuid,
    pub secret_hash: String,
    pub revoked_at: Option<DateTime<Utc>>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Clone)]
pub struct AuthRepository {
    db: DbPool,
}

impl AuthRepository {
    pub fn new(db: DbPool) -> Self {
        Self { db }
    }

    pub async fn create_otp_challenge(
        &self,
        channel: &str,
        identifier: &str,
        code_hash: &str,
        ttl_secs: i64,
        max_attempts: i32,
    ) -> AppResult<()> {
        sqlx::query(
            "INSERT INTO otp_challenges (channel, identifier, code_hash, max_attempts, expires_at) \
             VALUES ($1, $2, $3, $4, now() + make_interval(secs => $5::double precision))",
        )
        .bind(channel)
        .bind(identifier)
        .bind(code_hash)
        .bind(max_attempts)
        .bind(ttl_secs as f64)
        .execute(self.db.write())
        .await?;
        Ok(())
    }

    /// The most recent still-live (unconsumed, unexpired) challenge for this
    /// contact — a new `request_otp` call supersedes an earlier unused code
    /// implicitly, since only the latest one can ever be found here.
    pub async fn find_active_otp_challenge(
        &self,
        channel: &str,
        identifier: &str,
    ) -> AppResult<Option<OtpChallenge>> {
        let challenge = sqlx::query_as::<_, OtpChallenge>(
            "SELECT id, code_hash, attempts, max_attempts FROM otp_challenges \
             WHERE channel = $1 AND identifier = $2 \
               AND consumed_at IS NULL AND expires_at > now() \
             ORDER BY created_at DESC LIMIT 1",
        )
        .bind(channel)
        .bind(identifier)
        .fetch_optional(self.db.read())
        .await?;
        Ok(challenge)
    }

    pub async fn increment_otp_attempts(&self, id: Uuid) -> AppResult<()> {
        sqlx::query("UPDATE otp_challenges SET attempts = attempts + 1 WHERE id = $1")
            .bind(id)
            .execute(self.db.write())
            .await?;
        Ok(())
    }

    pub async fn consume_otp_challenge(&self, id: Uuid) -> AppResult<()> {
        sqlx::query("UPDATE otp_challenges SET consumed_at = now() WHERE id = $1")
            .bind(id)
            .execute(self.db.write())
            .await?;
        Ok(())
    }

    pub async fn find_account_by_contact(
        &self,
        channel: &str,
        identifier: &str,
    ) -> AppResult<Option<Account>> {
        let account = sqlx::query_as::<_, Account>(
            "SELECT a.* FROM accounts a \
             JOIN account_contacts c ON c.account_id = a.id \
             WHERE c.channel = $1 AND c.identifier = $2 AND a.deleted_at IS NULL",
        )
        .bind(channel)
        .bind(identifier)
        .fetch_optional(self.db.read())
        .await?;
        Ok(account)
    }

    /// Creates the account and its first (primary) contact together. The
    /// contact is inserted already verified — reaching this point means the
    /// caller just proved control of it via a correct OTP.
    pub async fn create_account_with_contact(
        &self,
        channel: &str,
        identifier: &str,
        display_name: &str,
    ) -> AppResult<Account> {
        let mut tx: sqlx::Transaction<'_, sqlx::Postgres> = self.db.write().begin().await?;

        let account = sqlx::query_as::<_, Account>(
            "INSERT INTO accounts (display_name) VALUES ($1) RETURNING *",
        )
        .bind(display_name)
        .fetch_one(&mut *tx)
        .await?;

        let contact_id: Uuid = sqlx::query_scalar(
            "INSERT INTO account_contacts (account_id, channel, identifier, verified_at, is_primary) \
             VALUES ($1, $2, $3, now(), true) RETURNING id",
        )
        .bind(account.id)
        .bind(channel)
        .bind(identifier)
        .fetch_one(&mut *tx)
        .await?;

        sqlx::query("UPDATE accounts SET primary_contact_id = $1 WHERE id = $2")
            .bind(contact_id)
            .bind(account.id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
        Ok(account)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn create_session(
        &self,
        session_id: Uuid,
        account_id: Uuid,
        secret_hash: &str,
        ttl_secs: i64,
        user_agent: Option<&str>,
        ip: Option<&str>,
    ) -> AppResult<()> {
        sqlx::query(
            "INSERT INTO sessions (id, account_id, secret_hash, user_agent, ip, expires_at) \
             VALUES ($1, $2, $3, $4, $5, now() + make_interval(secs => $6::double precision))",
        )
        .bind(session_id)
        .bind(account_id)
        .bind(secret_hash)
        .bind(user_agent)
        .bind(ip)
        .bind(ttl_secs as f64)
        .execute(self.db.write())
        .await?;
        Ok(())
    }

    pub async fn find_session(&self, id: Uuid) -> AppResult<Option<Session>> {
        let session = sqlx::query_as::<_, Session>(
            "SELECT id, account_id, secret_hash, revoked_at, expires_at FROM sessions WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(self.db.read())
        .await?;
        Ok(session)
    }

    pub async fn revoke_session(&self, id: Uuid) -> AppResult<()> {
        sqlx::query("UPDATE sessions SET revoked_at = now() WHERE id = $1 AND revoked_at IS NULL")
            .bind(id)
            .execute(self.db.write())
            .await?;
        Ok(())
    }

    pub async fn list_roles(&self, account_id: Uuid) -> AppResult<Vec<String>> {
        let roles: Vec<(String,)> =
            sqlx::query_as("SELECT role FROM account_roles WHERE account_id = $1")
                .bind(account_id)
                .fetch_all(self.db.read())
                .await?;
        Ok(roles.into_iter().map(|(role,)| role).collect())
    }

    pub async fn add_role(&self, account_id: Uuid, role: &str) -> AppResult<()> {
        sqlx::query(
            "INSERT INTO account_roles (account_id, role) VALUES ($1, $2) \
             ON CONFLICT (account_id, role) DO NOTHING",
        )
        .bind(account_id)
        .bind(role)
        .execute(self.db.write())
        .await?;
        Ok(())
    }
}
