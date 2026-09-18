# Design system

The living reference for StockLink's visual language: what it is today, why
it changed, and what's planned. See `web/src/styles.css` for the actual
token definitions — this file explains them, it doesn't duplicate them.

## Current pass: professional neobrutalism (2026-09-18)

The look is still neobrutalism — thick black borders, hard offset shadows
(never blurred, never a gradient), bold flat accent colours, no gradients
anywhere (`AGENTS.md` rule 8) — but tuned toward a business tool rather than
a consumer app:

- **One signature accent, not a cycling palette.** KPI cards and
  marketplace product cards used to cycle through blue/orange/pink tinted
  backgrounds per card (`nth-child` rules). That read as a consumer/toy
  app. Cards are now uniform white; colour is reserved for the icon chip,
  the status pill, and the one signature accent below — not the whole
  surface.
- **The signature accent is emerald (`--acid: #34d399`), not neon lime
  (`#c9ff4a`).** Same role in the CSS (nav active state, brand mark,
  avatar, primary "accent" pill, progress fill, focus/hover highlights) —
  only the hue changed. Emerald reads as considered/premium rather than
  playful; it still passes high-contrast black-text-on-accent and
  accent-text-on-black in both directions (both combinations sit above
  10:1 contrast — see the token comment in `styles.css` if that math needs
  rechecking after a future colour change).
- **Tighter geometry.** Corner radii and shadow offsets are one size down
  across the board:

  | Token | Before | Now |
  | --- | --- | --- |
  | `--radius-lg` | 26px | 16px |
  | `--radius` | 18px | 11px |
  | `--radius-sm` | 12px | 8px |
  | `--bw` / `--bw-lg` | 2px / 3px | 1.5px / 2px |
  | `--pop` | 4px 4px | 3px 3px |
  | `--pop-sm` | 2px 2px | 1.5px 1.5px |
  | `--pop-lg` | 7px 7px | 5px 5px |

  Still unmistakably neobrutalist (visible border, hard offset shadow, no
  blur) — just a crisper, less bubbly throw.

**Not yet updated:** the screenshots in `docs/ONBOARDING.md` and this
README predate this pass and show the old, more colourful look. New
captures are needed — see `docs/HANDOFF.md`'s open items. (Also unresolved:
this session's tooling had no mechanism to export the Browser-pane preview
as a saved PNG file, only to display it live — that's why this pass shipped
without new screenshots attached, not because it wasn't visually verified.)

## Planned: dark mode + a settings page

Not started. Scoping it here so it's a deliberate follow-up, not a surprise
mid-implementation.

**Why it's not just "invert everything":** every token in `:root` right now
assumes one cream/ink relationship (near-black text on a warm off-white
background, black borders and shadows that only read correctly against a
light surface). A shadow that's `var(--ink)` (near-black) offset under a
dark card would be nearly invisible — dark mode needs its own shadow colour
too, not just swapped background/text tokens.

**Planned approach:**

1. **Token layer.** Keep every existing `--token` name (nothing that
   consumes `var(--x)` should need to change) but scope a second set of
   values under `:root[data-theme="dark"]` — background/surface/text/line
   tokens definitely need new values; re-examine whether the shadow colour
   token (currently hardcoded `var(--ink)` inside `--pop`/`--pop-sm`/`--pop-lg`)
   needs to become its own `--shadow-color` token so dark mode can point it
   at something visible (e.g. a light grey or the accent colour) instead of
   near-invisible near-black-on-near-black.
2. **A settings screen.** Doesn't exist yet — `App.tsx`'s nav today is
   Dashboard/Catalogue/Orders/Notifications (warehouse) or
   Marketplace/Cart/Orders/Notifications (store). A "Settings" entry needs
   a spot in both, and a screen with (at minimum) a theme toggle. Where the
   preference lives is an open question: `localStorage` alone is simplest
   but doesn't follow the account across devices; a real per-account
   preference needs a new field somewhere in identity's account model and
   an API round-trip — worth deciding deliberately rather than defaulting
   to whichever is fastest to build.
3. **Contrast audit.** Every colour pairing in `styles.css` (accent-on-card,
   muted-text-on-soft-tint, status pill colours, etc.) needs re-checking
   against the dark values once they exist — a pairing that clears 4.5:1
   in light mode doesn't automatically clear it once the background
   changes. WO-05's original acceptance criteria already named "light and
   dark themes, contrast-checked pairs (≥ 4.5:1)" — this is that work,
   just not done when WO-05 was otherwise completed.
4. **`prefers-color-scheme`.** Decide whether dark mode should also
   auto-apply from the OS preference on first visit (before any explicit
   toggle), or only ever activate from the settings screen. Either is
   reasonable; pick one instead of leaving it implicit.

Not estimated as a work order yet — reasonable to size once someone commits
to the per-account-vs-localStorage question above, since that changes
whether this also touches `identity`'s schema.
