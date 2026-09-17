# StockLink

StockLink connects **warehouses** to **retail stores**, turns scattered store
demand into **consolidated bulk orders**, and follows every shipment with
**live parcel tracking**.

## The problem

Small retailers buy stock in small, frequent, expensive orders from many
warehouses. Warehouses see fragmented demand they cannot plan for. Deliveries
are tracked by phone calls. Nobody sees the whole picture.

## What StockLink does

1. **Warehouse catalogue.** Warehouses publish stock with server-priced tiers
   (unit, case, pallet) and live availability.
2. **Retail ordering.** Stores browse, compare and order through one cart
   across many warehouses. Prices and totals are always computed on the server.
3. **Bulk consolidation.** Store demand for the same product and route is
   gathered into consolidated bulk orders. Stores get volume pricing;
   warehouses get plannable, larger orders. Each store keeps its own
   allocation, invoice and delivery.
4. **Parcel tracking.** Every shipment has a tracking reference, milestones
   (packed, collected, in transit, out for delivery, delivered), a live map
   position and proof of delivery. Stores and their customers can follow a
   parcel without an account.
5. **Settlement and ledger.** Each order settles into a double-entry ledger per
   warehouse, so balances and reports are exact.

## Who uses it

| Role | Main jobs |
| --- | --- |
| Warehouse | Publish stock, accept orders, pack, hand over to carriers, see settlements |
| Retail store | Order stock, join bulk orders, track deliveries, confirm receipt |
| Carrier | Accept shipments, report milestones and position, capture proof |
| Admin | Oversee accounts, orders, shipments and errors |

## Product principles

- **Honest by default.** Anything not built is shown as unavailable, never
  simulated as real. No fake data in live mode.
- **Server-authoritative.** Price, stock, ownership, order status and
  settlement come from the API, never from the browser.
- **Offline-tolerant tracking.** Carriers can report milestones and positions
  on poor connections; updates queue and sync.
- **Observable.** Every request is measured; dashboards and alerts come first,
  not after launch.

## Experience and interface

StockLink looks calm and quiet so the data stands out.

- **Minimalism first.** Generous whitespace, one accent colour, a restrained
  type scale, and no decoration without a job.
- **Glass surfaces.** Panels are translucent solid fills with backdrop blur
  over a plain background. **No gradients anywhere.**
- **Smooth motion.** Short, eased transitions (150–250 ms) for panels, lists
  and state changes. Motion is reduced for users who ask for it.
- **Assistant-style interaction.** A command bar (`Ctrl/Cmd + K`) to search,
  jump and act; inline suggestions such as "3 stores need 40 cases of X: start
  a bulk order?"; natural-language filters. Suggestions start as transparent
  rules over real data and say so; a language model is added only behind an
  explicit, labelled integration.

## Architecture

| Layer | Technology |
| --- | --- |
| API | Rust, Axum, sqlx, PostgreSQL, Redis |
| Events | Transactional outbox to Kafka |
| Web app | React, Vite, TypeScript, TanStack Router and Query |
| Monitoring | Prometheus and Grafana |
| Documentation | Static docs site generated from this folder and the OpenAPI document |
| Local stack | Docker Compose with a single gateway |

## Roadmap

| Phase | Outcome |
| --- | --- |
| 0. Foundation | Repository, docs, web shell with the design system, local stack |
| 1. Commerce core | Accounts and sign-in, warehouse catalogue, cart, orders, settlements, ledger |
| 2. Bulk consolidation | Demand pooling, bulk order lifecycle, per-store allocation |
| 3. Parcel tracking | Shipments, carriers, milestones, live map, public tracking page, proof of delivery |
| 4. Assistant layer | Command bar actions, rule-based suggestions, optional model integration |
| 5. Operations | Alerts, runbooks, admin console, deployment |
