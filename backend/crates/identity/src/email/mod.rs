//! Transactional email delivery abstraction shared by OTP dispatch and (later)
//! onboarding lifecycle messages.
//!
//! Mirrors `sms`: [`gateway`] holds the `EmailGateway` trait and a
//! non-production logging stub, [`resend`] holds the real Resend-backed
//! implementation. Provider-specific code stays in [`resend`] only
//! (`.claude/rules/backend-alignment.md` rule 6).

pub mod gateway;
pub mod resend;

// Every type here is referenced via its submodule path at each use site
// (`email::gateway::EmailGateway`, `email::resend::ResendEmailGateway`, ...),
// so there is no top-level re-export.
