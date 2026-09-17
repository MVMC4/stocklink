#!/usr/bin/env node
/**
 * Downloads each service's live OpenAPI document and merges them into the
 * committed snapshot the API reference is generated from
 * (`docs/reference/openapi.json`).
 *
 * StockLink is four independent services (see docs/STATUS.md's
 * microservices entry), each with its own `/openapi.json` — there is no
 * single backend to fetch one document from any more. Paths and schemas
 * are namespaced by service name to avoid collisions (none exist yet, but
 * nothing stops a future one).
 *
 * This is a manual/CI step, not part of `predev`/`prebuild` — dev services
 * are not always running, and the reference must build from a stable,
 * reviewed snapshot rather than whatever happens to be running at build
 * time. Re-run this after an accepted API contract change, review the diff,
 * then run `generate-api-reference.mjs` (or `pnpm sync`, which calls it).
 *
 * Usage (defaults assume the dev stack's direct service ports — see
 * docker/compose.dev.yml; those services publish no fixed host port by
 * design, so from outside Docker use `docker exec <container> curl
 * localhost:8080/openapi.json` per service, or override each URL below):
 *   node scripts/fetch-openapi.mjs
 *   IDENTITY_OPENAPI_URL=http://localhost:8281/openapi.json node scripts/fetch-openapi.mjs
 */
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const here = path.dirname(fileURLToPath(import.meta.url));
const outPath = path.resolve(here, '../../docs/reference/openapi.json');

const SERVICES = [
  { name: 'identity', urlEnv: 'IDENTITY_OPENAPI_URL', default: 'http://localhost:8281/openapi.json' },
  { name: 'commerce', urlEnv: 'COMMERCE_OPENAPI_URL', default: 'http://localhost:8282/openapi.json' },
  { name: 'notifications', urlEnv: 'NOTIFICATIONS_OPENAPI_URL', default: 'http://localhost:8283/openapi.json' },
  { name: 'media', urlEnv: 'MEDIA_OPENAPI_URL', default: 'http://localhost:8284/openapi.json' },
];

async function fetchSpec(service) {
  const url = process.env[service.urlEnv] ?? service.default;
  let res;
  try {
    res = await fetch(url);
  } catch (err) {
    throw new Error(`could not reach ${service.name} at ${url}: ${err.message}`);
  }
  if (!res.ok) throw new Error(`GET ${url} -> ${res.status} ${res.statusText}`);
  const spec = await res.json();
  if (!spec.openapi || !spec.paths) throw new Error(`${url} does not look like an OpenAPI document`);
  return spec;
}

async function main() {
  const specs = await Promise.all(SERVICES.map(fetchSpec));

  const merged = {
    openapi: specs[0].openapi,
    info: { title: 'StockLink API', version: '0.1.0' },
    paths: {},
    components: { schemas: {} },
  };

  for (const [i, spec] of specs.entries()) {
    const service = SERVICES[i].name;
    for (const [p, ops] of Object.entries(spec.paths ?? {})) {
      if (merged.paths[p]) {
        console.warn(`path collision on ${p} (${service}) — keeping the first service's definition`);
        continue;
      }
      merged.paths[p] = ops;
    }
    for (const [name, schema] of Object.entries(spec.components?.schemas ?? {})) {
      merged.components.schemas[name] = schema; // identical shared shapes (e.g. health) collide harmlessly
    }
  }

  fs.mkdirSync(path.dirname(outPath), { recursive: true });
  fs.writeFileSync(outPath, JSON.stringify(merged, null, 2) + '\n');
  const pathCount = Object.keys(merged.paths).length;
  console.log(`wrote ${path.relative(process.cwd(), outPath)} (${pathCount} paths from ${specs.length} services)`);
}

main().catch((err) => {
  console.error(err.message);
  console.error('Is the dev stack running? docker compose -f docker/compose.dev.yml up -d --build');
  process.exit(1);
});
