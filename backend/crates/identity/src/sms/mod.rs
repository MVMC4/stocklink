//! SMS delivery abstraction used for OTP dispatch.
//!
//! Mirrors the `notifications` module's provider/impl split: [`gateway`] holds
//! the `SmsGateway` trait and a logging stub, [`africas_talking`] holds the
//! real Africa's Talking-backed implementation.

pub mod africas_talking;
pub mod gateway;

pub use africas_talking::AfricaTalkingSmsGateway;
