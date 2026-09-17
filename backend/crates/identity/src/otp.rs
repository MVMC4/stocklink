//! Cryptographic helpers for the OTP/session flow: generating and hashing
//! one-time codes and refresh-token secrets, and the refresh token's own
//! `{session_id}.{secret}` encoding.
//!
//! The session id is embedded in plaintext (it's not a secret — it's a
//! lookup key) so `find_session` doesn't need to guess or hash-scan every
//! active session to find the one a presented token claims to belong to;
//! only the secret half is hashed at rest, exactly like a password.

use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use rand::Rng;
use uuid::Uuid;

use stocklink_shared::errors::{AppError, AppResult};

pub const OTP_TTL_SECS: u64 = 300;
pub const OTP_MAX_ATTEMPTS: u32 = 5;
pub const OTP_RATE_LIMIT_MAX_REQUESTS: u32 = 5;
pub const OTP_RATE_LIMIT_WINDOW_SECS: u64 = 300;

/// A 6-digit numeric code, zero-padded — easy to read aloud or key in from
/// an SMS, unlike a full random token.
pub fn generate_code() -> String {
    let n: u32 = rand::thread_rng().gen_range(0..1_000_000);
    format!("{n:06}")
}

/// 32 random bytes, hex-encoded — the refresh token's secret half.
fn generate_secret() -> String {
    let bytes: [u8; 32] = rand::thread_rng().gen();
    hex::encode(bytes)
}

pub fn generate_refresh_token(session_id: Uuid) -> String {
    format!("{session_id}.{}", generate_secret())
}

pub struct RefreshTokenParts {
    pub session_id: Uuid,
    pub secret: String,
}

pub fn parse_refresh_token(token: &str) -> Option<RefreshTokenParts> {
    let (session_id, secret) = token.split_once('.')?;
    Some(RefreshTokenParts {
        session_id: session_id.parse().ok()?,
        secret: secret.to_string(),
    })
}

/// Argon2id with the crate's default parameters — used for both OTP codes
/// and refresh-token secrets, the same way a password would be hashed at
/// rest, since both are short-lived bearer secrets an attacker with
/// database read access shouldn't be able to replay directly.
pub fn hash_secret(secret: &str) -> AppResult<String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(secret.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|err| AppError::Internal(anyhow::anyhow!("failed to hash secret: {err}")))
}

pub fn verify_secret(secret: &str, hash: &str) -> AppResult<bool> {
    let parsed = PasswordHash::new(hash)
        .map_err(|err| AppError::Internal(anyhow::anyhow!("failed to parse stored hash: {err}")))?;
    Ok(Argon2::default()
        .verify_password(secret.as_bytes(), &parsed)
        .is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_code_is_six_digits() {
        for _ in 0..20 {
            let code = generate_code();
            assert_eq!(code.len(), 6);
            assert!(code.chars().all(|c| c.is_ascii_digit()));
        }
    }

    #[test]
    fn refresh_token_round_trips_through_parse() {
        let session_id = Uuid::new_v4();
        let token = generate_refresh_token(session_id);
        let parts = parse_refresh_token(&token).expect("just generated, always parses");
        assert_eq!(parts.session_id, session_id);
        assert!(!parts.secret.is_empty());
    }

    #[test]
    fn malformed_refresh_token_does_not_parse() {
        assert!(parse_refresh_token("not-a-token").is_none());
        assert!(parse_refresh_token("not-a-uuid.secret").is_none());
    }

    #[test]
    fn hashed_secret_verifies_against_the_original() {
        let hash = hash_secret("123456").unwrap();
        assert!(verify_secret("123456", &hash).unwrap());
        assert!(!verify_secret("654321", &hash).unwrap());
    }
}
