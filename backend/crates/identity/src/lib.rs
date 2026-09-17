//! StockLink identity service — accounts, email/phone OTP auth, onboarding
//! (warehouse/store/carrier), and the admin console.
#![allow(dead_code)]

pub mod admin_links;
pub mod auth_service;
pub mod config;
pub mod controllers;
pub mod email;
pub mod maintenance;
pub mod models;
pub mod openapi;
pub mod otp;
pub mod repositories;
pub mod routes;
pub mod schemas;
pub mod services;
pub mod sms;
pub mod state;
