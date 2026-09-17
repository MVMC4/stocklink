//! StockLink commerce service — warehouse catalogue, store cart, checkout,
//! bulk order consolidation, settlements and the double-entry ledger.
#![allow(dead_code)]

pub mod clients;
pub mod config;
pub mod controllers;
pub mod models;
pub mod openapi;
pub mod repositories;
pub mod routes;
pub mod schemas;
pub mod services;
pub mod state;
