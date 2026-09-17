# StockLink docs site

A [Fumadocs](https://fumadocs.dev) documentation site (Next.js 16, Fumadocs 16,
Tailwind CSS 4). Content is **generated** from the monorepo's `docs/`
Markdown by `scripts/sync-content.mjs`, driven by an explicit manifest
(`scripts/manifest.mjs`) — edit the source Markdown or the manifest, never
`content/docs/` (entirely generated, entirely gitignored).

The site is **internal** and runs only in the dev environment. Demo, staging
and production have no documentation container, and their gateways answer
`/docs` with 404.

## Develop

```bash
pnpm install            # once (runs the `fumadocs-mdx` postinstall)
pnpm dev                # http://localhost:3010/docs  (syncs content first)
```

Or through the dev stack: `docker compose -f docker/compose.dev.yml up -d --build`,
then open <http://localhost:3100/docs>.

## Build

```bash
pnpm build && pnpm start
```

The Docker image (`docs-site/Dockerfile`, repository-root build context)
installs dependencies without lifecycle scripts, then syncs the Markdown,
generates the Fumadocs index (`fumadocs-mdx`) and runs `next build` in
standalone mode.

## How it fits together

| Path | What it is |
| --- | --- |
| `../docs/**/*.md` | source of truth for authored pages — reviewed in the repo |
| `../docs/reference/openapi.json` | committed OpenAPI snapshot — source of truth for the API reference |
| `scripts/manifest.mjs` | the **only** place that decides what gets published and where: an explicit list of `docs/**` files, each mapped to a site section/slug/title/order |
| `scripts/sync-content.mjs` | walks `manifest.mjs`, writing `../docs/*.md` → `content/docs/**/*.mdx` (frontmatter, MDX-escaped prose, generated `meta.json`), then runs `generate-api-reference.mjs` |
| `scripts/fetch-openapi.mjs` | downloads the backend's live OpenAPI document into the committed snapshot `../docs/reference/openapi.json` — a manual/CI step, not run automatically |
| `scripts/generate-api-reference.mjs` | reads the OpenAPI snapshot and writes the whole `content/docs/reference/**` section: an overview page plus one page per OpenAPI tag, with parameter/request/response tables resolved from the spec's schemas |
| `scripts/check-links.mjs` | fails on broken relative links in tracked Markdown (pass a modified `git ls-files` invocation, or diff manually, to also check new untracked files before their first commit) |
| `content/docs/**` | 100% generated, 100% gitignored — never edit, never commit |
| `source.config.ts`, `lib/source.ts` | Fumadocs MDX collection and the `fumadocs-core` loader |
| `app/docs/[[...slug]]` | the page renderer |
| `app/(home)/page.tsx` | the landing page's section cards |
| `app/api/search/route.ts` | search endpoint (the dev gateway routes `/api/search` here) |

### Adding a page

1. Write the Markdown under `docs/` (a new file, or reuse an existing one
   another part of the repo already maintains, e.g. a contract or the
   engineering log).
2. Add an entry to the right section's `pages` array in `scripts/manifest.mjs`
   — `{ src, slug, title }` (title is optional; falls back to the file's own
   `# heading`, then the filename). Mark `optional: true` if another
   workstream owns the file and it may not exist yet — `sync-content.mjs`
   skips a missing optional page instead of failing the build, and drops it
   from that section's `meta.json` order.
3. Run `node scripts/sync-content.mjs` (or `pnpm dev`/`pnpm build`, which run
   it automatically) and check the sidebar.

**Nothing under `docs/` is published just by existing.** `sync-content.mjs`
walks `manifest.mjs`'s explicit list, not `docs/**` wholesale — a new file
saved under `docs/` sits there unpublished until it's added to a section's
`pages` array. This is deliberate: it keeps working notes, drafts, and
anything not meant for a public site out by default.

### Adding an API reference page

Nothing to do by hand — every OpenAPI tag becomes a page automatically. To
pick up a **backend contract change**:

```bash
# 1. With the dev stack running (docker compose -f docker/compose.dev.yml up -d --build):
node scripts/fetch-openapi.mjs           # refreshes ../docs/reference/openapi.json
# 2. Review the diff in ../docs/reference/openapi.json, then commit it, then:
node scripts/sync-content.mjs            # regenerates content/docs/reference/**
```

StockLink is four independent services (identity, commerce, notifications,
media — see `docs/STATUS.md`'s microservices entry), each serving its own
`/openapi.json`; `fetch-openapi.mjs` fetches all four and merges them into
one document, namespacing collisions by service. Each service's URL is
overridable independently (`IDENTITY_OPENAPI_URL`, `COMMERCE_OPENAPI_URL`,
`NOTIFICATIONS_OPENAPI_URL`, `MEDIA_OPENAPI_URL`) — see the script for
defaults.

## What publishes and what does not

Published — exactly what `scripts/manifest.mjs` lists:

| `docs/` file | Site section |
| --- | --- |
| `VISION.md` | `/docs` — landing page ("Start here") |

Plus the generated API reference (`docs/reference/openapi.json` →
`content/docs/reference/**`, see above).

**Not published:** `WORK_ORDERS.md` and `STATUS.md` are internal
engineering records, not product documentation — deliberately left out of
`manifest.mjs`'s `SECTIONS`. As new *product* documentation is written
(Getting started, Architecture, API guides, Operations, Design system),
add it to `docs/` and to the right section's `pages` array in
`manifest.mjs`; anything saved under `docs/` but left out of the manifest
stays unpublished (see "Adding a page" above). Don't add a section by
pointing it at `WORK_ORDERS.md` or `STATUS.md`.

## Serving under the gateway

Pages live under the `/docs` route, so the dev gateway
(`docker/nginx/gateway.dev.conf`) proxies `/docs` straight through. Next.js
assets (`/_next/*`) and search (`/api/search`) are root-relative, so that
gateway also forwards those two paths. No `basePath` is needed.

## Diagrams

Fumadocs 16 ships a Mermaid remark plugin in `fumadocs-core`
(`mdx-plugins/remark-mdx-mermaid`), but it is not wired into
`source.config.ts`, and no `Mermaid` render component is registered in
`mdx-components.tsx` — wiring it up would need both, plus likely the
`mermaid` npm package as a runtime dependency. That wiring was intentionally
left out of the 14 September 2026 information-architecture rework rather
than added as a side effect of it. Diagrams on the site are ASCII/text for
now (see `docs/architecture/STACK_OVERVIEW.md`). Revisit if a page needs a
diagram ASCII genuinely can't express.
