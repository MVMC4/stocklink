#!/usr/bin/env node
/**
 * Builds `content/docs/**` from two sources of truth:
 *
 *  1. `manifest.mjs` — an explicit list of `docs/**` Markdown files this site
 *     publishes, each with its own site section/slug/title/order. Nothing
 *     under `docs/` is published unless it is named in the manifest, so an
 *     agent handoff, work order, or integration manifest dropped into
 *     `docs/` does NOT appear on the site just by existing.
 *  2. `docs/reference/openapi.json` (a committed snapshot of the backend's
 *     OpenAPI document) via `generate-api-reference.mjs`, which builds the
 *     whole "API reference" section.
 *
 * Run automatically before `dev` / `build` (see package.json `predev`/
 * `prebuild`), or on demand with `pnpm sync`.
 *
 * Each source `.md` becomes `.mdx` with a `title` (the manifest entry's
 * title, else the first `# heading`, else the file name) and `description`
 * (first paragraph) in frontmatter; the leading H1 is dropped (Fumadocs
 * renders the frontmatter title). Prose outside code fences is escaped so
 * arbitrary Markdown (`<addr>`, `{channel}`) is valid MDX. `meta.json` files
 * are generated from the manifest's page order.
 *
 * NOT published, deliberately (see manifest.mjs and docs-site/README.md):
 *   - agent handoffs/continuations, work orders, the contract/current-state
 *     baselines, the documentation inventory, the CTO transition handoff,
 *     integration manifests, and SONNET_5_PROMPT.md — internal working
 *     history, not product documentation. They stay in the repo.
 *   - `docs/archive/**` — explicitly historical; see `docs/archive/README.md`.
 *   - `frontend/docs/**` — developer-local notes beside the frontend code;
 *     browse them in the repository. `frontend/README.md`,
 *     `frontend/architecture.md`, and the per-folder `README.md` files
 *     remain the right entry points for engineers working in that tree.
 */
import fs from 'node:fs';
import path from 'node:path';
import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { ROOT_INDEX, SECTIONS } from './manifest.mjs';

const here = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(here, '../..');
const outRoot = path.resolve(here, '../content/docs');

function escapeMdxProse(markdown) {
  const lines = markdown.split('\n');
  let inFence = false;
  return lines
    .map((line) => {
      if (/^\s*(```|~~~)/.test(line)) {
        inFence = !inFence;
        return line;
      }
      if (inFence) return line;
      // Keep leading blockquote markers (`> `, `> > `) as Markdown syntax;
      // escaping them turned every callout into a literal "&gt;" paragraph.
      const quote = line.match(/^\s*(?:>\s?)+/)?.[0] ?? '';
      // Protect inline code spans, then escape the rest.
      const parts = line.slice(quote.length).split(/(`[^`]*`)/);
      return quote + parts
        .map((part, i) =>
          i % 2 === 1
            ? part
            : part
                .replace(/&(?![a-zA-Z#][a-zA-Z0-9]*;)/g, '&amp;')
                .replace(/</g, '&lt;')
                .replace(/>/g, '&gt;')
                .replace(/\{/g, '&#123;')
                .replace(/\}/g, '&#125;'),
        )
        .join('');
    })
    .join('\n');
}

function toFrontmatterDoc(raw, fallbackTitle, titleOverride) {
  const lines = raw.split('\n');
  let title = fallbackTitle;
  let start = 0;
  const h1 = lines.findIndex((l) => /^#\s+/.test(l));
  if (h1 !== -1 && lines.slice(0, h1).every((l) => l.trim() === '')) {
    if (!titleOverride) title = lines[h1].replace(/^#\s+/, '').trim();
    start = h1 + 1;
  }
  if (titleOverride) title = titleOverride;
  const body = lines.slice(start).join('\n').trim();
  const firstPara =
    body
      .split('\n\n')
      .map((p) => p.replace(/\s+/g, ' ').trim())
      .find((p) => p && !p.startsWith('|') && !p.startsWith('#') && !p.startsWith('```')) ?? '';
  const description = firstPara.slice(0, 180).replace(/"/g, "'");
  const fm = ['---', `title: "${title.replace(/"/g, "'")}"`];
  // No `description`: Fumadocs renders it under the title, and it is the page's own
  // first paragraph, so every page showed that paragraph twice.
  void description;
  fm.push('---', '');
  return fm.join('\n') + escapeMdxProse(body) + '\n';
}

function humanize(name) {
  return name
    .replace(/^\d+[-_]/, '')
    .replace(/[-_]/g, ' ')
    .replace(/\.(md|mdx)$/, '')
    .replace(/\b\w/g, (c) => c.toUpperCase());
}

/** Copies one manifest page entry into `outDir/<slug>.mdx`. Returns the slug
 * actually written, or `null` if an optional source is missing. */
function writePage(page, outDir) {
  const srcPath = path.join(repoRoot, page.src);
  if (!fs.existsSync(srcPath)) {
    if (page.optional) {
      console.log(`  (skip, not yet present) ${page.src}`);
      return null;
    }
    throw new Error(
      `manifest.mjs references ${page.src}, which does not exist. Add it, remove the ` +
        `manifest entry, or mark it optional: true if another workstream owns it.`,
    );
  }
  const raw = fs.readFileSync(srcPath, 'utf8');
  const doc = toFrontmatterDoc(raw, humanize(path.basename(page.src)), page.title);
  fs.writeFileSync(path.join(outDir, `${page.slug}.mdx`), doc);
  return page.slug;
}

/** Generates a simple linking index page when a section has no explicit
 * `index` page among its manifest entries. */
function writeGeneratedIndex(outDir, title, pageSlugsInOrder) {
  const indexPath = path.join(outDir, 'index.mdx');
  if (fs.existsSync(indexPath)) return; // an explicit index page was already written
  const links = pageSlugsInOrder
    .filter((slug) => slug !== 'index')
    .map((slug) => `- [${humanize(slug)}](${slug})`)
    .join('\n');
  fs.writeFileSync(
    indexPath,
    `---\ntitle: "${title}"\ndescription: "Section index."\n---\n\n${links}\n`,
  );
}

function writeSection(section, outDir) {
  fs.mkdirSync(outDir, { recursive: true });
  console.log(`section: ${section.slug}`);
  const written = [];
  for (const page of section.pages) {
    const slug = writePage(page, outDir);
    if (slug) written.push(slug);
  }
  writeGeneratedIndex(outDir, section.title, written);
  const orderedPages = written.includes('index')
    ? ['index', ...written.filter((s) => s !== 'index')]
    : written;

  const subsectionSlugs = [];
  for (const sub of section.subsections ?? []) {
    writeSection(sub, path.join(outDir, sub.slug));
    subsectionSlugs.push(sub.slug);
  }

  fs.writeFileSync(
    path.join(outDir, 'meta.json'),
    JSON.stringify({ title: section.title, pages: [...orderedPages, ...subsectionSlugs] }, null, 2) + '\n',
  );
}

// ── reset and rebuild content/docs from the manifest ───────────────────────
fs.rmSync(outRoot, { recursive: true, force: true });
fs.mkdirSync(outRoot, { recursive: true });

const rootIndexPath = path.join(repoRoot, ROOT_INDEX.src);
if (!fs.existsSync(rootIndexPath)) {
  throw new Error(`manifest.mjs ROOT_INDEX references ${ROOT_INDEX.src}, which does not exist.`);
}
fs.writeFileSync(
  path.join(outRoot, 'index.mdx'),
  toFrontmatterDoc(fs.readFileSync(rootIndexPath, 'utf8'), 'Start here', ROOT_INDEX.title),
);

for (const section of SECTIONS) {
  writeSection(section, path.join(outRoot, section.slug));
}

// The API reference section is generated separately from the OpenAPI
// snapshot, not from manifest.mjs.
const generateApiReference = path.join(here, 'generate-api-reference.mjs');
execFileSync(process.execPath, [generateApiReference], { stdio: 'inherit', cwd: here });

// Root nav order. 'reference' (the API reference, generated above from the
// OpenAPI snapshot rather than from a manifest section) is inserted right
// after 'architecture' to match the target hierarchy; every other slug comes
// from SECTIONS in manifest.mjs, in the order declared there.
const afterArchitecture = ['reference'];
const rootPages = ['index'];
for (const section of SECTIONS) {
  rootPages.push(section.slug);
  if (section.slug === 'architecture') rootPages.push(...afterArchitecture);
}
fs.writeFileSync(path.join(outRoot, 'meta.json'), JSON.stringify({ pages: rootPages }, null, 2) + '\n');

console.log(`synced docs -> ${path.relative(process.cwd(), outRoot)}`);
