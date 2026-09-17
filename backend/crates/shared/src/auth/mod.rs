//! JWT verification, claims/roles, the access-token denylist, and the
//! `AuthUser` request extractor — everything a service needs to
//! *authenticate* a request. Issuing tokens (the OTP flow) is identity
//! service business logic, not shared.

pub mod claims;
pub mod denylist;
pub mod extractor;
pub mod jwt;

pub use claims::TokenClaims;
pub use denylist::AccessTokenDenylist;
pub use extractor::{AuthState, AuthUser, WarehouseUser, SESSION_COOKIE};
pub use jwt::JwtKeys;

/// Failures specific to token handling. Map to `AppError::Unauthorized` at the
/// boundary so we never disclose *why* a token was rejected.
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("token expired")]
    Expired,
    #[error("token invalid")]
    Invalid,
    #[error("failed to sign token")]
    Signing,
}
