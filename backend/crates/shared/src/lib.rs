//! Cross-cutting infrastructure shared by every StockLink service.

pub mod auth;
pub mod config;
pub mod database;
pub mod errors;
pub mod middleware;
pub mod outbox;
pub mod utils;
