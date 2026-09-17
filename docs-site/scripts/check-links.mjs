#!/usr/bin/env node
/**
 * Checks relative Markdown links across every git-tracked `*.md` file for a
 * target that does not exist on disk.
 *
 * Scope: only relative links (`[text](path)`, and the target half of
 * reference-style `[label]: path` definitions) that do not start with a
 * scheme (`http:`, `https:`, `mailto:`, etc.) and are not a bare in-page
 * anchor (`#section`). Absolute repo-rooted links (a leading `/`) are treated
 * as web-server paths, not filesystem paths, and are skipped — this repo's
 * Markdown does not use them for cross-file links.
 *
 * A link target is resolved relative to the linking file's own directory.
 * A `#fragment` suffix is stripped before checking the file exists; fragment
 * correctness (whether the heading exists) is not checked. A target with a
 * trailing `/` or that resolves to an existing directory is treated as
 * satisfied if the directory exists (covers links to a folder that has its
 * own README).
 *
 * Pending paths: some links point at files another concurrent workstream is
 * about to add (for example the payments contract under
 * `docs/backend-alignment/payments/` and
 * `docs/backend-alignment/contracts/08-payments-provider-neutral.md`). Rather
 * than an allowlist keyed to today's date, this script reports those
 * separately from hard failures: any target under a path listed in
 * PENDING_PATH_PREFIXES is printed as a pending note and does not affect the
 * exit code. Everything else that is missing is a hard failure.
 *
 * Usage:
 *   node scripts/check-links.mjs
 *
 * Exit code 0: no hard failures (pending notes may still be printed).
 * Exit code 1: at least one hard failure, each printed as `file:line -> target`.
 */
import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const here = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(here, '../..');

// Directories/files that are allowed to not exist yet because a concurrent
// workstream is actively creating them. Reported separately, never fails.
const PENDING_PATH_PREFIXES = [
  'docs/backend-alignment/payments/',
  'docs/backend-alignment/contracts/08-payments-provider-neutral.md',
];

const LINK_RE = /\[[^\]\n]*\]\(([^)\s]+)(?:\s+"[^"]*")?\)/g;
const REF_DEF_RE = /^\s*\[[^\]\n]+\]:\s*(\S+)/;

function isCheckable(target) {
  if (!target) return false;
  if (target.startsWith('#')) return false; // in-page anchor only
  if (target.startsWith('/')) return false; // repo-rooted / web-server path
  if (/^[a-z][a-z0-9+.-]*:/i.test(target)) return false; // scheme, e.g. https:, mailto:
  return true;
}

function stripFragmentAndQuery(target) {
  return target.split('#')[0].split('?')[0];
}

function isPending(relFromRoot) {
  const normalized = relFromRoot.split(path.sep).join('/');
  return PENDING_PATH_PREFIXES.some((prefix) => normalized.startsWith(prefix));
}

function listTrackedMarkdownFiles() {
  const out = execFileSync('git', ['ls-files', '*.md'], {
    cwd: repoRoot,
    encoding: 'utf8',
  });
  return out.split('\n').filter(Boolean);
}

function checkFile(relPath) {
  const absPath = path.join(repoRoot, relPath);
  const dir = path.dirname(absPath);
  const lines = fs.readFileSync(absPath, 'utf8').split('\n');

  const failures = [];
  const pending = [];

  lines.forEach((line, idx) => {
    const lineNo = idx + 1;
    const targets = [];

    for (const m of line.matchAll(LINK_RE)) targets.push(m[1]);
    const refMatch = line.match(REF_DEF_RE);
    if (refMatch) targets.push(refMatch[1]);

    for (const rawTarget of targets) {
      if (!isCheckable(rawTarget)) continue;
      const cleaned = stripFragmentAndQuery(rawTarget);
      if (!cleaned) continue;

      const resolved = path.resolve(dir, decodeURIComponent(cleaned));
      if (fs.existsSync(resolved)) continue;

      const relFromRoot = path.relative(repoRoot, resolved);
      const entry = `${relPath}:${lineNo} -> ${rawTarget}`;
      if (isPending(relFromRoot)) {
        pending.push(entry);
      } else {
        failures.push(entry);
      }
    }
  });

  return { failures, pending };
}

function main() {
  const files = listTrackedMarkdownFiles();
  const allFailures = [];
  const allPending = [];

  for (const relPath of files) {
    const absPath = path.join(repoRoot, relPath);
    if (!fs.existsSync(absPath)) continue; // deleted-but-still-listed edge case
    const { failures, pending } = checkFile(relPath);
    allFailures.push(...failures);
    allPending.push(...pending);
  }

  if (allPending.length > 0) {
    console.log(`Pending (not counted as failures — target owned by a concurrent workstream):`);
    for (const line of allPending) console.log(`  ${line}`);
    console.log('');
  }

  if (allFailures.length > 0) {
    console.error(`Broken relative links found (${allFailures.length}):`);
    for (const line of allFailures) console.error(`  ${line}`);
    process.exit(1);
  }

  console.log(
    `OK — checked ${files.length} tracked Markdown files, no broken relative links` +
      (allPending.length > 0 ? ` (${allPending.length} pending).` : '.'),
  );
  process.exit(0);
}

main();
