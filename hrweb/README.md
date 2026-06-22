# HRWeb on Closkell

This is the production HRWeb port driven by Closkell source modules and the no-VDOM runtime.

## Commands

```powershell
npm install
npm run check:closkell
npm run build
npm run test:e2e
npm run verify
npm run dev
```

`npm run check:closkell` is quiet when the app type/effect check succeeds; use
`cargo run --manifest-path ../Cargo.toml -p cli -- check src/app.clsk --types`
inside this directory when you want the inferred form dump while debugging.

Vite imports `/src/app.clsk` directly through the workspace `@closkell/vite-plugin` package. The plugin emits cache ESM under Vite's `node_modules/.vite/closkell/` cache, builds imported Closkell modules beside it, and vendors `@closkell/runtime` into this package before Vite bundles the app. Tailwind is wired through `@tailwindcss/vite`; `src/styles.css` is copied from the Solid HRWeb project, while Tailwind scans the direct `.clsk` Vite module graph for utility classes inside `#html` templates.
The Closkell plugin keeps its vendored runtime out of Vite's optimized-deps cache, so dev preview imports the current runtime source after compiler/runtime edits.

During `npm run dev`, `http://127.0.0.1:5174/__closkell/inspect` returns the
current app introspection JSON with component dependencies, state-path slot
reads, command schema, type declarations, and template metadata.
For live browser debugging, set `globalThis.__closkellDevtoolsOverlay = true`
before `/src/app.clsk` loads to install the bundled Closkell event overlay.
Custom hooks can still use `globalThis.__closkellDevtools`.

## Project Shape

- `src/domain.clsk` owns state defaults, storage keys, and effect ids.
- `src/zones.clsk`, `src/log.clsk`, and `src/metrics.clsk` hold pure HRWeb domain logic.
- `src/app.clsk` wires typed commands, update handling, and `#html` templates.
- `src/styles.css` is the Solid HRWeb Tailwind entry.
- `node_modules/.vite/closkell/` is the Vite-consumed ESM cache emitted by the Closkell Vite plugin.
- `@closkell/vite-plugin` lets Vite resolve `.clsk` modules as generated ESM.
- `tests/hrweb.spec.ts` covers stored boot, simulator, Bluetooth, zones, import/export, deletion, responsive state, and DOM node reuse during granular updates.
