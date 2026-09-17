#!/usr/bin/env node
// Health-gated rolling restart for one StockLink service, on plain Docker
// Compose (no Kubernetes/Swarm) — no downtime, no double-running the whole
// stack (contrast with blue-green).
//
// How it works:
//   1. Build the service's image from the current source tree.
//   2. Scale the service to 2 replicas WITHOUT recreating the existing one
//      (`--no-recreate`) — Compose starts exactly one new container, on the
//      freshly built image, alongside the old one, which keeps serving
//      traffic the whole time. This is why the four scalable services in
//      docker/compose.dev.yml have no `container_name` and no fixed host
//      `ports:` (see that file's comments) — both are required for two
//      containers of the same service to coexist at all.
//   3. Poll the *new* container's own Docker healthcheck
//      (`docker inspect .State.Health.Status`) until it reports "healthy"
//      or a timeout elapses. The healthcheck (defined per-service in
//      compose.dev.yml) hits that container's own `/health/ready` —
//      real readiness, not just "the process started".
//   4. The gateway (docker/nginx/gateway.dev.conf) resolves each service's
//      Docker DNS name through Docker's embedded resolver (127.0.0.11) with
//      a 5s TTL, using a `set $upstream ...; proxy_pass $upstream;` — not a
//      static upstream block — so it picks up the new container as soon as
//      Docker's DNS returns its IP too, without an nginx reload. Docker
//      Compose gives every replica of a service the same DNS name and
//      returns all of them, so during step 3 the old and new containers
//      both receive traffic; nothing is dropped.
//   5. Once healthy, stop and remove the *old* container specifically (by
//      the container ID captured before scaling — never "the other one" by
//      guesswork). Docker's DNS stops returning its IP immediately; nginx's
//      next re-resolution (within 5s) stops sending it traffic.
//   6. On failure (build error, or the new container never turns healthy
//      within the timeout), the new container is removed instead and the
//      old one is left running untouched — a rolling update never leaves
//      the service on zero healthy replicas, and a failed update is a
//      no-op from the outside.
//
// Usage:
//   node scripts/rolling-update.mjs <identity|commerce|notifications|media> [--timeout-secs=120]
//
// Requires the stack already running via:
//   docker compose -f docker/compose.dev.yml up -d

import { execFileSync } from 'node:child_process';

const COMPOSE_FILE = 'docker/compose.dev.yml';
const SCALABLE_SERVICES = ['identity', 'commerce', 'notifications', 'media'];

function run(cmd, args, opts = {}) {
  return execFileSync(cmd, args, { encoding: 'utf8', ...opts }).trim();
}

function compose(...args) {
  return run('docker', ['compose', '-f', COMPOSE_FILE, ...args]);
}

function containerIds(service) {
  const out = compose('ps', '-q', service);
  return out ? out.split('\n').filter(Boolean) : [];
}

function healthStatus(containerId) {
  try {
    return run('docker', ['inspect', '--format', '{{.State.Health.Status}}', containerId]);
  } catch {
    return 'unknown';
  }
}

async function sleep(ms) {
  await new Promise((resolve) => setTimeout(resolve, ms));
}

async function main() {
  const service = process.argv[2];
  const timeoutArg = process.argv.find((a) => a.startsWith('--timeout-secs='));
  const timeoutSecs = timeoutArg ? Number(timeoutArg.split('=')[1]) : 120;

  if (!SCALABLE_SERVICES.includes(service)) {
    console.error(`usage: node scripts/rolling-update.mjs <${SCALABLE_SERVICES.join('|')}> [--timeout-secs=N]`);
    process.exit(1);
  }

  console.log(`[1/5] building ${service}...`);
  compose('build', service);

  console.log(`[2/5] capturing current container(s) for ${service}...`);
  const before = new Set(containerIds(service));
  if (before.size === 0) {
    console.error(`no running container for "${service}" — start the stack first: docker compose -f ${COMPOSE_FILE} up -d`);
    process.exit(1);
  }
  console.log(`      existing: ${[...before].join(', ')}`);

  console.log(`[3/5] scaling ${service} to 2 (old container is NOT recreated)...`);
  compose('up', '-d', '--no-deps', '--no-recreate', '--scale', `${service}=2`, service);

  const after = containerIds(service);
  const newContainer = after.find((id) => !before.has(id));
  if (!newContainer) {
    console.error('could not identify the new container — aborting without changing anything');
    process.exit(1);
  }
  console.log(`      new container: ${newContainer}`);

  console.log(`[4/5] waiting for ${newContainer} to report healthy (timeout ${timeoutSecs}s)...`);
  const deadline = Date.now() + timeoutSecs * 1000;
  let healthy = false;
  while (Date.now() < deadline) {
    const status = healthStatus(newContainer);
    process.stdout.write(`      health: ${status}\r`);
    if (status === 'healthy') {
      healthy = true;
      break;
    }
    if (status === 'unhealthy') break; // fail fast, don't wait out the full timeout
    await sleep(2000);
  }
  console.log('');

  if (!healthy) {
    console.error(`${newContainer} did not become healthy in time — rolling back (removing the new container, old one untouched)`);
    run('docker', ['rm', '-f', newContainer]);
    process.exit(1);
  }

  console.log('[5/5] new container is healthy — removing the old one(s)...');
  for (const oldId of before) {
    run('docker', ['stop', oldId]);
    run('docker', ['rm', oldId]);
    console.log(`      removed ${oldId}`);
  }

  console.log(`done — ${service} is now running only the updated container (${newContainer}).`);
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
