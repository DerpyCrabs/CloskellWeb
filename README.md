# Closkell

Greenfield Rust compiler workspace for a pure functional Lisp targeting HRWeb.

The current repo intentionally does not preserve old Closkell semantics. The
first proof target is rebuilding the HRWeb app with:

- strict expression-oriented evaluation
- hygienic macro expansion before type checking
- typed command effects
- `#html` templates lowered to granular DOM update plans
- JS ESM output for Vite

## Workspace

- `syntax`: source spans, diagnostics, Lisp parser, and the first `#html` reader
- `macro_expand`: deterministic expansion boundary with quasiquote, gensyms, and `compile-error`
- `hir`: typed compiler-facing module shape
- `typecheck`: initial local inference and shape checks
- `effects`: command-effect catalog and validation boundary
- `template_ir`: static DOM/update-slot lowering
- `js_backend`: readable JS ESM emission
- `cli`: `closkell` developer commands
- `runtime-js`: runtime helpers consumed by generated JS
- `packages/vite-plugin-closkell`: Vite plugin that lets apps import `.clsk`
  modules directly
- `packages/vscode-closkell`: VS Code extension with syntax highlighting,
  formatter, and diagnostics support for `.clsk` files

## CLI

```powershell
cargo run -p cli -- check hrweb/src/app.clsk
cargo run -p cli -- check hrweb/src/app.clsk --json
cargo run -p cli -- check hrweb/src/app.clsk --json --stdin
cargo run -p cli -- check hrweb/src/app.clsk --types
cargo run -p cli -- build hrweb/src/app.clsk --out dist/app.mjs --sourcemap
cargo run -p cli -- test fixtures/hrweb/hrweb_test_suite.clsk
cargo run -p cli -- dev --watch hrweb/src/app.clsk --out dist/app.mjs --sourcemap
cargo run -p cli -- expand fixtures/hrweb/hrweb_macro_app.clsk
cargo run -p cli -- fmt hrweb/src/app.clsk
cargo run -p cli -- inspect hrweb/src/app.clsk
cargo test
```

`check <file>` is quiet on success and prints diagnostics on failure. Add
`--types` to dump inferred forms and the lowered template count. Add `--json`
to emit machine-readable diagnostics for editor integrations. Add `--stdin` to
read the root module source from stdin while still resolving imports relative to
the file path.

The VS Code extension calls `closkell fmt` for formatting and
`closkell check --json --stdin` for diagnostics, piping editor buffers through
stdin instead of writing temporary files beside source files. If a separate
formatting engine is introduced later, it should sit behind `closkell fmt` so
CLI, editor, and CI formatting stay aligned.

`test <file>` compiles a module to temporary ESM and runs its exported `tests`
vector under Node. Each test record uses `:name`, `:actual`, and `:expected`.

HRWeb-specific compiler/runtime fixtures live in `fixtures/hrweb/`. They are
test inputs, not app examples; the full runnable app is under `hrweb/`.

`build --sourcemap` and `dev --watch --sourcemap` write `.mjs.map` files for
the entry and same-tree imported modules, including original `.clsk`
`sourcesContent`.
Imports that contain only type names are checked for validity but are skipped
when writing recursive `.mjs` outputs, and stale generated files for those
type-only modules are removed.

`build --app --out src/main.mjs` turns an entry module that exports `init`,
`update`, and `view` into a Vite-ready browser entry. It imports the no-VDOM
runtime, optionally imports `--css <path>`, finds `document.getElementById` for
`--root <id>` (default `root`), and calls `startApp` with core command handlers.
`--vendor-runtime` copies the bundled `@closkell/runtime` package into the
nearest `package.json` project root's `node_modules`, so Vite can resolve the
runtime import without a manual package step. `dev --watch` accepts the same
`--app`, `--root`, `--css`, and `--vendor-runtime` flags.

Template slot metadata projects reads through local helpers and component calls,
so HRWeb slots such as `(connection-label state)` update from
`state.connected?`/`state.simulated?` instead of treating all of `state` as
dirty.
Dynamic `class={...}` template attributes accept strings, records/maps of
class-name to enabled flag, vectors/arrays, and sets. Structured class values are
normalized with duplicate tokens removed, so view code can write state-derived
class maps without assembling strings by hand. `check` validates concrete class
attribute shapes and infers record/map class flags as `Bool`.
Dynamic `style={...}` template attributes accept CSS strings, records, or
persistent maps. Style keys may use CSS names such as `"box-shadow"`, custom
properties such as `"--accent"`, or JS/Solid-style names such as `:boxShadow`;
the runtime normalizes camelCase and keyword/symbol keys before applying or
removing DOM style properties. `check` rejects dynamic style attributes whose
concrete type is not a CSS string, nil, record, or map with string/number/bool
style property values.
Template `ref={...}` values are checked as string, keyword, or nil optional ref
names; state-derived dynamic refs infer the referenced value as `String`, and
refs are registered through the runtime instead of being emitted as DOM
attributes.
Static `ref="name"`, `class="..."`, and `style="..."` attributes remain valid,
but bare `ref`, bare `class`, and bare `style` are rejected because they have no
useful runtime meaning. Boolean HTML attributes should be written bare, such as
`disabled`, or as dynamic booleans, such as `disabled={state.locked?}`;
misleading static strings like `disabled="false"` are rejected because the
browser would still treat the attribute as enabled by presence.

Generated app entries pass `globalThis.__closkellDevtools` into the runtime.
The hook may be a function, an object with `emit(event)`, or an object with an
`events` array. It receives app lifecycle events, template mount metadata with
slot state reads, template disposal events, state transitions, and command
execution events. During app dispatches, the runtime compares previous and next
state values, passes changed state paths into template updates, and reports
which slots updated or skipped through `template/update` events.
When no custom hook is installed, setting
`globalThis.__closkellDevtoolsOverlay = true` before the generated app module
loads creates the bundled in-browser overlay. The overlay keeps a bounded event
history, renders concise state/template/command summaries, and exposes its
instance at `globalThis.__closkellDevtoolsOverlayInstance` for inspection or
manual disposal.
Persistent vectors are diffed by index path, while persistent `Set` and `Map`
values are compared as atomic collection paths, so slots that read
`state.tags`, `state.metricRegistry`, or similar collection values update when
their immutable contents change.

Apps returned by `startApp` expose `dispose()`. Disposing removes mounted DOM,
unregisters refs, removes template event listeners, and asks bundled command
handlers to release active effect resources such as timers, animation frames,
media-query listeners, resize observers, window listeners, and Bluetooth
notification connections. After an app is disposed, manual dispatches and late
asynchronous command completions are ignored so torn-down apps cannot mutate
state or remount templates.
Keyed list removals dispose their row components even when a row's DOM has
already been detached, preventing stale row listeners or refs from dispatching
after the row leaves state.
Reused keyed rows derive local item/index changed paths such as `entry.label`
from the previous and next row values, so unchanged cells in the same row can
skip while the row DOM remains stable.
Reused child components also receive local prop changed paths from their
parameter metadata, so a component like `(summary-card state.summary)` can
update `summary.value` without touching slots that only read `summary.label`.
Repeated keys are tolerated by assigning stable internal identities to later
occurrences, so imperfect imported data cannot orphan duplicate rows while HRWeb
still gets predictable reuse when keys are unique.
Storage reset flows can use `{:kind :storage/remove :key ... :onSuccess ...}`;
the runtime removes the key and dispatches a success payload containing the
removed `:key`. `storage/get` supports `:format :json` for saved app state that
should fail loudly on malformed JSON and route through `:onError`.
`Cmd.storageRemove(key, { onSuccess, onError })` emits the same typed success
shape, while the older `Cmd.storageRemove(key, msg, onError)` form remains
available for fire-and-forget reset messages.
Throwing or rejecting command handlers are routed through the command's
`:onError` continuation when present, and devtools receives a `command/error`
event before the error message is dispatched.
Commands whose handler is not registered also route through `:onError` when the
command supplies one, making missing host capabilities visible to app state.
Command records must use concrete runtime kinds such as `:timer/every` or
`:http/request`; abstract families like `:timer` and `:http` remain
introspection types, not executable command data.
Runtime `Cmd` helpers for error-capable effects, including storage writes,
downloads, HTTP, canvas draw, DOM refs, window events, and media queries, accept
optional `onError` continuations to match the underlying command records.
Cleanup helpers such as `animationCancel`, `domRefResizeUnwatch`,
`windowEventUnwatch`, and `mediaQueryUnwatch` also accept `{ onSuccess, onError }`
options when the app needs typed completion payloads.
Value-producing helpers such as `timeNow`, `storageGet`, `randomNumber`, and
`httpRequest` emit `onSuccess` continuations so success payloads stay typed;
the effect validator rejects storage reads that omit `:onSuccess`/`:toMessage`.
HTTP commands may pass fetch init through `:request {:url ... :method ...}` or
with top-level `:url`, `:method`, `:headers`, and `:body`; both forms normalize
to the same runtime fetch call. `Cmd.httpRequest` supports both a request record
and a URL plus options object so tests can construct either command shape.
`simulation/heart-rate` provides a typed fake heart-rate monitor for development
and tests. It registers a runtime interval, emits `:onReading` messages with a
`:bpm Number` payload, supports `:onDisconnected`/`:onError`, and is cleaned up
with `simulation/stop` or app disposal.
`window/event-watch` supports guarded `:preventDefault`/`:stopPropagation`
records with key/code and modifier fields, which lets dev hotkeys such as
Ctrl+Shift+H suppress browser defaults only for the matching chord.

Macros expand before type checking. Macro bodies support compile-time `do`,
`let`, `with-gensyms`, `gensym`, and `compile-error`; emitted code is authored
with quasiquote/unquote. `tmp#` inside quasiquote, explicit `(gensym "tmp")`,
and `(with-gensyms [tmp] ...)` produce deterministic fresh symbols, so generated
HRWeb bindings can be spliced into multiple positions without capturing user
locals. Same-tree modules can export macros with `defmacro`; importing a macro
makes it available at expansion time and erases that name from the generated ESM
import surface.

`(env-dev?)` is a zero-argument Bool primitive for Vite-style development
gates such as HRWeb's simulator hotkeys. Generated ESM reads
`import.meta.env.DEV`, with `globalThis.__CLOSKELL_ENV__.DEV` available as a
test or embedding override.

Module-level `(type Name schema)` declarations are checked, erased from emitted
JS, and included in `inspect` output. Schemas support records, tuples,
`Option`, `Result`, `Cmd`, `Fn`, and tagged `Union` declarations.
Module-level `(ann exported-name schema)` contracts are also erased from JS, but
`check` verifies structurally checkable annotations against matching `def` and
`defn` exports. `Cmd` annotations require emitted command values to include a
`:kind` field, so `update` result aliases such as `[State (Cmd Msg)]` can be
checked. Annotated union message parameters seed body inference, including
record-pattern matches against tagged message variants. Command continuation
fields such as `:msg`, `:onSuccess`, `:onError`, and `:onFrame` are checked
against the declared `Cmd` message type, including known payload fields such as
`:value Number` for `time/now`, `:bpm Number` for heart-rate readings, and
nested `:batch` commands. `if` and `match` branch results preserve tagged
record variants so later `Cmd` checks still see the exact command kind produced
by each branch. When a module annotates `update` as `(Fn [State Msg] ...)`,
`#html` event handler messages are checked against the same `Msg` union, so
misspelled UI message tags fail at compile time instead of reaching the browser.
Direct browser globals such as `window.innerWidth`, `document.title`,
`navigator.bluetooth`, `localStorage`, and `fetch` are rejected in pure code,
including `#html` dynamic expressions and event handlers; browser work must be
returned as typed command data.
`match` arms support wildcard, literal, record, vector, fixed `(list ...)`,
`(cons head tail)`, `(as pattern name)`, `(some pattern)`, `(ok pattern)`, and
`(err pattern)` patterns, so handlers can destructure messages, persistent
lists, optional values, and import results while keeping the original value for
persistent updates.
Ordinary `let` bindings accept the same pattern forms, so pure helper code can
unpack records, tuples, fixed lists, and `(cons head tail)` list shapes without
a separate `match`.
The same destructuring works in `let`-wrapped template component bodies; slot
metadata projects destructured locals back to their source state paths such as
`state.payload.reading.bpm`.
Anonymous `fn` parameters accept those patterns too, which keeps HRWeb
`map`/`filter`/`reduce` callbacks close to the data shapes they consume while
ordinary named `defn` helpers can destructure at their API boundary as well.
Template component `defn` parameters stay symbol-only because component update
metadata needs stable prop names.

Numeric literals support underscore separators such as `60_000`, matching the
timing constants used by HRWeb. Numeric primitives include arithmetic,
comparison, `max`, `min`, `min-of`, `max-of`, `sum`, `abs`, `round`, `floor`,
`ceil`, `mod`/`%`, `to-number`, and `to-fixed` plus date-format helpers used by
metrics, chart labels, and JSON-backed zone settings. `date-format` supports
`:month-year`, `:month-day-time`,
`:month-day`, `:month`, `:day`, and `:iso-date`.

String helpers include `trim`, `lower-case`, `pad-start`, `to-radix`,
`string-slice`, `regex-test?`, `includes?`, and `locale-compare` for HRWeb
type labels, workout IDs, and import cleanup.

Vector helpers include `first`, `second`, `last`, `find`, `filter`, `map`,
`map-indexed`, `any?`, `every?`, `includes?`, `reduce`, `reduce-indexed`,
`sort-by`, `sort-by-desc`, `sort-with`, `slice`, `drop-last`, and `take-last`
for HRWeb log, metric, zone, and chart transformations.

Persistent list data is available through `(list ...)`, `(list? value)`,
`(cons value list)`, `(rest list)`, `first`, `second`, `nth`, `last`, `conj`,
`count`, and `empty?`. List updates emit fresh JavaScript arrays instead of
mutating the input collection.

`#html` keyed loops support both `(for [item items :key item.id] ...)` and
`(for [item items index :key item.id] ...)`, so HRWeb zone bars and chart
labels can use loop indices directly while still reusing keyed DOM nodes.

Persistent sets are available through set literals (`#{...}`), `(set ...)`,
`(set? value)`, `(contains? set value)`, `(conj set value...)`, `(disj set
value...)`, `(set-values set)`, `count`, and `empty?`. Set enumeration returns
a vector in insertion order. Set updates emit new JavaScript `Set` instances
rather than mutating the input collection.

Persistent maps for dynamic keys are available through `(hash-map key value...)`,
`(map? value)`, `(map-get map key)`, `(map-assoc map key value...)`, and
`(map-dissoc map key...)`, plus `map-entries`, `map-keys`, `map-values`,
`contains?`, `count`, and `empty?`. Map entries are exposed as
`{:key key :value value}` records. Map reads return `nil` for missing keys, and
updates emit new JavaScript `Map` instances rather than mutating the input
collection.

Typed result values use `(ok value)` and `(err error)`, plus `(ok? result)`,
`(err? result)`, `(result-value result)`, `(result-error result)`, and
`(unwrap-or result fallback)`. These helpers compile to plain JavaScript
objects while preserving `(Result Ok Err)` annotations.

Self-recursive `defn` bodies can be checked against their own signature, and
direct self tail calls are lowered to JavaScript loops through tail positions
inside `if`, `do`, `let`, and `match`.

`dev --watch` performs an immediate checked build, recursively watches same-tree
`.clsk` imports, and rebuilds the entry plus imported modules when any source
changes. `--poll-ms <ms>` controls the polling interval, and `--once` runs a
single checked build for smoke tests.

The Vite plugin exposes the entry module's introspection report at
`/__closkell/inspect` during dev. The JSON matches `closkell inspect`, including
component dependencies, state-path-to-slot metadata, command log schema, type
declarations, annotations, and lowered template slots.
Command log schema includes concrete command records reached through imported
`Cmd` helpers, so entry reports expose browser effects hidden behind modules
such as HRWeb's chart drawing helpers.
By default the plugin lets Vite import `/src/app.clsk` directly, emits cache ESM
under Vite's `node_modules/.vite/closkell/` cache, treats `css: "src/styles.css"` as a Vite-root
path, and can also be used from plain JavaScript imports such as
`import { summary } from "./math.clsk"` without a separate generated source
directory. The plugin also excludes the vendored `@closkell/runtime` package
from Vite dependency prebundling, so dev servers use the current runtime module
instead of a stale optimized copy.

## Current Slice

The compiler now parses core Lisp plus `#html`, expands hygienic macros before
type checking, supports macro-authored `compile-error` diagnostics, validates
typed command effects, emits JS ESM, lowers templates to granular runtime
update slots, skips state-dependent slots whose read paths did not change, and
has HRWeb-focused vertical slices for metrics, logs, zones, workout tag sets,
typed import results, tail-recursive reading transforms, timers, Bluetooth,
import/export, DOM ref focus/click triggers, selected-file reads from registered
input refs, storage reset, JSON-backed zone boot, canvas drawing, detail tabs,
log selection reconciliation, delete-hold flows, lifecycle cleanup, and runtime
devtool hooks for template/state/command inspection.
