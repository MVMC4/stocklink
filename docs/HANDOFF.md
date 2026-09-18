# Handoff — 2026-09-18, branch `feat/ui-ux-improvements`

Written for whoever (human or agent) picks this branch up next, because the
session that did this work ran out of usage budget. Read this whole file
before touching code — it says what's done, what's verified vs. not, and
exactly what's next. If you're an AI agent: **read [`AGENTS.md`](../AGENTS.md)
and [`docs/WORK_ORDERS.md`](WORK_ORDERS.md) first**, they're this repo's own
execution contract and this file doesn't repeat their rules.

## Where things stand right now

- **Branch:** `feat/ui-ux-improvements`, pushed to `origin`. Not merged, no
  PR opened yet (left for you/the owner to decide — see "Suggested next
  steps").
- **3 commits on top of `main`**, each independently coherent:
  1. `a53bae3` — fast standalone web dev loop (Vite on :5191)
  2. `ad2d1a1` — backend: catalogue item photos (up to 5, thumbnail, reorder)
  3. `3d24dbd` — frontend: wires photos into both screens
- **Repo went public this session** (previously private) — see `a5c8214`
  and the [MVMC4/stocklink](https://github.com/MVMC4/stocklink) repo itself.
  No secrets were found in the initial commit; dev-only placeholder
  credentials only.
- **GitHub profile README** (`MVMC4/MVMC4`) was rewritten to feature
  StockLink and `transit-route-optimization` as flagship projects — done
  directly via the GitHub API, not part of this repo/branch.

## What was actually built this session, in order

1. **Repo made public.** Checked for secrets (found none — only
   `dev-only-change-me` style placeholders and AWS's own public
   `AKIAIOSFODNN7EXAMPLE` test key). Updated the README's closing line for a
   public/portfolio framing (all rights reserved, no license granted).
2. **Docker stack refreshed** (`docker compose -f docker/compose.dev.yml down`
   then `up -d --build`) to pick up the README change and confirm a clean
   baseline. All 9 `stocklink-dev-*` containers came up healthy. (A second,
   larger rebuild was still running when this session's budget ran out —
   see "Unfinished / needs a rebuild" below.)
3. **Fast dev loop for the web app.** `web/vite.config.ts` now runs on its
   own port (5191) and proxies `/api/*` to the gateway container (:5190)
   instead of to unpublished per-service ports that don't exist on the host
   (identity/commerce/notifications/media deliberately have no fixed host
   port — see the comment in `docker/compose.dev.yml` about rolling
   updates). `npm run dev` inside `web/` now just works standalone against
   the already-running Docker backend, with real HMR. Wired through
   `.claude/launch.json` as a named dev server. Verified working (screenshot
   of the real login screen, a real `/api/v1/catalog` response through the
   proxy).
   - Also added `web/.dockerignore` (was missing `node_modules`, which was
     bloating every docker build context once `npm install` had been run
     locally for this workflow — a real, unrelated bug this surfaced).
4. **Catalogue item photos** — the main feature request. Details below.
5. **Marketing site draft** — a separate design-canvas Artifact (not part of
   this repo), matching StockLink's actual neo-brutalist tokens pulled from
   `web/src/styles.css`. Link was shared earlier in the session; ask the
   user if they still have it, it isn't reproducible from this repo.
6. **Dev-only seed images.** Inserted `catalog_item_images` rows directly
   into the running dev database for the items already there (2 seeded
   items existed from earlier ad-hoc testing, not from `seed.sql`, which is
   still a stub — see gaps below), using Lorem Picsum
   (`https://picsum.photos/seed/<name>/900/700`) for realistic-looking,
   copyright-safe placeholder photos. This is **not** part of any migration
   or seed script — it's rows in the running dev Postgres only, and will be
   lost if you `docker compose down -v` (which destroys the dev volumes).
7. **GitHub profile + repo housekeeping** (outside this repo/branch):
   rewrote `MVMC4/MVMC4`'s README to lead with StockLink and
   `transit-route-optimization`; set a proper description on
   `transit-route-optimization` (was blank). **Could not pin StockLink to
   the profile** — GitHub's API has no mutation for profile-pinned
   repositories (verified via GraphQL schema introspection: only
   `pinIssue`/`pinEnvironment` exist, nothing for repos). You need to do
   this by hand: open the [MVMC4 profile](https://github.com/MVMC4), click
   "Customize your pins", check StockLink. `transit-route-optimization` was
   already pinned before this session; it's untouched.

## The catalogue-images feature, in detail

### Data model (commerce service, migration `0002_catalog_images.sql`)

New table `catalog_item_images`: `id`, `catalog_item_id` (FK, cascade
delete), `media_asset_id` (opaque cross-service ref to media's own
`media_assets` table — same no-FK-across-services pattern as everything
else in this schema), `url`, `sort_order` (smallint), `is_thumbnail`
(boolean). A **partial unique index** enforces at most one thumbnail per
item at the DB level (`... WHERE is_thumbnail`). No unique constraint on
`sort_order` — reordering does sequential single-row updates rather than a
transaction, which is a deliberate simplification (see gaps).

### New API surface (commerce, all warehouse-scoped and ownership-checked)

- `POST /v1/warehouses/{warehouse_id}/catalog/{item_id}/images`
  `{ media_asset_id, url }` → `201 CatalogItemRes` (max 5 enforced
  server-side as `409 Conflict`; `url` must be `http(s)://...`)
- `DELETE /v1/warehouses/{warehouse_id}/catalog/{item_id}/images/{image_id}`
  → `200 CatalogItemRes` (deleting the thumbnail auto-promotes the next
  lowest `sort_order` image, so a listing with photos never ends up
  thumbnail-less)
- `PUT /v1/warehouses/{warehouse_id}/catalog/{item_id}/images/order`
  `{ image_ids: [...] }` → `200 CatalogItemRes` (must list exactly the
  item's current image ids, once each, or `400 Validation`)
- `PUT /v1/warehouses/{warehouse_id}/catalog/{item_id}/images/{image_id}/thumbnail`
  → `200 CatalogItemRes`

`CatalogItemRes` (returned by these, `publish_item`, `list_warehouse_catalog`
and `browse_catalog`) gained:

```jsonc
{
  // ...existing fields...
  "thumbnail_url": "https://.../900/700",   // or null
  "images": [{ "id", "url", "sort_order", "is_thumbnail" }],
  "warehouse_name": "...",                  // denormalized from identity
  "warehouse_region": "...",
  "warehouse_address": "..." // or null
}
```

The warehouse fields are a **read-time join over the network**
(`IdentityClient::get_warehouse`), not stored in commerce's own DB —
`browse_catalog` dedupes by warehouse id before calling out (one call per
distinct warehouse on the page, not per item). `identity`'s internal
`/internal/warehouses/:id` endpoint gained `address` in its response to
support this (it only returned `name`/`region` before).

### Media service — the other real piece of work

`media`'s "local" dev storage backend (`PUBLIC_MEDIA_BACKEND=local`, what
`docker-compose.dev.yml` actually runs) existed only as a stub:
`presign_upload` would hand back a URL, but **nothing served requests to
it** — no route, no file I/O. This was silently broken before this session;
uploading an image in dev would have failed outright. Finished it:

- `PUT /v1/media/local/*key` (auth required) — writes bytes to
  `PUBLIC_MEDIA_LOCAL_DIR` (the `media-data` Docker volume, already declared
  in compose, just unused until now). Validates the object key is exactly
  `{kind}/{account_id}/{uuid}` and that `{account_id}` matches the
  authenticated caller before touching disk, both to stop one account
  overwriting another's upload and to make path traversal impossible.
- `GET /v1/media/local/*key` (no auth — mirrors the S3 backend, where these
  objects are meant to be publicly readable) — serves the bytes back with
  the content-type recorded at presign time.
- `docker/nginx/gateway.dev.conf` needed a **more specific location block**
  for `/api/v1/media/local/` with a larger `client_max_body_size` (12m) —
  the existing `/api/v1/media/` block caps at 1m because for the S3 backend
  only the small presign *request* goes through the gateway, never the
  actual image bytes. For local dev, the bytes really do flow through this
  route, so it needed room.

**Production is unaffected** — the S3 backend
(`PUBLIC_MEDIA_BACKEND=s3`) already worked before this session
(`s3_sigv4.rs`, untouched) and doesn't use any of the above; it just needs
real bucket credentials at deploy time. No MinIO or other S3-compatible
container was stood up for dev — the local-disk path was already
half-built and was the smaller lift. If a future session wants dev to
exercise the *exact* production code path, standing up MinIO in
`docker-compose.dev.yml` and pointing `PUBLIC_MEDIA_BACKEND=s3` at it is a
reasonable follow-up, not required.

### Frontend

- `web/src/lib/api.ts`: `CatalogItem` gained the new fields;
  `uploadCatalogImage(warehouseId, itemId, file)` does presign → PUT bytes →
  attach in one call; a new `uploadFile()` helper PUTs raw bytes (not JSON)
  to an absolute URL; `api.put()` was added (only `get`/`post`/`patch`/`delete`
  existed before).
- `web/src/screens/WarehouseDashboard.tsx`: new `ImageManager` component
  (upload via a file input, star-to-set-thumbnail, left/right arrows to
  reorder, X to remove — **not drag-and-drop**, see gaps). Publishing a new
  item opens straight into it. Existing rows get a "Photos (n/5)" button.
  Table rows show the thumbnail.
- `web/src/screens/Marketplace.tsx`: product cards show the thumbnail;
  clicking a card opens a new `ListingDetail` modal with the full photo
  gallery (click a thumbnail strip image to change the main image),
  description, every configured price tier, stock, and a "Ships from
  {warehouse name} · {region} · {address}" line. Tier/quantity/add-to-cart
  logic was factored into a shared `useAddToCart(item)` hook so the card and
  the detail view can't drift.

## What was verified vs. NOT verified

**Verified this session:**
- `cargo check -p stocklink-commerce -p stocklink-media -p stocklink-identity`
  — clean compile, only a pre-existing unrelated `sqlx-postgres` future-incompat
  warning.
- `npm run typecheck` and `npm run lint` in `web/` — clean (lint: 3
  pre-existing `react-refresh/only-export-components` warnings, not errors,
  not new to this session's pattern).
- The fast dev loop (:5191) end-to-end against the live Docker backend,
  including a real API response through the proxy.

**NOT run this session — do this before merging:**
- `cargo fmt --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test` (there's an existing `commerce_flow.rs` integration test —
  it was not touched, but also not re-run against the new schema)
- `cargo sqlx prepare --check` — almost certainly moot, since every query in
  this codebase uses the runtime `sqlx::query_as::<_, T>("...")` form, not
  the compile-time-checked `query_as!` macro, but worth confirming
- `pnpm test` / `pnpm build` in `web/` (only `typecheck`/`lint` were run)
- **The image upload flow was never exercised through the running app** —
  the migration hadn't been applied yet (see below) when the session ran
  out of budget, so nothing about upload/reorder/thumbnail/delete has been
  clicked through in a browser. The seed-image SQL insert (step 6 above)
  also failed on the first attempt for the same reason.
- `docs-site` build/link-check (untouched, but also never confirmed still
  green)

## Unfinished — pick this up first

A `docker compose -f docker/compose.dev.yml up -d --build` was **still
running** when this session ended, rebuilding all service images (release
profile, with LTO — slow) to pick up:
- the new migration (`0002_catalog_images.sql`)
- all the Rust changes described above
- the gateway config change

**Before doing anything else:** check if that build finished
(`docker compose -f docker/compose.dev.yml ps`, and check
`commerce-migrate`/`media` exited/started cleanly). If it didn't finish or
failed, re-run it. Once it's up, the seed-image insert in the session's
history (see git-independent SQL, not committed anywhere — you'll need to
re-run it or write your own) can go in, and then **actually click through
the feature in a browser** — nothing above has been eyeballed running yet.

## Known gaps / deliberate scope cuts (not oversights — flagging them honestly)

1. **No real distance/"X km away."** Warehouses and stores have `region`
   and an optional free-text `address` in identity's schema — no
   geocoordinate on either. The listing detail view shows
   `warehouse_region`/`warehouse_address` (real data) and deliberately does
   **not** show a fabricated distance number. A real "distance from you"
   feature needs: a geocoordinate on `Warehouse` and `Store` (new identity
   migration), a geocoding step at registration time (address → lat/lng,
   needs a provider), and a haversine calculation somewhere in commerce or
   the frontend. Not started.
2. **Reordering is arrow-buttons, not drag-and-drop.** Functionally
   complete (you can rearrange the 5 images into any order) but not the
   drag gesture the original request implied. Would need an HTML5 DnD (or a
   library) pass in `ImageManager`.
3. **Commerce trusts the client-supplied image `url` without confirming it
   with the media service.** `AttachImageReq` is only validated as
   "starts with `http://`/`https://`" server-side — there's no network call
   back to media to confirm `media_asset_id` actually belongs to the
   calling account or even exists. Low real risk (an `<img>` tag can't
   execute anything from an arbitrary URL), but it's a real gap from "every
   cross-service reference is checked over the network" that the rest of
   this codebase holds to.
4. **`byte_size` on `media_assets` isn't reliably updated.** `store_local`
   does update it after a successful write, but if a caller presigns and
   never uploads, the row stays at the `0` it was created with. Cosmetic
   only — nothing reads `byte_size` yet.
5. **`seed.sql` is still the WO-04-era stub** ("seeds nothing"). The two
   catalog items this session added photos to were **not** created by it —
   they exist in the dev DB from earlier ad-hoc testing/UI use, before this
   session. Writing a real seed script needs to create identity-side
   warehouse/store accounts too (via the onboarding API, not raw SQL, since
   the schema has no direct insert path for those), which is a bigger,
   cross-service task — properly a small work order of its own.
6. **`docs/STATUS.md` has no entries at all** despite WO-05 and most of
   WO-06 being built (see "Work order priority" below) — the project's own
   evidence log is behind the code. This session added one entry for its
   own work; the backlog from before this session is still unrecorded.
7. **No automated test was added for the images feature.** Given the
   "verify before merge" list above, treat this as untested until someone
   (human or agent) clicks through it and/or writes a contract test similar
   to `commerce/tests/commerce_flow.rs`'s existing style.

## Work order priority (for whoever picks the next piece of work, not just this branch)

Per [`docs/WORK_ORDERS.md`](WORK_ORDERS.md), sequentially:
- **WO-05** (web foundation) — done.
- **WO-06** (commerce) — catalogue/cart/orders are built and now have
  photos; the **bulk order board and settlements UI have no screens at
  all** despite `BulkOrder`/`BulkOrderAllocation`/`Settlement`/`LedgerEntry`
  models already existing in commerce. That's the nearest incomplete piece
  of WO-06.
- **WO-07 (parcel tracking) has not been started at all** — no
  shipment/carrier code beyond the onboarding role existing. This is the
  next full work order in the documented sequence, ahead of anything not on
  the list (like the distance feature above).

`AGENTS.md` rule 2 says one work order at a time — this session's work
(catalogue images) wasn't on the list at all, it was the owner's explicit
in-session request. Worth the next agent/human flagging that tension
consciously rather than silently drifting further from the documented plan.

## Suggested next steps, in order

1. Finish/verify the docker rebuild (see "Unfinished" above).
2. Re-seed dev images if the volume was reset, and **actually exercise the
   feature in a browser** — upload, reorder, set thumbnail, delete, view a
   listing detail, on both a warehouse account and a store account.
3. Run the full backend gate (`fmt --check`, `clippy -D warnings`, `test`)
   and web gate (`lint`, `test`, `build`) — fix whatever breaks.
4. Decide whether to open a PR (`gh pr create` from this branch) or keep
   iterating on it directly — not done this session since it wasn't asked
   for.
5. Manually pin StockLink on the GitHub profile (can't be scripted — see
   above).
6. Pick a next work order: bulk order board/settlements UI (finishes
   WO-06) or parcel tracking (WO-07, next in sequence) — the owner's call.

## Environment notes for whoever runs this next

- **Do not launch Docker Desktop via a shell command on this machine** —
  the owner reported it crashes when started that way; only their own
  Start Menu/taskbar shortcut works reliably. If Docker isn't running, ask
  them to start it themselves.
- Dev stack: `docker compose -f docker/compose.dev.yml up -d --build` →
  app at `http://localhost:5190`, docs at `http://localhost:3011`.
- Fast web-only loop: `cd web && npm run dev` (needs the Docker backend
  already running) → `http://localhost:5191`, hot-reloading, proxied
  through the gateway.
- There are **other, unrelated** `transit-*` Docker containers on this same
  machine (a different project, `transit-route-optimization`) — never
  `docker compose down`/`stop` those from this repo's context; scope any
  compose command to `-f docker/compose.dev.yml`.
