# Demo credentials & endpoints (dev only)

Everything below is `docker/compose.dev.yml`'s dev stack only — none of this
exists in demo/staging/production (`OTP_DEV_ECHO_ENABLED` is unset there;
see `docs/ONBOARDING.md`). Start the stack first:

```bash
docker compose -f docker/compose.dev.yml up -d --build
```

## How sign-in actually works — there is no password

StockLink has no passwords anywhere. Every account signs in with a one-time
code emailed to it. In dev, `OTP_DEV_ECHO_ENABLED=true` skips the email
entirely — the API returns the code in the response and the app prints it
on screen in a blue "Development mode" banner. Enter any email below, hit
**Send code**, then read the code off the screen and enter it. That's the
whole flow.

## Seeded demo accounts

| Role | Email | What it owns |
| --- | --- | --- |
| Warehouse | `demo-warehouse-g2ohzl@stocklink.example` | Gaborone Central Warehouse — owns "Cordless Drill Set 18V" (has 3 photos already attached, good for exercising the image manager) |
| Store | `demo-store-lb3dz6@stocklink.example` | Tlokweng Fresh Mart — a plain store account, good for browsing the marketplace and checking out |

First sign-in with a brand-new email creates the account and asks it to
pick a role (warehouse or store) — the two emails above already have a role
and skip straight to their dashboard/marketplace. Use any other email to
walk the full first-run flow instead (see `docs/ONBOARDING.md`).

## URLs

| What | URL |
| --- | --- |
| The app (full Docker stack) | http://localhost:5190 |
| Fast dev loop — Vite HMR, no container rebuild (`cd web && npm run dev`) | http://localhost:5191 |
| Docs site (Fumadocs, dev-only) | http://localhost:3011 |
| Postgres (one database per service, shared instance) | `localhost:5443`, user/pass `stocklink`/`stocklink`, databases `stocklink_identity`/`stocklink_commerce`/`stocklink_notifications`/`stocklink_media` |
| Redis | `localhost:6390` |

## API surface

Every route below is reachable **only** through the gateway
(`http://localhost:5190/api/v1/...`) — the four backend services have no
published host ports of their own (see `docker/compose.dev.yml`'s comment on
why: they're designed to be scaled during a rolling update, and a scaled
service can't hold a fixed host port). A live, generated reference for every
endpoint — auth, request/response shapes, errors — is at
**http://localhost:3011/docs/reference**, built from the backend's own
OpenAPI document (`docs/reference/openapi.json`), not hand-maintained.

| Prefix | Service | Covers |
| --- | --- | --- |
| `/api/v1/auth/*`, `/api/v1/onboarding/*`, `/api/v1/admin/*` | `identity` | OTP sign-in, account/role management, admin console |
| `/api/v1/catalog*`, `/api/v1/warehouses/*`, `/api/v1/cart/*`, `/api/v1/orders*` | `commerce` | Catalogue (incl. the new photo endpoints), cart, checkout, orders |
| `/api/v1/notifications*`, `/api/v1/devices` | `notifications` | In-app inbox, push device registration |
| `/api/v1/media/*` | `media` | Presigned uploads (local-disk backend in dev) |

Each service also has its own Swagger UI (`/docs/swagger` on the service's
own port, `utoipa-swagger-ui`, non-production only) — not reachable through
the gateway by design (same reasoning as the missing host ports above); use
`docker compose -f docker/compose.dev.yml port <service> 8080` for a one-off
direct connection if you specifically need the interactive Swagger UI rather
than the generated reference site.
