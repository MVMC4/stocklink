# StockLink work orders

Do them in order. Each is done only when every acceptance item has evidence in
`docs/STATUS.md`.

---

## WO-02 — Recreate configuration, middleware and security controls from scratch

**Goal.** A secure, typed runtime written fresh.

- `backend/src/config/mod.rs`: typed config for server, database (pool size, acquire timeout), Redis, JWT, OTP (dev echo allowed only in development), email and SMS providers, CORS origins, rate limits, media storage, telemetry. Startup **refuses** unsafe production settings.
- `backend/.env.example`: every variable, placeholders only, comments on dev-only values.
- `backend/src/middleware/mod.rs`: exact-origin CORS allow-list, request id, security headers (HSTS, nosniff, frame deny, referrer policy), HTTPS enforcement behind a trusted proxy, Redis rate limiting (IP and per-account for OTP), request validation, metrics with route-labelled latency histograms.
- Gateway blocks `/metrics` publicly.

**Acceptance.** Unit tests per control, each red-tested (e.g. disallowed origin rejected; production with OTP echo refuses to start; limiter returns 429 after budget). No secret values committed.

## WO-03 — Recreate routing, OpenAPI and module wiring; get the backend green

**Goal.** The backend compiles and all gates pass.

- Write `routes/mod.rs`, `openapi.rs` and the five `mod.rs` files from scratch for the modules kept after WO-04's decisions (do WO-04's removal list first if simpler).
- OpenAPI coverage guard: every mounted route is documented and vice versa; unique operation ids; all schema refs resolve.
- Regenerate `.sqlx` with `cargo sqlx prepare` against a fresh local database.

**Acceptance.** `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo sqlx prepare --check`, `cargo test` all pass; coverage guard red-tested.

## WO-04 — Domain scope and schema

**Goal.** Only StockLink's domains exist.

- **Kept:** auth (email OTP for business users; phone optional), accounts, onboarding (warehouse, store, carrier roles), catalog, cart, orders, settlements, ledger, media, outbox, admin console, notifications, health.
- **Schema:** one fresh `0001_baseline` migration per service, append-only from there.
- Roles: `warehouse`, `store`, `carrier`, `admin`.

**Acceptance.** Gates green; migration test up/down clean on an empty database.

## WO-05 — Web foundation and design system

**Goal.** A running, minimal, glass UI.

- Add `web/package.json` (React, Vite, TypeScript, TanStack Router and Query, lucide icons, Vitest, ESLint).
- App shell: sidebar, top bar, command palette (`Ctrl/Cmd+K`), toasts, empty/loading/error states.
- Design tokens already started in `web/src/styles.css`: extend with light and dark themes, contrast-checked pairs (≥ 4.5:1), reduced-motion and reduced-transparency fallbacks.
- `scripts/no-gradients-guard.mjs`: fails on `gradient(` in CSS/TS/SVG; red-tested.
- Session: HttpOnly cookie auth against the API (no tokens in `localStorage`).

**Acceptance.** `pnpm typecheck`, `lint`, `test`, `build` pass; guard red-tested; screenshot of the shell in light and dark; no preview data unlabelled.

## WO-06 — Commerce and bulk consolidation

**Goal.** Warehouses sell, stores order, demand is pooled.

- Warehouse catalogue with unit, case and pallet tiers; stock levels.
- Store cart and checkout against server pricing; idempotent order creation.
- Bulk orders: pool compatible store demand (same product, warehouse, delivery window) into a consolidated order with per-store allocations, invoices and settlements; tier price applied server-side.
- Web screens: catalogue, cart, orders, bulk order board, settlements.

**Acceptance.** Contract tests for pricing, idempotency, allocation totals equal the bulk order, and ownership scoping (store A cannot see store B); UI verified live.

## WO-07 — Parcel tracking (fresh logistics)

**Goal.** Every shipment is traceable.

- Shipments linked to orders and bulk allocations; carrier assignment; milestones (packed, collected, in transit, out for delivery, delivered, exception).
- Position ingest with offline queue on the carrier client; last known position and track; stale indicator.
- Public tracking page by unguessable reference (no account, minimal data).
- Proof of delivery (photo or signature) with retention limits.

**Acceptance.** Contract tests for milestone order and scoping; public page leaks no personal data (tested); honest "no position yet" state.

## WO-08 — Assistant layer

**Goal.** Fast, helpful, honest interaction.

- Command palette actions (navigate, search, create order, start bulk order).
- Rule-based suggestions over real data (e.g. pooling opportunities, low stock), each labelled "suggestion" with its reason.
- Optional language-model integration behind a feature flag with a visible label and no silent fallbacks.

**Acceptance.** Every suggestion traceable to data and rule; flag off by default; tests for each rule.

## WO-09 — Documentation site

**Goal.** A populated docs site.

- Update `docs-site/scripts/manifest.mjs` to product documentation only:
  Getting started, Architecture, Domain model, API reference, Operations,
  Design system. Don't map `docs/WORK_ORDERS.md` or `docs/STATUS.md` into a
  section — those are internal engineering records, not product docs.
- Generate the API reference from the StockLink OpenAPI document.
- Add Getting started, Architecture, Domain model, API, Operations, Design system pages.

**Acceptance.** Docs build passes; link check passes (red-tested).

## WO-10 — CI, observability and release readiness

**Goal.** Gates enforced and the system observable.

- GitHub Actions at repository root: backend gates, web gates, no-gradients guard, docs build. Red-test that a failing check blocks the run.
- Prometheus metrics (latency histograms, DB pool, outbound calls), Grafana dashboards, alert rules with tests.
- Dev Compose on StockLink ports; production deployment plan documented, not executed without owner approval.

**Acceptance.** CI red-tested; dashboards show data in dev; alert rule tests pass.
