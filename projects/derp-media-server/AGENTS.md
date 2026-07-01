# Agent Notes

- Use `npm run check:closkell` for Closkell checks.
- After larger changes, run `npm run test:batch`.
- The UI is migrating to **Closkell** under [`src/features/`](src/features/) with Vite ([`vite.config.mjs`](vite.config.mjs)) and Tailwind ([`src/globals.css`](src/globals.css)).
- Prefer the Closkell web framework app shape: pure state/update/view functions, `#html` templates, commands, and subscriptions.
- Shared view must not use admin-only routes; share flows stay scoped by `shareToken`.
- When adding e2e tests, keep files independent so they can run in parallel without ordering assumptions.
- `test:batch` sets `BATCH_ID`; Playwright uses **4 workers** (parallel **files**; `fullyParallel: false` keeps tests inside a file ordered). Local `npm run test` uses **1 worker** for easier debugging.

## Commands

- **Dev:** `npm run dev` - Closkell Fastify entrypoint + Vite middleware ([`server/main.clsk`](server/main.clsk)).
- **Production:** `npm run build` then `npm run start` (static `dist/client`).
- **E2E:** `npm run test` or `npm run test:batch` - specs in [`tests/e2e/`](tests/e2e/), config [`playwright.config.ts`](playwright.config.ts), batches in [`tests/run-batches.mjs`](tests/run-batches.mjs).

## Closkell Patterns

- Do not add non-Node JavaScript runtimes, framework compatibility layers, or separate linter/formatter dependencies.
- Keep browser effects behind Closkell commands/subscriptions or narrowly scoped runtime helpers.
- Don't write useless comments.
- Keep at most **6** e2e batches in `run-batches.mjs` when extending CI.

For framework docs, see [`docs/web-framework-design.md`](../../docs/web-framework-design.md).
