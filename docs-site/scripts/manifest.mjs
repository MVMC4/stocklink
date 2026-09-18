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

export const SECTIONS = [
  {
    slug: 'getting-started',
    title: 'Getting started',
    pages: [{ src: 'docs/ONBOARDING.md', slug: 'index', title: 'Getting started' }],
  },
  {
    slug: 'architecture',
    title: 'Architecture',
    pages: [{ src: 'docs/ARCHITECTURE.md', slug: 'index', title: 'Architecture' }],
  },
  {
    slug: 'design-system',
    title: 'Design system',
    pages: [{ src: 'docs/DESIGN_SYSTEM.md', slug: 'index', title: 'Design system' }],
  },
];
