# StockLink status log

Evidence of work performed, appended after every work order. Newest entry
first. See `docs/WORK_ORDERS.md` for the plan and `AGENTS.md` for the rules
this log exists to satisfy.

---

## 2026-09-18 — Catalogue item photos (not a numbered work order)

**Not on `docs/WORK_ORDERS.md`** — an explicit in-session request from the
owner, done on branch `feat/ui-ux-improvements`, not merged to `main`. Full
detail, including what's verified vs. not and known gaps, is in
[`docs/HANDOFF.md`](HANDOFF.md) — this entry is the short version.

**What:** up to 5 photos per catalogue item, one marked as thumbnail,
reorderable. New `catalog_item_images` table (commerce, migration
`0002_catalog_images.sql`), 4 new warehouse-scoped endpoints, and finished
the media service's local dev storage backend (`presign_upload` promised it
but nothing actually served `PUT`/`GET` requests before this — a real,
previously-broken piece of infrastructure). `CatalogItemRes` also gained
denormalized warehouse name/region/address for the marketplace listing
view. Frontend: an image manager on the warehouse catalogue screen, a full
listing-detail view with a photo gallery on the marketplace screen.

**Verified:** `cargo check` clean on `commerce`/`media`/`identity`;
`npm run typecheck`/`lint` clean in `web/`.

**Not verified:** `cargo fmt`/`clippy`/`test`, `pnpm test`/`build`, and —
importantly — **the feature has not been clicked through in a running
browser yet**. The docker rebuild needed to apply the migration was still
running when this session's usage budget ran out.

**Honest gaps, not fixed here:** no real "distance from warehouse" (no
geocoordinate on `Warehouse`/`Store` — see `docs/HANDOFF.md`); reordering is
arrow-buttons, not drag-and-drop; commerce trusts the client-supplied image
`url` without a network round-trip to media to confirm it; `seed.sql` is
still the WO-04-era stub, untouched.

**Next:** finish verifying (see `docs/HANDOFF.md`'s checklist), then either
WO-06's still-missing bulk order board/settlements UI or WO-07 (parcel
tracking, next in the documented sequence and not started at all).

---
