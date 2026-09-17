//! Background retention reaper: prunes churny, short-lived rows that
//! otherwise accumulate unbounded — expired OTP challenges and
//! expired/revoked sessions. See `service::CleanupScheduler`.

pub mod service;
