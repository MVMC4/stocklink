# StockLink

StockLink connects **warehouses** to **retail stores**, turns scattered store
demand into **consolidated bulk orders**, and follows every shipment with
**live parcel tracking**.

Warehouses publish server-priced stock (unit, case, pallet tiers) with up to
five photos per listing. Stores browse and order across many warehouses from
one cart, with prices and totals always computed server-side, and can open
any listing for its full photo gallery, description and price tiers.
Compatible store demand for the same product and route is pooled into
consolidated bulk orders, giving stores volume pricing and warehouses
plannable, larger orders, while each store keeps its own allocation, invoice
and delivery. Every shipment carries a tracking reference, milestones, a
live position and proof of delivery.

## Status

The backend is split into four independent services behind one gateway, the
web app runs against all four end-to-end, and the docs site builds and
serves from the repository's own Markdown. See [`docs/STATUS.md`](docs/STATUS.md)
for the work-order-by-work-order evidence log — what's been built, how it
was verified, and what's honestly still missing.

**Recently added:** catalogue item photos — up to 5 per listing, one marked
as the thumbnail, reorderable, with a full gallery in the marketplace's
listing detail view. See [`docs/HANDOFF.md`](docs/HANDOFF.md) for exactly
what shipped, what's verified, and what's still open on that work.

## Planned features

Not built yet, in the order they're planned (see
[`docs/WORK_ORDERS.md`](docs/WORK_ORDERS.md) for the full spec of each):

- **Bulk order board & settlements UI** — the data model already exists
  (`BulkOrder`, `BulkOrderAllocation`, `Settlement`, `LedgerEntry` in
  `commerce`); there's no screen for either yet.
- **Parcel tracking** — shipments, carrier assignment, milestones, live
  position, proof of delivery, and a public tracking page by unguessable
  reference. Nothing beyond the carrier onboarding role exists today.
- **Assistant layer** — command palette actions, rule-based suggestions
  over real data (pooling opportunities, low stock), each labelled with its
  reasoning; an optional LLM integration behind a feature flag.
- **Real "distance from warehouse."** Warehouses and stores currently carry
  a region and a free-text address, not a geocoordinate — the marketplace
  shows real ship-from region/address today rather than a fabricated
  distance figure.

## Design

Thick black borders, hard offset shadows (no blur, no gradient — ever, see
`AGENTS.md`), bold flat accent colours, generous rounded corners. A floating
bottom dock replaces the sidebar below 880px instead of squeezing or
scrolling it.

## Screenshots

Real captures from the running dev stack — not mockups. See
[`docs/ONBOARDING.md`](docs/ONBOARDING.md) for the full first-run walkthrough
for both a warehouse and a store account.

![Sign in — current design](docs/assets/screenshots/14-login-professional.png)

The dashboard/catalogue/marketplace captures below are from just before the
professional-neobrutalism pass in `docs/DESIGN_SYSTEM.md` — same structure,
the old louder palette. Still accurate for layout, not for current colours.

| Warehouse dashboard | Marketplace |
| --- | --- |
| ![Warehouse dashboard](docs/assets/screenshots/05-warehouse-dashboard.png) | ![Marketplace](docs/assets/screenshots/10-marketplace.png) |

| Cart → checkout | Docs site |
| --- | --- |
| ![Cart](docs/assets/screenshots/11-cart.png) | ![Documentation site](docs/assets/screenshots/13-docs-site.png) |

## Documentation

- [`docs/VISION.md`](docs/VISION.md) — what StockLink is, who uses it, product principles
- [`docs/ONBOARDING.md`](docs/ONBOARDING.md) — first-run guide with screenshots
- [`docs/WORK_ORDERS.md`](docs/WORK_ORDERS.md) — the ordered build plan
- [`docs/STATUS.md`](docs/STATUS.md) — evidence log, updated after every work order
- [`docs/HANDOFF.md`](docs/HANDOFF.md) — session handoff notes for in-progress work
- [`docs/DEMO_CREDENTIALS.md`](docs/DEMO_CREDENTIALS.md) — dev sign-in accounts, every URL, the full API surface
- [`AGENTS.md`](AGENTS.md) — execution contract for anyone working in this repository

## Architecture

Four independent Rust/Axum services, each with its own Postgres database,
behind one nginx gateway — split by domain, not by table (see
`docs/STATUS.md`'s microservices entry for the full reasoning):

| Service | Owns |
| --- | --- |
| `identity` | accounts, OTP auth, onboarding (warehouse/store/carrier), admin console |
| `commerce` | catalogue, cart, checkout, bulk order consolidation, settlements, ledger |
| `notifications` | in-app inbox, push (FCM) |
| `media` | presigned uploads (S3 in production; a local-disk backend in dev) |

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

This repository is public for portfolio purposes. All rights reserved — no
license is granted to use, copy, or modify this code.
