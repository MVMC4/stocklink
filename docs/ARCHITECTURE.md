# Architecture

Four independent Rust/Axum services, each with its own Postgres database,
behind one nginx gateway — split by domain, not by table (see
`docs/STATUS.md`'s microservices entry for the full reasoning).

## Services

| Service | Owns |
| --- | --- |
| `identity` | accounts, OTP auth, onboarding (warehouse/store/carrier), admin console |
| `commerce` | catalogue, cart, checkout, bulk order consolidation, settlements, ledger |
| `notifications` | in-app inbox, push (FCM) |
| `media` | presigned uploads (S3 in production; a local-disk backend in dev) |

## Stack

| Layer | Technology |
| --- | --- |
| API | Rust, Axum, sqlx, PostgreSQL (one database per service), Redis (JWT denylist, rate limiting) |
| Cross-service auth | Stateless JWT verified independently by every service; internal-only HTTP APIs (`X-Internal-Token`, never gateway-routed) for cross-service lookups |
| Zero-downtime updates | Health-gated rolling restart on plain Docker Compose (`scripts/rolling-update.mjs`) — no Kubernetes |
| Events | Transactional outbox pattern implemented in `commerce`; the Kafka publisher binary exists but isn't deployed yet — nothing currently writes to the outbox |
| Web app | React, Vite, TypeScript, TanStack Query |
| Documentation | Fumadocs/Next.js site generated from `docs/` and a committed OpenAPI snapshot |
| Local stack | Docker Compose: 4 services + Postgres + Redis + web + docs, one gateway |

## Repository layout

| Path | Contents |
| --- | --- |
| `backend/` | Rust/Axum services (`crates/{identity,commerce,notifications,media,shared}`) |
| `docker/` | Local Compose stack, gateway routing, observability provisioning |
| `docs/` | Product and process documentation, screenshots |
| `docs-site/` | Generated documentation site tooling |
| `web/` | React web app |
| `scripts/` | Provenance guard, health-gated rolling-update script |
