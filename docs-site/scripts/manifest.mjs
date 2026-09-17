/**
 * Explicit source -> site mapping for the documentation site.
 *
 * This is the single place that decides what gets published and where.
 * `sync-content.mjs` walks this manifest, not `docs/**` wholesale — nothing
 * is published unless it's named here.
 *
 * `docs/WORK_ORDERS.md` and `docs/STATUS.md` are deliberately NOT published
 * here — they're internal engineering records, not product documentation.
 * Add sections here only for real product documentation (Getting started,
 * Architecture, API guides, Operations, Design system); don't add a section
 * by pointing it at either of those two files.
 */

/** The root `/docs` landing page. */
export const ROOT_INDEX = { src: 'docs/VISION.md', title: 'Start here' };

export const SECTIONS = [];
