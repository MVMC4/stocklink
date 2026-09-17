# StockLink — agent execution contract

Read this first, then `docs/WORK_ORDERS.md` and `docs/VISION.md`, in that order.

## Non-negotiable rules

1. **Provenance.** Write your own code. Never copy, paste, port, paraphrase
   or "reference-implement" anything from elsewhere.
2. **One work order at a time.** Finish, verify and record it before starting
   the next. Record evidence in `docs/STATUS.md` (commands run, results, files
   changed, next work order).
3. **Honesty.** Never fake success. Unbuilt features render as visibly
   unavailable. Demo or sample data is labelled on screen. No invented API paths:
   a route exists only once it is in the OpenAPI document.
4. **Red-test every guard.** A check counts as evidence only after it has been
   shown to fail on a deliberate break and pass once restored.
5. **Secrets.** No secrets, tokens, OTP codes, personal contact details or
   production credentials in code, logs, fixtures, commits or docs. Development
   defaults are documented as development-only.
6. **Server-authoritative.** Price, stock, ownership, order state and
   settlement come from the API, never from browser state.
7. **Design.** Complete minimalism with glass surfaces (translucent solid fills,
   backdrop blur). **No gradients anywhere**, including CSS, SVG and images.
   Smooth, short, eased motion that respects reduced-motion settings.
8. **Git.** Work on a `feat/` or `fix/` branch. No commit or push while a
   required gate is red. No AI, tool or co-author attribution in commits, pull
   requests or docs.
9. **Stop and ask** before anything irreversible (history rewrites, data
   deletion, publishing the repository publicly) or any legal, provider,
   payment or credential decision.

## Required gates

| Area | Commands |
| --- | --- |
| Backend | `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo sqlx prepare --check`, `cargo test` |
| Web | `pnpm typecheck`, `pnpm lint`, `pnpm test`, `pnpm build` |
| Design | no gradients, anywhere, in any form — enforced by not writing one, not by a script; a visual gradient-like effect (e.g. a blurred glow standing in for one) is still a violation even where no literal `gradient(` syntax appears |
| Docs | docs-site build and link check |
