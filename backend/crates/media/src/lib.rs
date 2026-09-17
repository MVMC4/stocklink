//! StockLink media service — presigned uploads (catalogue photos, proof of
//! delivery) to S3-compatible object storage.
#![allow(dead_code)]

pub mod config;
pub mod controllers;
pub mod models;
pub mod openapi;
pub mod public_media;
pub mod repositories;
pub mod routes;
pub mod s3_sigv4;
pub mod schemas;
pub mod services;
pub mod state;
