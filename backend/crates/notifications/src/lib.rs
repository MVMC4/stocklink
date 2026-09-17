//! StockLink notifications service — in-app notification inbox and push
//! (FCM) delivery.
#![allow(dead_code)]

pub mod config;
pub mod controllers;
pub mod fcm;
pub mod models;
pub mod openapi;
pub mod provider;
pub mod repositories;
pub mod routes;
pub mod schemas;
pub mod service;
pub mod state;
