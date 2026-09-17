#!/usr/bin/env node
/**
 * Generates the API reference section of the docs site from the committed
 * OpenAPI snapshot (`docs/reference/openapi.json`, written by
 * `fetch-openapi.mjs`). Run by `sync-content.mjs` (and therefore by `pnpm
 * sync`/`predev`/`prebuild`) — never hand-edit anything under
 * `content/docs/reference/`.
 *
 * Output, one MDX file per unit:
 *   content/docs/reference/index.mdx        - API overview (base URLs, auth,
 *                                              error envelope, pagination,
 *                                              rate limits, regeneration)
 *   content/docs/reference/<tag>.mdx         - one page per OpenAPI tag,
 *                                              every operation under it
 *   content/docs/reference/meta.json         - ordering
 *
 * $refs are resolved locally (components.schemas only, the only $ref target
 * this spec uses) to render field tables, depth-limited to avoid runaway
 * recursion on self-referential or deeply nested schemas.
 */
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const here = path.dirname(fileURLToPath(import.meta.url));
const specPath = path.resolve(here, '../../docs/reference/openapi.json');
const outDir = path.resolve(here, '../content/docs/reference');

const MAX_SCHEMA_DEPTH = 3;

// ── MDX-safety helpers ──────────────────────────────────────────────────
// Table cells and inline code below can contain `{`, `<`, `}`, `|` from the
// spec (enum values, formats, descriptions). `escapeMdxProse` in
// sync-content.mjs handles prose; this generator writes its own MDX
// directly, so it escapes inline text itself before placing it in a table
// cell or heading.
function escInline(value) {
  return String(value ?? '')
    .replace(/\\/g, '\\\\')
    .replace(/&(?![a-zA-Z#][a-zA-Z0-9]*;)/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/\{/g, '&#123;')
    .replace(/\}/g, '&#125;')
    .replace(/\|/g, '\\|')
    .replace(/\r?\n/g, ' ');
}

function code(value) {
  return '`' + String(value ?? '').replace(/`/g, "'") + '`';
}

function humanizeTag(tag) {
  const overrides = {
    ussd: 'USSD',
    api: 'API',
  };
  return tag
    .split(/[-_]/)
    .map((w) => overrides[w.toLowerCase()] ?? w.charAt(0).toUpperCase() + w.slice(1))
    .join(' ');
}

function refName(ref) {
  return ref.replace('#/components/schemas/', '');
}

function resolveSchema(spec, schema, depth = 0, seen = new Set()) {
  if (!schema) return { type: 'unknown' };
  if (schema.$ref) {
    const name = refName(schema.$ref);
    if (seen.has(name) || depth >= MAX_SCHEMA_DEPTH) {
      return { type: 'ref', name, truncated: true };
    }
    const target = spec.components?.schemas?.[name];
    if (!target) return { type: 'ref', name, missing: true };
    return { type: 'ref', name, resolved: resolveSchema(spec, target, depth + 1, new Set([...seen, name])) };
  }
  if (schema.allOf) {
    const merged = { type: 'object', properties: {}, required: [] };
    for (const sub of schema.allOf) {
      const r = resolveSchema(spec, sub, depth, seen);
      if (r.properties) Object.assign(merged.properties, r.properties);
      if (r.required) merged.required.push(...r.required);
    }
    return merged;
  }
  if (schema.oneOf || schema.anyOf) {
    const variants = (schema.oneOf ?? schema.anyOf).map((s) => resolveSchema(spec, s, depth, seen));
    return { type: 'union', variants };
  }
  if (schema.type === 'array') {
    return { type: 'array', items: resolveSchema(spec, schema.items, depth, seen) };
  }
  if (schema.type === 'object' || schema.properties) {
    return {
      type: 'object',
      properties: schema.properties ?? {},
      required: schema.required ?? [],
      additionalProperties: schema.additionalProperties,
    };
  }
  return {
    type: schema.type ?? 'unknown',
    format: schema.format,
    enum: schema.enum,
    nullable: schema.nullable,
    description: schema.description,
  };
}

function scalarLabel(schema) {
  if (schema.$ref) return refName(schema.$ref);
  if (schema.allOf?.length === 1 && schema.allOf[0].$ref) return refName(schema.allOf[0].$ref);
  if (schema.type === 'array') {
    const items = schema.items ?? {};
    return `array of ${scalarLabel(items)}`;
  }
  let label = schema.type ?? 'object';
  if (schema.format) label += ` (${schema.format})`;
  if (schema.enum) label = `enum: ${schema.enum.map((v) => code(v)).join(', ')}`;
  if (schema.nullable) label += ', nullable';
  return label;
}

/** Render a field table for an object schema, resolving one level of $refs
 * inline for nested objects (depth-limited by resolveSchema). */
function renderSchemaTable(spec, schema, depth = 0) {
  if (!schema) return '_No schema._\n';
  let target = schema;
  let refLabel = null;
  if (schema.$ref) {
    refLabel = refName(schema.$ref);
    target = spec.components?.schemas?.[refLabel];
    if (!target) return `_Schema ${code(refLabel)} not found in the spec._\n`;
  }
  if (target.allOf) {
    const merged = { type: 'object', properties: {}, required: [] };
    for (const sub of target.allOf) {
      let s = sub;
      if (s.$ref) s = spec.components?.schemas?.[refName(s.$ref)] ?? {};
      if (s.properties) Object.assign(merged.properties, s.properties);
      if (s.required) merged.required.push(...s.required);
    }
    target = merged;
  }
  if (target.type === 'array') {
    const itemsLabel = scalarLabel(target.items ?? {});
    let out = `Array of ${itemsLabel}.\n`;
    if (target.items?.$ref && depth < MAX_SCHEMA_DEPTH) {
      out += '\n' + renderSchemaTable(spec, target.items, depth + 1);
    }
    return out;
  }
  if (!target.properties) {
    return `Type: ${code(scalarLabel(target))}${target.description ? ' — ' + escInline(target.description) : ''}\n`;
  }
  const required = new Set(target.required ?? []);
  const rows = Object.entries(target.properties).map(([name, propSchema]) => {
    const req = required.has(name) ? 'yes' : 'no';
    const type = scalarLabel(propSchema);
    const desc = escInline(propSchema.description ?? '');
    return `| ${code(name)} | ${escInline(type)} | ${req} | ${desc} |`;
  });
  let out = [
    '| Field | Type | Required | Description |',
    '| --- | --- | --- | --- |',
    ...rows,
  ].join('\n') + '\n';

  // One extra level of nested-object/array-of-ref tables, depth-limited.
  if (depth < MAX_SCHEMA_DEPTH) {
    for (const [name, propSchema] of Object.entries(target.properties)) {
      const nestedRef = propSchema.$ref ?? (propSchema.type === 'array' && propSchema.items?.$ref);
      if (nestedRef) {
        const nestedName = refName(nestedRef);
        out += `\n<details>\n<summary>${code(name)} fields (${escInline(nestedName)})</summary>\n\n`;
        out += renderSchemaTable(
          spec,
          propSchema.$ref ? propSchema : propSchema.items,
          depth + 1,
        );
        out += '\n</details>\n';
      }
    }
  }
  return out;
}

function renderParameters(spec, parameters) {
  if (!parameters?.length) return null;
  const byLocation = { path: [], query: [], header: [] };
  for (const p of parameters) {
    (byLocation[p.in] ??= []).push(p);
  }
  let out = '';
  for (const [loc, label] of [
    ['path', 'Path parameters'],
    ['query', 'Query parameters'],
    ['header', 'Header parameters'],
  ]) {
    const params = byLocation[loc];
    if (!params?.length) continue;
    out += `**${label}**\n\n`;
    out += '| Name | Type | Required | Description |\n| --- | --- | --- | --- |\n';
    for (const p of params) {
      const type = scalarLabel(p.schema ?? {});
      out += `| ${code(p.name)} | ${escInline(type)} | ${p.required ? 'yes' : 'no'} | ${escInline(p.description ?? '')} |\n`;
    }
    out += '\n';
  }
  return out;
}

function renderRequestBody(spec, requestBody) {
  if (!requestBody) return null;
  const content = requestBody.content ?? {};
  const mediaType = content['application/json'] ? 'application/json' : Object.keys(content)[0];
  if (!mediaType) return null;
  const schema = content[mediaType].schema;
  let out = `**Request body** (${code(mediaType)}${requestBody.required ? ', required' : ', optional'})\n\n`;
  out += renderSchemaTable(spec, schema);
  const example = content[mediaType].example;
  if (example) {
    out += '\nExample:\n\n```json\n' + JSON.stringify(example, null, 2) + '\n```\n';
  }
  return out;
}

function renderResponses(spec, responses) {
  if (!responses) return '_No documented responses._\n';
  let out = '';
  for (const [status, resp] of Object.entries(responses)) {
    out += `#### ${status} — ${escInline(resp.description ?? '')}\n\n`;
    const content = resp.content?.['application/json'];
    if (content?.schema) {
      out += renderSchemaTable(spec, content.schema);
      if (content.example) {
        out += '\nExample:\n\n```json\n' + JSON.stringify(content.example, null, 2) + '\n```\n';
      }
    } else if (!content) {
      out += '_No body._\n';
    }
    out += '\n';
  }
  return out;
}

function renderOperation(spec, method, opPath, op) {
  // No explicit `{#anchor}` heading id: fumadocs-mdx's default MDX pipeline
  // parses a literal `{...}` after a heading as an MDX/JS expression (it is
  // not wired with a heading-id remark plugin here), so this would otherwise
  // fail the build on any path containing a curly-brace parameter. The
  // heading text itself is enough to find an operation on its tag page.
  let out = `## ${method.toUpperCase()} ${escInline(opPath)}\n\n`;
  if (op.summary) out += `${escInline(op.summary)}\n\n`;
  if (op.description && op.description !== op.summary) out += `${escInline(op.description)}\n\n`;
  out += `- **Operation ID:** ${code(op.operationId ?? '(none)')}\n`;
  out += `- **Auth:** ${op.security?.length ? 'Bearer JWT required (`Authorization: Bearer <token>`)' : 'None'}\n`;
  if (op.deprecated) out += `- **Deprecated.**\n`;
  out += '\n';

  const params = renderParameters(spec, op.parameters);
  if (params) out += params;

  const body = renderRequestBody(spec, op.requestBody);
  if (body) out += body + '\n';

  out += '**Responses**\n\n';
  out += renderResponses(spec, op.responses);

  return out + '\n---\n\n';
}

function renderOverview(spec) {
  const title = spec.info?.title ?? 'StockLink API';
  const version = spec.info?.version ?? '0.0.0';
  const pathCount = Object.keys(spec.paths ?? {}).length;
  let opCount = 0;
  const tagCounts = {};
  for (const p of Object.values(spec.paths ?? {})) {
    for (const op of Object.values(p)) {
      if (!op || typeof op !== 'object' || !op.operationId) continue;
      opCount++;
      for (const t of op.tags ?? ['untagged']) tagCounts[t] = (tagCounts[t] ?? 0) + 1;
    }
  }
  const tagRows = Object.keys(tagCounts)
    .sort()
    .map((t) => `| [${humanizeTag(t)}](${slugForTag(t)}) | ${tagCounts[t]} |`)
    .join('\n');

  return `---
title: "API overview"
description: "Base URLs, authentication, the error envelope, pagination and how to regenerate this reference."
---

Generated from the backend's own OpenAPI document
(\`docs/reference/openapi.json\`, a committed snapshot — see
[Regenerating this reference](#regenerating-this-reference)). **${escInline(title)}**, spec version \`${escInline(version)}\`, **${pathCount} paths / ${opCount} operations** across **${Object.keys(tagCounts).length} tags**.

## Base URLs

| Environment | Browser-reachable base URL |
| --- | --- |
| Dev (through the gateway) | \`http://localhost:3100/api/v1\` |
| Dev (direct, bypassing the gateway) | \`http://localhost:8180/v1\` |
| Staging / production | The environment's gateway origin plus \`/api/v1\` — see [Environments](/docs/operations/environments) |

Interactive Swagger UI (development only): \`http://localhost:8180/docs\`. Raw
document: \`http://localhost:8180/v1/openapi.json\`.

## Authentication

Every account signs in with a one-time code, then holds a bearer JWT:

1. \`POST /v1/auth/otp/request\` — phone (\`sms\`) or email (\`email\`) channel. In
   dev only, the response can echo \`dev_code\` (\`OTP_DEV_ECHO_ENABLED=true\`);
   staging and production never expose it.
2. \`POST /v1/auth/otp/verify\` — the code, plus the channel/identifier — returns
   an access token and a refresh token.
3. Send \`Authorization: Bearer <access token>\` on every authenticated request.
4. \`POST /v1/auth/token/refresh\` exchanges a refresh token for a new access
   token.

Either channel is available to any account; phone/SMS is optional, email is
always available. The **Auth** line on each operation below reflects the
OpenAPI \`security\` requirement recorded for that exact operation in the
spec snapshot — not every route needs a token (health checks, browsing the
catalogue, and requesting an OTP do not).

## Error envelope

Every error response uses one shape:

\`\`\`json
{ "error": { "code": "VALIDATION", "message": "quantity_kg must be positive", "field": "quantity_kg" } }
\`\`\`

\`field\` is present only for validation errors tied to one request field. See
[contract 06](/docs/contracts-and-design/06-validation-and-error-codes) for the
error-code catalogue.

## Pagination

List endpoints that page accept \`page\` (1-based) and \`per_page\` query
parameters and return:

\`\`\`json
{ "items": [ ... ], "page": 1, "per_page": 50, "total": 137 }
\`\`\`

Defaults and maximums differ per endpoint (see each operation's query
parameters below); most default to 50 and cap at 100.

## Rate limits

A coarse, Redis-backed limiter applies to every route (\`RATE_LIMIT_ENABLED\`,
default 120 requests/minute per client). Three sensitive, expensive or
abusable routes carry an additional per-account limit on top of it:
\`POST /v1/quality/assess\`, \`POST /v1/sync/push\`, and
\`POST /v1/finance/applications\`. A rejected request returns \`429\`. See
\`backend/README.md\`'s environment-variable table for the exact
requests-per-minute and window settings.

## Tags

| Tag | Operations |
| --- | --- |
${tagRows}

## Regenerating this reference

\`\`\`bash
# 1. With the dev API running (docker compose -f docker/compose.dev.yml up -d --build):
node docs-site/scripts/fetch-openapi.mjs        # refreshes docs/reference/openapi.json
# 2. Review the diff in docs/reference/openapi.json, then:
node docs-site/scripts/sync-content.mjs         # regenerates these pages (also runs on dev/build)
\`\`\`

\`fetch-openapi.mjs\` accepts \`OPENAPI_URL\` to point at a different running
API. \`sync-content.mjs\` calls \`generate-api-reference.mjs\` automatically, so
\`pnpm dev\` / \`pnpm build\` always render whatever \`openapi.json\` last
committed.
`;
}

function slugForTag(tag) {
  return tag.toLowerCase().replace(/[^a-z0-9]+/g, '-');
}

function renderTagPage(spec, tag, operations) {
  const title = humanizeTag(tag);
  let body = `---
title: "${title}"
description: "${operations.length} operation${operations.length === 1 ? '' : 's'} under the \\"${tag}\\" tag."
---

${operations.length} operation${operations.length === 1 ? '' : 's'}.

`;
  for (const { method, opPath, op } of operations) {
    body += renderOperation(spec, method, opPath, op);
  }
  return body;
}

function main() {
  if (!fs.existsSync(specPath)) {
    console.warn(
      `generate-api-reference: no snapshot at ${path.relative(process.cwd(), specPath)} — ` +
        'run fetch-openapi.mjs first. Skipping API reference generation.',
    );
    return;
  }
  const spec = JSON.parse(fs.readFileSync(specPath, 'utf8'));

  fs.rmSync(outDir, { recursive: true, force: true });
  fs.mkdirSync(outDir, { recursive: true });

  const byTag = new Map();
  for (const [opPath, methods] of Object.entries(spec.paths ?? {})) {
    for (const [method, op] of Object.entries(methods)) {
      if (!op || typeof op !== 'object' || !op.operationId) continue;
      for (const tag of op.tags?.length ? op.tags : ['untagged']) {
        if (!byTag.has(tag)) byTag.set(tag, []);
        byTag.get(tag).push({ method, opPath, op });
      }
    }
  }

  fs.writeFileSync(path.join(outDir, 'index.mdx'), renderOverview(spec));

  const tagSlugs = [];
  for (const [tag, operations] of [...byTag.entries()].sort(([a], [b]) => a.localeCompare(b))) {
    operations.sort((a, b) => (a.opPath + a.method).localeCompare(b.opPath + b.method));
    const slug = slugForTag(tag);
    tagSlugs.push(slug);
    fs.writeFileSync(path.join(outDir, `${slug}.mdx`), renderTagPage(spec, tag, operations));
  }

  fs.writeFileSync(
    path.join(outDir, 'meta.json'),
    JSON.stringify({ title: 'API reference', pages: ['index', ...tagSlugs] }, null, 2) + '\n',
  );

  const opCount = [...byTag.values()].reduce((sum, ops) => sum + ops.length, 0);
  console.log(
    `generated API reference: ${Object.keys(spec.paths ?? {}).length} paths, ${opCount} tagged operations, ${byTag.size} tags -> ${path.relative(process.cwd(), outDir)}`,
  );
}

main();
