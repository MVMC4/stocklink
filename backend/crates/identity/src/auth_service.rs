//! Email/phone OTP authentication flow: request a code, verify it (creating
//! the account on first verification), refresh, and logout.
//!
//! Email is the required channel (WO-04); phone/SMS is wired the same way
//! but only usable when `AfricaTalkingConfig` is configured (see
//! `services::auth_service` construction in `state.rs`/controllers).

use std::sync::Arc;
use std::time::Duration;

use uuid::Uuid;

use crate::email::gateway::{EmailGateway, EmailMessage};
use crate::otp::{
    generate_code, generate_refresh_token, hash_secret, parse_refresh_token, verify_secret,
    OTP_MAX_ATTEMPTS, OTP_RATE_LIMIT_MAX_REQUESTS, OTP_RATE_LIMIT_WINDOW_SECS, OTP_TTL_SECS,
};
use crate::repositories::auth_repository::AuthRepository;
use crate::sms::gateway::SmsGateway;
use stocklink_shared::auth::{AccessTokenDenylist, JwtKeys};
use stocklink_shared::database::redis::RedisPool;
use stocklink_shared::errors::{AppError, AppResult};
use stocklink_shared::middleware::rate_limit::check_and_increment;

pub struct OtpRequestAck {
    pub expires_in_secs: u64,
    /// Only ever `Some` when `OTP_DEV_ECHO_ENABLED=true`, which config
    /// refuses to allow outside development (see `config::validate_otp_dev_echo`).
    pub dev_code: Option<String>,
}

pub struct TokenPair {
    pub account_id: Uuid,
    pub access_token: String,
    pub refresh_token: String,
    pub roles: Vec<String>,
}

#[derive(Clone)]
pub struct AuthService {
    repo: AuthRepository,
    email: Arc<dyn EmailGateway>,
    sms: Option<Arc<dyn SmsGateway>>,
    jwt: Arc<JwtKeys>,
    denylist: AccessTokenDenylist,
    redis: RedisPool,
    dev_echo_enabled: bool,
    refresh_ttl_secs: i64,
}

impl AuthService {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        repo: AuthRepository,
        email: Arc<dyn EmailGateway>,
        sms: Option<Arc<dyn SmsGateway>>,
        jwt: Arc<JwtKeys>,
        denylist: AccessTokenDenylist,
        redis: RedisPool,
        dev_echo_enabled: bool,
        refresh_ttl: Duration,
    ) -> Self {
        Self {
            repo,
            email,
            sms,
            jwt,
            denylist,
            redis,
            dev_echo_enabled,
            refresh_ttl_secs: refresh_ttl.as_secs() as i64,
        }
    }

    fn validate_channel(channel: &str) -> AppResult<()> {
        match channel {
            "email" => Ok(()),
            "phone" => Ok(()),
            _ => Err(AppError::Validation {
                field: "channel".into(),
                message: "must be `email` or `phone`".into(),
            }),
        }
    }

    pub async fn request_otp(&self, channel: &str, identifier: &str) -> AppResult<OtpRequestAck> {
        Self::validate_channel(channel)?;
        if channel == "phone" && self.sms.is_none() {
            return Err(AppError::ServiceUnavailable("phone verification"));
        }

        let rate_key = format!("otp:request:{channel}:{identifier}");
        let allowed = check_and_increment(
            &self.redis,
            &rate_key,
            OTP_RATE_LIMIT_MAX_REQUESTS,
            OTP_RATE_LIMIT_WINDOW_SECS,
        )
        .await?;
        if !allowed {
            return Err(AppError::RateLimited);
        }

        let code = generate_code();
        let code_hash = hash_secret(&code)?;
        self.repo
            .create_otp_challenge(
                channel,
                identifier,
                &code_hash,
                OTP_TTL_SECS as i64,
                OTP_MAX_ATTEMPTS as i32,
            )
            .await?;

        match channel {
            "email" => {
                self.email
                    .send(EmailMessage {
                        to: identifier,
                        subject: "Your StockLink verification code",
                        text_body: &format!(
                            "Your StockLink verification code is {code}. It expires in 5 minutes."
                        ),
                        html_body: None,
                    })
                    .await
                    .map_err(|_| AppError::ServiceUnavailable("email delivery"))?;
            }
            "phone" => {
                // Best-effort secondary channel: log and continue rather than
                // fail the whole request — the code is already persisted, and
                // dev/test can read it back via `dev_code`.
                if let Some(sms) = &self.sms {
                    if let Err(err) = sms
                        .send(
                            identifier,
                            &format!("Your StockLink verification code is {code}."),
                        )
                        .await
                    {
                        tracing::warn!(error = ?err, "otp sms delivery failed");
                    }
                }
            }
            _ => unreachable!("validated above"),
        }

        Ok(OtpRequestAck {
            expires_in_secs: OTP_TTL_SECS,
            dev_code: self.dev_echo_enabled.then_some(code),
        })
    }

    /// Verify a code, creating the account on first successful verification
    /// for this contact. Issues a fresh access/refresh token pair.
    pub async fn verify_otp(
        &self,
        channel: &str,
        identifier: &str,
        code: &str,
        user_agent: Option<&str>,
        ip: Option<&str>,
    ) -> AppResult<TokenPair> {
        Self::validate_channel(channel)?;

        let challenge = self
            .repo
            .find_active_otp_challenge(channel, identifier)
            .await?
            .ok_or(AppError::Unauthorized)?;

        if challenge.attempts >= challenge.max_attempts {
            return Err(AppError::RateLimited);
        }

        if !verify_secret(code, &challenge.code_hash)? {
            self.repo.increment_otp_attempts(challenge.id).await?;
            return Err(AppError::Unauthorized);
        }
        self.repo.consume_otp_challenge(challenge.id).await?;

        let account = match self
            .repo
            .find_account_by_contact(channel, identifier)
            .await?
        {
            Some(account) => account,
            None => {
                self.repo
                    .create_account_with_contact(channel, identifier, identifier)
                    .await?
            }
        };

        self.issue_tokens(account.id, user_agent, ip).await
    }

    /// Mint a fresh access/refresh pair carrying the account's *current*
    /// roles. `verify_otp` already calls this internally; onboarding
    /// (`register_warehouse`/`register_store`/`register_carrier`) also calls
    /// it directly after granting a new role — the session issued at sign-in
    /// time predates that role, so without this the caller would be stuck
    /// holding a token that can never pass the new role's guard until it
    /// naturally expires and is refreshed.
    pub async fn issue_tokens(
        &self,
        account_id: Uuid,
        user_agent: Option<&str>,
        ip: Option<&str>,
    ) -> AppResult<TokenPair> {
        let roles = self.repo.list_roles(account_id).await?;
        let access_token = self
            .jwt
            .issue_access(account_id, roles.clone())
            .map_err(|_| AppError::Internal(anyhow::anyhow!("failed to issue access token")))?;

        // The session id is generated here (not by the database default) so
        // the refresh token's `{session_id}.{secret}` shape can be built
        // before the row is written.
        let session_id = Uuid::new_v4();
        let refresh_token = generate_refresh_token(session_id);
        let parts = parse_refresh_token(&refresh_token).expect("just generated, always parses");
        self.repo
            .create_session(
                session_id,
                account_id,
                &hash_secret(&parts.secret)?,
                self.refresh_ttl_secs,
                user_agent,
                ip,
            )
            .await?;

        Ok(TokenPair {
            account_id,
            access_token,
            refresh_token,
            roles,
        })
    }

    pub async fn refresh(
        &self,
        refresh_token: &str,
        user_agent: Option<&str>,
        ip: Option<&str>,
    ) -> AppResult<TokenPair> {
        let parts = parse_refresh_token(refresh_token).ok_or(AppError::Unauthorized)?;
        let session = self
            .repo
            .find_session(parts.session_id)
            .await?
            .ok_or(AppError::Unauthorized)?;

        if session.revoked_at.is_some() || session.expires_at < chrono::Utc::now() {
            return Err(AppError::Unauthorized);
        }
        if !verify_secret(&parts.secret, &session.secret_hash)? {
            return Err(AppError::Unauthorized);
        }

        // Rotate: revoke the used session, issue a brand-new one. A reused
        // (already-revoked) refresh token can never mint another pair.
        self.repo.revoke_session(session.id).await?;
        self.issue_tokens(session.account_id, user_agent, ip).await
    }

    /// Revoke the refresh-token session and, when the caller also presents
    /// their still-valid access token claims, denylist its `jti` too — see
    /// `auth::denylist` for why both halves need revoking.
    pub async fn logout(
        &self,
        refresh_token: Option<&str>,
        access_jti: Option<Uuid>,
        access_remaining_ttl_secs: i64,
    ) -> AppResult<()> {
        if let Some(refresh_token) = refresh_token {
            if let Some(parts) = parse_refresh_token(refresh_token) {
                self.repo.revoke_session(parts.session_id).await?;
            }
        }
        if let Some(jti) = access_jti {
            self.denylist.revoke(jti, access_remaining_ttl_secs).await?;
        }
        Ok(())
    }
}
