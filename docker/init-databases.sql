-- Runs once, only on first container start (postgres only executes
-- docker-entrypoint-initdb.d/* against a brand-new data directory) — creates
-- one database per service. Each service's own `<service> migrate` then
-- creates that database's tables; this file only makes the databases exist.
CREATE DATABASE stocklink_identity;
CREATE DATABASE stocklink_commerce;
CREATE DATABASE stocklink_notifications;
CREATE DATABASE stocklink_media;
