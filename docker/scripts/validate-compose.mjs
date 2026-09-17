#!/usr/bin/env node
// StockLink Compose guard — no new dependencies, Node's own `child_process`
// + `fs` only.
//
// Renders demo/dev/staging/prod through `docker compose ... config --format
// json` and asserts, across ALL FOUR files at once:
//
//   1. every container_name is globally unique;
//   2. every service carries every required label, with the right
//      io.stocklink.environment value for its file;
//   3. the Nginx gateway config used by each env has /, /api/ and /docs
//      routes (skips files with no gateway service — none currently);
//   4. every service that is a "data" service (db/redis/kafka) sits only on
//      network(s) marked internal: true, OR (dev only) is documented as a
//      deliberate exception;
//   5. every stateful service has a named volume;
//   6. every long-running service (not a one-shot `migrate`/`seed` job) has
//      a healthcheck;
//   7. staging/prod have no bind mount whose source is a path inside this
//      repository (a real secret/config file mount is fine; source code is
//      not);
//   8. OTP_DEV_ECHO_ENABLED is "false" wherever it is set in staging/prod.
//
// Usage:
//   node docker/scripts/validate-compose.mjs
//
// Exit code 0 = all assertions passed. Exit code 1 = at least one failed
// (every failure is printed, not just the first).
import { execFileSync } from 'node:child_process';
import path from 'node:path';
import fs from 'node:fs';
import { fileURLToPath } from 'node:url';

const here = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(here, '../..');
const dockerDir = path.join(repoRoot, 'docker');
const validationEnv = path.join(dockerDir, 'env', 'validation.env');

const ENVS = [
  { name: 'demo', file: 'compose.demo.yml', requiresEnvFile: false, dataInternal: false },
  { name: 'dev', file: 'compose.dev.yml', requiresEnvFile: false, dataInternal: false },
  { name: 'staging', file: 'compose.staging.yml', requiresEnvFile: true, dataInternal: true },
  { name: 'prod', file: 'compose.prod.yml', requiresEnvFile: true, dataInternal: true },
];

const REQUIRED_LABELS = [
  'app.kubernetes.io/name',
  'app.kubernetes.io/instance',
  'app.kubernetes.io/component',
  'app.kubernetes.io/part-of',
  'app.kubernetes.io/version',
  'app.kubernetes.io/managed-by',
  'io.stocklink.environment',
  'io.stocklink.service',
  'io.stocklink.owner',
  'io.stocklink.data-classification',
  'org.opencontainers.image.revision',
];

// Services that are intentionally one-shot jobs, not long-running processes,
// so the "every service has a healthcheck" assertion does not apply to them.
const ONE_SHOT_SERVICES = new Set(['migrate', 'seed']);

// Services recognised as holding this environment's state, so the "every
// stateful service has a named volume" assertion applies to them.
const STATEFUL_SERVICE_PREFIXES = ['db', 'redis', 'kafka', 'postgres'];

const failures = [];
function fail(scope, message) {
  failures.push(`[${scope}] ${message}`);
}

function runConfig(envFile, withEnvFile) {
  const args = ['compose'];
  if (withEnvFile) args.push('--env-file', validationEnv);
  args.push('-f', envFile, 'config', '--format', 'json');
  return execFileSync('docker', args, { cwd: repoRoot, encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] });
}

function loadRendered(env) {
  const filePath = path.join(dockerDir, env.file);
  try {
    const json = runConfig(filePath, env.requiresEnvFile);
    return JSON.parse(json);
  } catch (err) {
    fail(env.name, `docker compose config failed to render: ${err.message}`);
    return null;
  }
}

// ── fail-closed proof: staging/prod config WITHOUT the validation env must
// exit non-zero and name a missing variable ──────────────────────────────
function assertFailsClosedWithoutEnv(env) {
  const filePath = path.join(dockerDir, env.file);
  try {
    execFileSync('docker', ['compose', '-f', filePath, 'config', '--quiet'], {
      cwd: repoRoot,
      encoding: 'utf8',
      stdio: ['ignore', 'pipe', 'pipe'],
    });
    fail(env.name, 'docker compose config succeeded WITHOUT required secrets — staging/prod must fail closed');
  } catch (err) {
    const stderr = String(err.stderr ?? '');
    if (!/required variable .* is missing a value/.test(stderr)) {
      fail(env.name, `failed closed, but without the expected "required variable ... is missing a value" message (got: ${stderr.slice(0, 200)})`);
    }
  }
}

// ── 1. unique container names across ALL files ────────────────────────────
const containerNamesSeen = new Map(); // name -> [envs]
function collectContainerNames(env, rendered) {
  for (const [svcName, svc] of Object.entries(rendered.services ?? {})) {
    const cname = svc.container_name;
    if (!cname) {
      fail(env.name, `service "${svcName}" has no container_name`);
      continue;
    }
    const expectedPrefix = `stocklink-${env.name}-`;
    if (!cname.startsWith(expectedPrefix)) {
      fail(env.name, `service "${svcName}" container_name "${cname}" does not start with "${expectedPrefix}"`);
    }
    const owners = containerNamesSeen.get(cname) ?? [];
    owners.push(`${env.name}/${svcName}`);
    containerNamesSeen.set(cname, owners);
  }
}

// ── 2. labels ───────────────────────────────────────────────────────────
function normalizeLabels(labels) {
  if (!labels) return {};
  if (Array.isArray(labels)) {
    const out = {};
    for (const entry of labels) {
      const idx = entry.indexOf('=');
      if (idx === -1) out[entry] = '';
      else out[entry.slice(0, idx)] = entry.slice(idx + 1);
    }
    return out;
  }
  return labels;
}

function assertLabels(env, rendered) {
  for (const [svcName, svc] of Object.entries(rendered.services ?? {})) {
    const labels = normalizeLabels(svc.labels);
    for (const required of REQUIRED_LABELS) {
      if (!(required in labels) || labels[required] === '') {
        fail(env.name, `service "${svcName}" is missing required label "${required}"`);
      }
    }
    if (labels['io.stocklink.environment'] && labels['io.stocklink.environment'] !== env.name) {
      fail(
        env.name,
        `service "${svcName}" io.stocklink.environment="${labels['io.stocklink.environment']}", expected "${env.name}"`,
      );
    }
  }
}

// ── 3. gateway routes exist in the Nginx config used by this env ──────────
function assertGatewayRoutes(env, rendered) {
  const gateway = rendered.services?.gateway;
  if (!gateway) {
    fail(env.name, 'no "gateway" service defined');
    return;
  }
  // Documentation (Fumadocs) runs only in dev. Dev builds its gateway from
  // gateway.dev.conf (proxies /docs); every other environment uses
  // gateway.conf, which has no docs upstream and answers /docs with 404.
  const isDev = env.name === 'dev';
  const hasDocs = Boolean(rendered.services?.docs);
  if (isDev && !hasDocs) fail(env.name, 'dev must run the "docs" service');
  if (!isDev && hasDocs) fail(env.name, 'documentation must run only in dev; remove the "docs" service');
  const buildArg = gateway.build?.args?.GATEWAY_CONF;
  const confFile = buildArg ?? 'docker/nginx/gateway.conf';
  if (isDev && confFile !== 'docker/nginx/gateway.dev.conf') {
    fail(env.name, `dev gateway must build with GATEWAY_CONF=docker/nginx/gateway.dev.conf (got ${confFile})`);
  }
  if (!isDev && confFile !== 'docker/nginx/gateway.conf') {
    fail(env.name, `gateway must build with docker/nginx/gateway.conf (got ${confFile})`);
  }
  const confPath = path.join(repoRoot, confFile);
  if (!fs.existsSync(confPath)) {
    fail(env.name, `gateway conf not found at ${confPath}`);
    return;
  }
  const conf = fs.readFileSync(confPath, 'utf8');
  for (const route of ['location / {', 'location /api/ {', 'location /docs {']) {
    if (!conf.includes(route)) {
      fail(env.name, `gateway conf is missing route "${route}"`);
    }
  }
  const proxiesDocs = /proxy_pass\s+http:\/\/docs:/.test(conf);
  if (isDev && !proxiesDocs) fail(env.name, 'dev gateway conf must proxy /docs to the docs service');
  if (!isDev && proxiesDocs) fail(env.name, 'gateway conf outside dev must not proxy to a docs service');
  if (!/location[^{]*metrics[^{]*\{\s*return 404;/.test(conf)) {
    fail(env.name, 'gateway conf does not block /metrics with a 404');
  }
}

// ── 4. data services only on internal networks ─────────────────────────────
function assertDataServiceNetworks(env, rendered) {
  const networks = rendered.networks ?? {};
  for (const [svcName, svc] of Object.entries(rendered.services ?? {})) {
    const isDataService = STATEFUL_SERVICE_PREFIXES.some((p) => svcName === p || svcName.startsWith(p));
    if (!isDataService) continue;
    const svcNetworks = Array.isArray(svc.networks) ? svc.networks : Object.keys(svc.networks ?? {});
    const onGateway = svcNetworks.includes('edge');
    if (onGateway) {
      fail(env.name, `data service "${svcName}" is attached to the "edge" (gateway) network`);
    }
    if (env.dataInternal) {
      const allInternal = svcNetworks.every((n) => networks[n]?.internal === true);
      if (!allInternal) {
        fail(env.name, `data service "${svcName}" is not confined to an internal:true network (networks: ${svcNetworks.join(', ')})`);
      }
      if (svc.ports?.length) {
        fail(env.name, `data service "${svcName}" publishes host ports in an environment where the data network must be internal`);
      }
    }
  }
}

// ── 5. named volumes for stateful services ─────────────────────────────────
function assertNamedVolumes(env, rendered) {
  const topVolumes = new Set(Object.keys(rendered.volumes ?? {}));
  for (const [svcName, svc] of Object.entries(rendered.services ?? {})) {
    const isDataService = STATEFUL_SERVICE_PREFIXES.some((p) => svcName === p || svcName.startsWith(p));
    if (!isDataService) continue;
    const volumes = svc.volumes ?? [];
    const hasNamedVolume = volumes.some((v) => {
      const source = typeof v === 'string' ? v.split(':')[0] : v.source;
      return source && topVolumes.has(source);
    });
    if (!hasNamedVolume) {
      // prod intentionally has no local db/redis (external managed stores) —
      // only flag this when the service actually exists in the file.
      fail(env.name, `stateful service "${svcName}" has no named top-level volume`);
    }
  }
}

// ── 6. healthchecks on every long-running service ──────────────────────────
function assertHealthchecks(env, rendered) {
  for (const [svcName, svc] of Object.entries(rendered.services ?? {})) {
    if (ONE_SHOT_SERVICES.has(svcName)) continue;
    if (!svc.healthcheck || svc.healthcheck.disable) {
      fail(env.name, `long-running service "${svcName}" has no healthcheck`);
      continue;
    }
    // Alpine images resolve `localhost` to ::1 first, and busybox wget does
    // not fall back to IPv4. nginx and Next.js listen on IPv4 only, so a
    // `localhost` probe reports a running service as unhealthy (found in the
    // 13 September 2026 cold start). Probe 127.0.0.1 instead.
    const test = [svc.healthcheck.test ?? []].flat().join(' ');
    if (/https?:\/\/localhost[:/]/.test(test)) {
      fail(env.name, `service "${svcName}" healthcheck probes localhost; use 127.0.0.1 (${test})`);
    }
  }
}

// ── 7. no source-code bind mounts in staging/prod ──────────────────────────
function assertNoSourceMounts(env, rendered) {
  if (env.name !== 'staging' && env.name !== 'prod') return;
  for (const [svcName, svc] of Object.entries(rendered.services ?? {})) {
    for (const v of svc.volumes ?? []) {
      const source = typeof v === 'string' ? v.split(':')[0] : v.source;
      if (!source) continue; // anonymous/named volume, not a bind mount
      const isNamedVolume = Object.keys(rendered.volumes ?? {}).includes(source);
      if (isNamedVolume) continue;
      const isAbsoluteHostPath = source.startsWith('/') || /^[A-Za-z]:[\\/]/.test(source);
      if (isAbsoluteHostPath) {
        fail(env.name, `service "${svcName}" bind-mounts a host path "${source}" — staging/prod must not mount source code`);
      }
    }
    if (svc.build) {
      // build: is fine (produces an immutable image); a bind-mounted source
      // tree at runtime is the thing this assertion forbids. Nothing further
      // to check here beyond the volumes loop above.
    }
  }
}

// ── 8. OTP_DEV_ECHO_ENABLED is false in staging/prod ────────────────────────
function assertOtpEchoDisabled(env, rendered) {
  if (env.name !== 'staging' && env.name !== 'prod') return;
  for (const [svcName, svc] of Object.entries(rendered.services ?? {})) {
    const value = svc.environment?.OTP_DEV_ECHO_ENABLED;
    if (value === undefined) continue;
    if (String(value) !== 'false') {
      fail(env.name, `service "${svcName}" has OTP_DEV_ECHO_ENABLED="${value}", must be "false"`);
    }
  }
}

for (const env of ENVS) {
  if (env.requiresEnvFile) assertFailsClosedWithoutEnv(env);
  const rendered = loadRendered(env);
  if (!rendered) continue;
  collectContainerNames(env, rendered);
  assertLabels(env, rendered);
  assertGatewayRoutes(env, rendered);
  assertDataServiceNetworks(env, rendered);
  assertNamedVolumes(env, rendered);
  assertHealthchecks(env, rendered);
  assertNoSourceMounts(env, rendered);
  assertOtpEchoDisabled(env, rendered);
}

for (const [name, owners] of containerNamesSeen) {
  if (owners.length > 1) {
    fail('cross-env', `container_name "${name}" is used by more than one service: ${owners.join(', ')}`);
  }
}

if (failures.length > 0) {
  console.error(`Compose guard FAILED (${failures.length} issue${failures.length === 1 ? '' : 's'}):`);
  for (const f of failures) console.error(`  - ${f}`);
  process.exit(1);
}

console.log('Compose guard passed: demo/dev/staging/prod all render, names are unique, labels/routes/networks/volumes/healthchecks/secrets/OTP-echo all check out.');
process.exit(0);
