# Closkell Language And Web Framework Design Target

This document defines the target language and web framework design for
Closkell. It is intentionally precise enough for an AI coding agent to judge
whether an implementation matches the desired shape.

The design is based on the current compiler crates, runtime, fixtures, and the
two example projects:

- `projects/hrweb`: a stateful HR monitor app with typed commands, canvas,
  storage, timers, Bluetooth, keyed lists, and granular DOM updates.
- `projects/better-swagger-ui`: a larger document app that imports JS libraries
  such as `marked`, `dompurify`, and `yaml`, and exposes scalability pressure in
  a large single `update` function.

## North Star

Closkell is a pure functional, Lisp-shaped language that compiles to JavaScript
ESM and ships with a no-VDOM web framework. Its main product goal is to let AI
and humans build large browser applications with fewer accidental bugs than
JavaScript/TypeScript and with better runtime performance than React-style
rerendering.

The framework must not use signals, proxies, or a virtual DOM. It compiles
static template knowledge into direct DOM creation and direct DOM updates. State
changes update only slots whose compile-time read paths overlap the changed
state paths.

## Current Baseline To Preserve

The existing repo already has the right spine:

- Lisp syntax with `def`, `defn`, `let`, `if`, `match`, records, vectors, sets,
  keywords, quoting, macros, and `#html`.
- Hygienic macro expansion before type checking.
- Type declarations and annotations with records, tuples, `Option`, `Result`,
  `Cmd`, `Fn`, and tagged `Union` records.
- Pure app functions where `init` and `update` return `[state cmd]`.
- Command effects represented as typed data such as `{:kind :http/request ...}`.
- Compiler/runtime rejection of many direct browser globals in app code.
- `#html` lowering to static nodes and update slots.
- Slot read-path metadata, helper read summaries, component dependency reports,
  keyed list reuse, conditional component branches, and template devtools.
- ESM output that works with Vite and can import `.clsk` and bare JS packages.
- Module tests can already be compiled and run under Node from exported
  Closkell test data.
- Runtime command handlers for browser capabilities and deterministic cleanup on
  app disposal.

Extensions must build on these ideas rather than replacing them with a
JavaScript framework model.

## Design Principles

1. Pure by default.
   Every ordinary Closkell function is referentially transparent. It cannot
   mutate objects, mutate DOM, read browser globals, start timers, fetch, write
   storage, call random, or perform hidden I/O.

2. Effects are data.
   Effects are returned as `Cmd Msg`, `Task Err Ok`, or `Sub Msg` values and are
   interpreted by the host runtime. Pure code can construct effect descriptions,
   transform them, and batch them, but cannot execute them.

3. State is immutable.
   Records, vectors, lists, sets, and maps are persistent values. Update helpers
   must return fresh values and never mutate their inputs.

4. Templates are analyzable.
   Template dynamic expressions must be ordinary pure expressions over explicit
   parameters and read-only event payloads. The compiler must be able to collect
   conservative read paths for every dynamic slot.

5. No hidden reactivity.
   There are no signals, proxies, dependency-tracking effects, lifecycle hooks,
   or `useEffect` equivalents. Long-lived resources are declared as data through
   subscriptions.

6. Interop is explicit.
   JS imports are allowed with zero runtime wrapping. Unannotated JS values have
   compile-time type `Js`; annotated foreign bindings expose programmer-owned
   Closkell types. TypeScript declaration files are not trusted as the language
   type system.

7. Code must be reviewable.
   The formatter is canonical, syntax has few special cases, public bindings are
   annotated, union messages are explicit data, and compiler diagnostics point
   to the bad form.

8. Testing is first-class.
   Pure functions, reducers, commands, subscriptions, templates, and mounted
   components have a standard test harness. Tests are ordinary Closkell modules
   that type check, compile to ESM, run without bespoke app glue, and can be
   executed by the Closkell CLI or by Vitest.

9. AI editing is a first-class workflow.
   The primary AI workflow is ordinary file editing followed by fast compiler
   feedback. The compiler provides deterministic, machine-readable diagnostics,
   inspection data, and incremental caches so an AI can edit, check, inspect,
   test, and repeat without relying on an IDE language server.

## Language Surface

### Modules

A `.clsk` file is a module. Top-level forms are:

- `(import path [names...])`
- `(type Name schema)`
- `(ann name schema)`
- `(def name expr)`
- `(defn name [params...] body...)`
- `(defmacro name [params...] body...)`

Top-level side effects are forbidden. A top-level `def` may bind pure constants,
pure functions, template components, command constructors, and foreign pure
wrappers. It must not execute commands or host effects.

Design rule:

- Keep current import syntax.
- Continue erasing `type` and `ann` from emitted JS.
- Exported non-test `defn` bindings have `ann`.

### Syntax

Keep the current core syntax:

```clojure
nil true false
42 60_000 "text" :keyword symbol
[a b c]
{:field value "dynamic" value}
#{a b c}
(fn [x] (+ x 1))
(let [x 1 y 2] (+ x y))
(match value pattern result _ fallback)
#html <button class={button-class}>{label}</button>
```

Rules:

- `if` conditions must be `Bool`. There is no truthiness.
- Record field access uses `value.field`.
- `get` is for dynamic or uncertain records/JS-shaped data.
- Pattern binding is allowed in `let`, `fn`, `defn`, and `match`.
- Component `defn` parameters remain symbol-only until the compiler has stable
  metadata for destructured component props.

### Types

The type system supports:

- `Number`, `String`, `Bool`, `Nil`, `Html`, `Js`.
- `Option T`, with `nil` as the none value.
- `Result Ok Err`.
- `Vector T`, `List T`, `Set T`, `Map K V`.
- Tuple syntax `[A B C]`.
- Structural records `{:field Type}`.
- Tagged unions:

```clojure
(type AppMsg
  (Union
    {:kind :load}
    {:kind :loaded :value Spec}
    {:kind :failed :error String}))
```

Final design includes:

- Parametric type aliases:

```clojure
(type RemoteData a
  (Union
    {:kind :idle}
    {:kind :loading}
    {:kind :ready :value a}
    {:kind :failed :error String}))
```

- Exhaustive `match` checking for annotated union values.
- Better `Option` discipline: a value of type `T` is not implicitly nullable.
- Distinct `Js` type for statically unknown interop values. `Js` is a type
  checker boundary only and has no runtime representation.
- `Decoder T` helpers for moving from unknown `Js` into typed Closkell records
  when runtime validation is desired.

Design rule:

- `match` over an annotated union is an error when it is not exhaustive and does
  not contain an explicit fallback arm.
- A fallback arm in an app `update` function is allowed, but review tools can
  report which union variants are handled only by fallback.

### Equality And Ordering

Closkell avoids JavaScript equality gotchas.

- `=` means value equality for Closkell data.
- `identical?` means reference identity or JavaScript `Object.is`.
- `<`, `>`, `<=`, `>=` are numeric unless the function name says otherwise.
- `locale-compare` remains the string ordering primitive.

Value equality covers records, vectors, lists, sets, maps, keywords, `Option`,
and `Result`. Implementations may use primitive fast paths, but observable
behavior is Closkell value equality rather than JavaScript reference equality.

### Standard Library

The standard library is larger and safer than hand-written JS utility code. It
includes:

- `Option`: `some?`, `nil?`, `map-option`, `and-then-option`, `unwrap-or`.
- `Result`: `ok`, `err`, `ok?`, `err?`, `map-ok`, `map-err`, `and-then-result`,
  `unwrap-or`.
- Immutable records: `assoc`, `merge`, `dissoc`, `update-in`, `get-in`.
- Vectors/lists: `map`, `map-indexed`, `filter`, `find`, `reduce`,
  `reduce-indexed`, `sort-by`, `sort-with`, `slice`, `take`, `drop`,
  `take-last`, `drop-last`, `range`.
- Sets/maps: `set`, `contains?`, `conj`, `disj`, `hash-map`, `map-get`,
  `map-assoc`, `map-dissoc`, `map-entries`, `map-keys`, `map-values`.
- Strings: `trim`, `lower-case`, `upper-case`, `split`, `join`,
  `starts-with?`, `ends-with?`, `includes?`, `pad-start`, `string-slice`,
  `regex-test?`, `regex-capture`, `regex-capture-all`.
- Numbers/dates: arithmetic, `min`, `max`, `min-of`, `max-of`, `sum`, `abs`,
  `round`, `floor`, `ceil`, `mod`, `to-number`, `to-fixed`, date helpers.
- JSON: `json-parse-result`, `json-stringify`, schema-oriented decoders.
- URL helpers as pure functions when all inputs are explicit.
- Explicit build/runtime environment constants such as `env-dev?` and
  `env-mode`, used for visible capability choices like development proxy mode.

Host reads and writes must not be pure standard library functions. For example,
clipboard writes, current URL reads, history writes, storage reads, file reads,
DOM queries, and theme DOM changes are commands, tasks, boot inputs, or
subscriptions.

## Effects

### Command Values

`Cmd Msg` is the effect type for one-shot host operations that may dispatch zero
or one completion messages. `Cmd.none` and `Cmd.batch` are compact helpers for
explicit command data; they do not execute effects.

Examples:

```clojure
Cmd.none

(Cmd.batch [(load-user-command id)
            (focus-search-command)])

{:kind :http/request
 :request {:url "/api/user" :method "GET"}
 :toMessage (fn [response] {:kind :user-loaded :value response})
 :onError :user-load-failed}
```

Framework helpers may construct common command records, but they are still
plain data constructors. They must not perform host work and must not create
implicit data flow. Completion messages stay explicit through `Msg` helpers:

```clojure
(Msg.of :saved)
(Msg.with :selected :id id)
(Msg.with2 :heart-rate-at :bpm bpm :timestamp timestamp)
(Msg.mapper :loaded :payload)

(Cmd.storage/get "settings" :json (Msg.mapper :loaded :payload) :load-failed)
(Cmd.time/now (Msg.mapper :started-at :timestamp))
(Cmd.dom-ref/measure "track" to-track-message :measure-failed)
(Cmd.bluetooth/connect-heart-rate "hrm"
                                  {:filters [{:services ["heart_rate"]}]}
                                  (Msg.mapper :connected :info)
                                  :heart-rate
                                  :disconnected
                                  :connect-failed)
```

HTTP command response modes include `:json`, `:text`, and `:auto`. `:auto`
keeps response handling explicit while avoiding repeated app code: the runtime
classifies JSON, text, CSV, and file/blob responses, preserves download blobs,
derives filenames, and exposes copyable text and image preview URLs in the
success payload.

Rules:

- `init` returns `[State (Cmd Msg)]`.
- `update` returns `[State (Cmd Msg)]`.
- Command records must use concrete `:kind` values.
- Completion fields must be type checked against `Msg`.
- Commands are plain data and may be tested without a browser.
- Command execution is owned by the runtime, not by Closkell code.
- Command helper functions may be shared across modules when their public
  annotation says they return `Cmd`, for example
  `(ann copy-command (Fn [String] (Cmd msg)))`. Lowercase names inside an
  annotation are explicit annotation-local type variables; they let no-message
  helpers stay reusable without widening to `Js` or hiding effects.
- Network proxying is explicit command data, for example `:proxy true` on
  `:http/request`, and is usually guarded by `env-mode`.
- One-shot DOM actions such as scrolling to a known element are commands, for
  example `{:kind :dom/scroll-into-view :testId "operation-get:/pets"}`. They
  are not hidden lifecycle hooks.

### Tasks

Final design includes `Task Err Ok`.

Use `Task` for composable async operations before converting them into a
command:

```clojure
(ann load-spec-task (Fn [String] (Task String Spec)))
(defn load-spec-task [url]
  (Task.and-then (Http.get-text url) decode-spec-text))

(defn load-spec-command [url]
  (Task.perform load-spec-task
                url
                (fn [spec] {:kind :loaded :value spec})
                (fn [error] {:kind :failed :error error})))
```

Tasks are still effect descriptions. Pure code cannot run them directly.

### Subscriptions

Final design includes `Sub Msg`.

Use `Sub` for long-lived resources that are tied to state: timers, media query
watches, resize watches, window events, animation loops, and streams.

```clojure
(ann subscriptions (Fn [AppState] (Sub AppMsg)))
(defn subscriptions [state]
  (Sub.batch
    [(if (= state.exerciseState "running")
         (Sub.timer/every clock-timer-id 250 {:kind :clock-tick})
         Sub.none)
     (Sub.media-query "mobile" "(max-width: 700px)" :media-changed)
     (Sub.window/event-with "drag"
                            "pointermove"
                            :drag-moved
                            {:preventDefault true
                             :options {:passive false}})]))
```

Runtime behavior:

- After `init` and every `update`, compute `subscriptions state`.
- Diff by stable subscription id and kind.
- Start new subscriptions, keep unchanged subscriptions, stop removed ones.
- Dispose all subscriptions when the app is disposed.

This replaces `useEffect`-style lifecycle code with pure state-derived data.
Long-lived effects use `Sub`.

### Event Handlers

Template event handlers are pure expressions evaluated with a typed, read-only
`event` value.

Allowed handler results:

- `Msg`
- `Event Msg`
- `nil`, meaning dispatch nothing

`Event Msg` is an event-control value built by pure helpers:

```clojure
on:submit={(Event.prevent {:kind :load})}

on:keydown={(if (= event.key "Enter")
                (Event.prevent {:kind :commit :value event.currentTarget.value})
                nil)}
```

Runtime behavior:

- If the result is `Event.prevent`, call `preventDefault` before dispatch.
- If the result has stop-propagation metadata, call `stopPropagation`.
- Dispatch the contained message if present.

Compiler rule:

- Direct calls such as `(event.preventDefault)` and `(event.stopPropagation)`
  are rejected in Closkell code.

## JS Interop

Closkell must be able to use JS/TS libraries from npm without depending on
TypeScript type declarations.

Current import syntax remains:

```clojure
(import "marked" [(parse as markedParse)])
(import "dompurify" [(default domPurify)])
(import "yaml" [(parse as parseYaml)])
```

Rules:

- JS interop is zero-cost by default. `Js` does not wrap, clone, proxy, validate,
  or tag the runtime value.
- Imported JS values have compile-time type `Js` unless a Closkell `foreign`
  declaration gives them a programmer-owned type.
- A `foreign pure` declaration is allowed only for deterministic functions that
  do not read or write host state.
- A `foreign task` or command wrapper is required for async or impure JS.
- Foreign declarations are trust boundaries. They compile to direct JS imports
  and calls; they do not emit runtime validators unless the programmer asks for
  decoding.
- TS `.d.ts` files may help generate wrapper declarations, but generated
  wrappers must expose explicit Closkell schemas, `Js`, or explicit unchecked
  boundaries.
- App code may intentionally sidestep type checking at interop boundaries with
  `unsafe-cast`; the cast is visible in source and inspection output and emits
  no runtime code.

Example:

```clojure
(import "yaml" [(parse as parseYaml)])

(foreign pure parseYaml (Fn [String] Js))

(ann decode-openapi (Fn [Js] (Result OpenApiSpec String)))
(defn decode-openapi [value]
  ...)

(ann parse-openapi-yaml (Fn [String] (Result OpenApiSpec String)))
(defn parse-openapi-yaml [text]
  (decode-openapi (parseYaml text)))
```

When the programmer owns the boundary and wants no runtime validation, they
write the foreign type directly:

```clojure
(import "yaml" [(parse as parseYaml)])

(foreign pure parseYaml (Fn [String] OpenApiSpec))

(ann parse-openapi-yaml (Fn [String] OpenApiSpec))
(defn parse-openapi-yaml [text]
  (parseYaml text))
```

When a value is dynamic and the programmer intentionally accepts the risk, the
escape hatch is explicit and zero-cost:

```clojure
(ann parse-openapi-yaml (Fn [String] OpenApiSpec))
(defn parse-openapi-yaml [text]
  (unsafe-cast OpenApiSpec (parseYaml text)))
```

Design rule:

- App state stores raw `Js` only when that field is explicitly annotated as
  `Js`; otherwise JS values entering app state are typed by a foreign
  declaration, decoded, or explicitly `unsafe-cast`.
- Pure URL parsing is allowed when all inputs are explicit.
- History, clipboard, storage, DOM, selected file, cookie, and theme DOM access
  are commands, tasks, subscriptions, or boot inputs.

## Web Framework

### App Contract

An app module exports:

```clojure
(ann init (Fn [] [State (Cmd Msg)]))
(ann update (Fn [State Msg] [State (Cmd Msg)]))
(ann view (Fn [State] Html))
```

Optional export:

```clojure
(ann subscriptions (Fn [State] (Sub Msg)))
```

The runtime owns:

- calling `init`,
- mounting `view state`,
- dispatching messages,
- calling `update`,
- computing changed state paths,
- updating template slots,
- executing commands,
- diffing subscriptions,
- disposing DOM, refs, command resources, and subscriptions.

### Template Model

`#html` is not JSX and does not create a VDOM. It is a compile-time template.

Compiler output for each component:

- `create()`: create DOM nodes once, store stable node references.
- `update(instance, dispatch, updateContext)`: update only dynamic slots.
- `dispose(instance)`: remove listeners, refs, child components, subscriptions
  owned by the component, and DOM nodes.
- `slots`: metadata for devtools and update pruning.

Every dynamic part is a slot:

- text slot
- attribute slot
- style slot
- class slot
- event slot
- ref slot
- keyed list slot
- conditional slot
- component slot

Every non-event slot has:

- a stable slot id,
- source expression,
- conservative read paths,
- component uses.

Example:

```clojure
(defn summary-card [summary]
  #html <article class={summary.class}>
          <strong>{summary.value}</strong>
        </article>)

(defn view [state]
  #html <main>
          {(summary-card state.summary)}
        </main>)
```

The component call slot in `view` reads:

```text
state.summary.class
state.summary.value
```

It must not read all of `state`.

### Update Granularity

Runtime update flow:

1. `update previous-state msg` returns `[next-state cmd]`.
2. Runtime computes changed paths such as `state.summary.value`.
3. Component `update` receives changed paths.
4. A slot updates when any read path overlaps any changed path.
5. A slot skips when all its reads are disjoint from changed paths.

Path overlap rule:

- `state.summary` overlaps `state.summary.value`.
- `state.summary.value` overlaps `state.summary`.
- `state.summary.label` does not overlap `state.summary.value`.

Collections:

- Vectors report index paths when possible: `state.entries.3.label`.
- Sets and maps may be atomic paths unless the compiler/runtime has a stable
  key-path strategy.
- Keyed lists must compute local row changed paths from previous and next item
  values, so unchanged row slots skip.

Optimization contract:

- Keep runtime diff as the fallback.
- Add compiler-produced changed-path summaries for common update forms:
  `assoc`, `merge`, `dissoc`, `update-in`, `map-assoc`, `map-dissoc`, and
  scoped child updates.
- Generated update code may return hidden metadata with exact changed paths.

### Keyed Lists

Syntax:

```clojure
{(for [entry state.entries :key entry.id]
   #html <article>{entry.label}</article>)}

{(for [entry state.entries index :key entry.id]
   #html <article data-index={index}>{entry.label}</article>)}
```

Rules:

- Key expressions must be pure.
- Rows are reused by key.
- Duplicate keys are tolerated with deterministic internal identities.
- Removing a row disposes that row component even if the DOM node was already
  detached.
- Reordering moves existing DOM nodes instead of remounting.

### Conditionals

Template conditionals:

```clojure
{(if state.connected?
     #html <strong>Connected</strong>
     #html <em>Disconnected</em>)}
```

Rules:

- Branches may be templates, nested conditionals, or component calls.
- Branch changes dispose the old branch component.
- Unchanged branch updates only its dirty slots.
- Conditional slot reads include condition reads and branch reads, but nested
  keyed loop locals must not leak as state reads.

### Components

Current component form is a pure `defn` returning `Html`.

```clojure
(defn monitor-card [state]
  #html <section>{state.latestBpm}</section>)
```

Keep this form for simple presentational components.

Final design includes scoped stateful components for large apps.

Each feature module may export:

```clojure
(type State ...)
(type Msg ...)
(ann init (Fn [] [State (Cmd Msg)]))
(ann update (Fn [State Msg] [State (Cmd Msg)]))
(ann subscriptions (Fn [State] (Sub Msg)))
(ann view (Fn [State] Html))
```

Parent composition target:

```clojure
(import "./log.clsk" [(State as LogState)
                      (Msg as LogMsg)
                      (init as log-init)
                      (update as log-update)
                      (subscriptions as log-subscriptions)
                      (view as log-view)])

(type AppState
  {:log LogState
   :route String})

(type AppMsg
  (Union
    {:kind :log :msg LogMsg}
    {:kind :route-changed :route String}))

(defn update [state msg]
  (match msg
    {:kind :log :msg child-msg}
      (scope-update state :log child-msg log-update :log)
    {:kind :route-changed :route route}
      [(assoc state :route route) {:kind :none}]))

(defn subscriptions [state]
  (scope-subscriptions state.log log-subscriptions :log))

(defn view [state]
  #html <main>
          {(scope-view :log log-view state.log)}
        </main>)
```

`scope-update` behavior:

- Calls child `update` with child state and child message.
- Replaces parent field with the returned child state.
- Maps every child command message into `{:kind :log :msg child-message}`.
- Returns `[parent-state mapped-command]`.

`scope-subscriptions` behavior:

- Calls child subscriptions.
- Maps child subscription messages into parent wrapper messages.

`scope-view` behavior:

- Renders child view.
- Wraps child event dispatches in the same parent message wrapper.
- Preserves child slot read metadata under the parent path, for example
  `state.log.entries`.

Design contract:

- `scope-update`, `scope-subscriptions`, and `scope-view` are ordinary,
  reviewable composition forms from the programmer's perspective.
- Whether they are implemented as macros, stdlib helpers, or compiler-recognized
  forms is not visible in app code.
- Inspect output must show the expanded parent/child state paths and message
  wrappers.

### Context

Implicit dynamic context makes AI review harder. Closkell avoids React-like
hidden context by default.

Rules:

- Shared read-only app environment is an explicit parameter or boot input.
- Any auto-threaded context must appear in component annotations and inspection
  output.
- Context must not be mutable and must not be an effect channel.

Prefer explicit props until repeated prop threading becomes a measured problem.

### Styling

The framework supports:

- static `class` and `style`,
- dynamic class strings,
- dynamic class maps/vectors/sets with duplicate token removal,
- dynamic style strings,
- dynamic style records/maps with CSS property normalization.

Rules:

- Boolean attributes are written bare or as dynamic booleans.
- Static misleading booleans such as `disabled="false"` are errors.
- `ref` is a runtime ref registration, not a DOM attribute.
- `innerHTML` is allowed only for values explicitly marked trusted or sanitized.

Rules:

- Add `TrustedHtml` type.
- `innerHTML={...}` requires `TrustedHtml`, not `String`.
- JS sanitizers such as DOMPurify return `TrustedHtml` only through an
  explicit wrapper.

### SSR And Hydration

SSR and hydration are part of the target design.

Server render target:

```clojure
(render-to-string view state)
```

Output:

- HTML string.
- serialized initial state when requested.
- template/component ids for hydration.
- ref and event slot metadata, not event listeners.

Hydration target:

```js
hydrateApp({ root, initState, update, view, subscriptions, handlers })
```

Rules:

- `view` remains pure and DOM-free.
- Template IR must be serializable.
- Browser-only commands/subscriptions do not execute on the server.
- Components may define server-safe boot inputs, but not server-side DOM reads.

## Compiler Architecture Target

The compiler architecture is this pipeline:

1. Parse source to syntax AST.
2. Resolve module graph.
3. Expand macros deterministically.
4. Lower to HIR with stable node ids and binding ids.
5. Type check HIR.
6. Validate purity and effects.
7. Lower `#html` to Template IR.
8. Infer template read paths and component graph.
9. Emit JS ESM.
10. Emit optional sourcemaps and inspection JSON.

HIR is the semantic boundary for name resolution, stable ids, type checking,
purity checks, and template analysis. Syntax AST remains a parsed source shape
rather than accumulating semantic special cases.

Inspection JSON remains a first-class product. It includes:

- exports,
- types and annotations,
- command log schema,
- subscription schema,
- component graph,
- template slots,
- state path to slots,
- JS interop boundaries,
- effect capability usage,
- unused top-level definitions and imports reachable from app roots,
- compiler-derived changed-path summaries when available.

## AI Editing Feedback

Closkell treats AI-assisted editing as a core development mode. The assumed loop
is simple:

```powershell
closkell check src/app.clsk --json
closkell inspect src/app.clsk
closkell test src/app_test.clsk --json
closkell build src/app.clsk --out dist/app.mjs --sourcemap
```

An AI can edit source files directly, run those commands, read diagnostics and
inspection JSON, and continue. A language server, query-based compiler API, or
long-running daemon may exist, but the design does not depend on one.

Compiler feedback requirements:

- `check`, `build`, `test`, and `inspect` accept file paths and produce stable
  machine-readable JSON when requested.
- Diagnostics include file, span, severity, stable diagnostic code, concise
  message, expected/actual types when relevant, and suggested fix metadata when
  the compiler can produce it safely.
- Output order is deterministic.
- Diagnostics are local and actionable: the compiler reports the smallest source
  form that explains the error.
- `inspect` exposes the semantic facts an AI needs for edits: exports, imports,
  type declarations, annotations, inferred public signatures, component graph,
  slot reads, command schema, subscription schema, JS interop boundaries, test
  cases, unused-code reachability, and changed-path summaries.

Compilation is incremental and cache-backed. The compiler maintains a persistent
cache under the project or workspace, for example `.closkell/cache`, keyed by:

- compiler version,
- target backend and runtime ABI version,
- source file content hash,
- import path resolution,
- macro definitions and macro expansion inputs,
- relevant compiler flags,
- dependency fingerprints.

The cache stores reusable artifacts at module granularity:

- parsed syntax tree,
- macro-expanded source,
- HIR with stable ids,
- type exports and type summaries,
- effect and purity summaries,
- template IR and slot read metadata,
- command and subscription schemas,
- test IR,
- emitted JS,
- sourcemaps,
- inspection JSON.

The cache also supports definition-level reuse where HIR ids and dependency
fingerprints make it sound. Definition-level artifacts include inferred
signatures, free-variable/read-path summaries, command shapes, template slots,
emitted function bodies, and test case metadata.

A small compiler database is part of the target design. It may be a structured
file set or an embedded database such as SQLite. Its job is to answer compiler
questions without re-reading and re-checking the whole app:

- Which modules import this module?
- Which definitions depend on this definition?
- Which templates read `state.entries`?
- Which tests cover this function or component?
- Which commands or subscriptions can this module emit?
- Which JS interop boundaries enter this module?
- Which generated ESM files are stale?

The database is a cache, not source of truth. Deleting it never changes program
meaning; it only makes the next command colder.

Performance contract:

- Rechecking an unchanged app after process restart uses persistent cache data.
- Editing a leaf pure function rechecks that definition, dependent definitions,
  affected tests, and affected emit artifacts rather than the whole app.
- Editing a type, macro, import, or public annotation invalidates the dependent
  graph conservatively.
- Editing a template invalidates that component's Template IR, emitted body,
  component graph edges, affected slot-read inspection data, and tests that
  render it.
- Cache misses are safe; stale cache hits are not allowed.
- The compiler can explain invalidation in debug output so cache behavior is
  inspectable.

## Runtime Architecture Target

Runtime public surface:

- `startApp`
- `hydrateApp`
- `createCommandHandlers`
- `createSubscriptionHandlers`
- `createTemplateComponent`
- low-level slot helpers: `setText`, `setAttr`, `setEvent`, `setRef`,
  `setKeyedList`, `setConditional`, `setComponent`
- devtools overlay and event hooks

Runtime invariants:

- Never call `update` after disposal.
- Ignore late async command completions after disposal.
- Dispose listeners, refs, keyed rows, child components, command resources, and
  subscriptions.
- Keep slot values cached to avoid redundant DOM writes.
- Keep event listeners stable when event slot identity is unchanged.
- Report template mount/update/dispose and command/subscription events to
  devtools.

## Testing

Testing is part of the language and framework design, not an external convention
left to each app.

Closkell test modules are `.clsk` modules that import a standard test API:

```clojure
(import "closkell/test" [describe test expect= expect-ok render fire text attr messages commands])

(describe "duration-label"
  (test "formats whole minutes"
    (expect= (duration-label 60_000) "1:00")))

(describe "monitor button"
  (test "dispatches connect message"
    (let [view (render (monitor-card disconnected-state))
          _ (fire.click view "[data-testid='connect-monitor']")]
      (expect= (messages view) [{:kind :connect-monitor}]))))
```

The exact helper names may be implemented as macros or functions, but the
programmer-facing model is:

- `describe` groups tests.
- `test` defines a named test case.
- `expect=`, `expect-not=`, `expect-ok`, `expect-err`, `expect-some`,
  `expect-nil`, `expect-match`, and `expect-throws` are standard assertions.
- Failed assertions report the source span, expected value, actual value, and a
  stable data diff for Closkell records, vectors, lists, maps, sets, keywords,
  `Option`, and `Result`.
- Tests are type checked before execution.

Pure function tests run without a DOM or host command handlers. They are the
default shape for domain modules and reducers:

```clojure
(test "zone lookup clamps to configured bounds"
  (expect= (zone-label-for-bpm zones 130) "Zone 2"))

(test "update returns explicit command data"
  (let [[next cmd] (update state {:kind :start})]
    (expect= next.appStatus "Starting")
    (expect= cmd.kind :time/now)))
```

Component tests use a built-in DOM harness. The harness can render any `Html`
component into an isolated document, query it, dispatch browser-like events, and
capture messages without starting a whole app:

```clojure
(test "tab strip selects metrics"
  (let [h (render (desktop-tab-strip state))
        _ (fire.click h "[data-testid='tab-metrics']")]
    (expect= (messages h) [{:kind :select-tab :view "metrics"}])
    (expect= (text h "[data-testid='tab-metrics']") "Metrics")))
```

Component harness requirements:

- `render` mounts a component into an isolated root and returns a harness value.
- `rerender` updates the same component with new props or state.
- `dispose` verifies cleanup of refs, listeners, child components, and keyed
  rows.
- `find`, `find-all`, `text`, `html`, `attr`, `class?`, and `style` inspect DOM.
- `fire.click`, `fire.input`, `fire.change`, `fire.keydown`, `fire.pointerdown`,
  and a generic `fire.event` dispatch typed events.
- Event dispatch captures emitted messages and event-control behavior such as
  prevented default and stopped propagation.
- Harness updates expose template devtools frames, updated slots, skipped slots,
  and state read paths for granular update assertions.

App tests use a higher-level harness that mounts `init`, `update`, `view`, and
optional `subscriptions` together:

```clojure
(test "start emits time command"
  (let [app (mount-app {:init init :update update :view view}
                       {:handlers fake-handlers})
        _ (app.dispatch {:kind :start})]
    (expect= app.state.appStatus "Starting")
    (expect= (commands app) [{:kind :time/now}])))
```

App harness requirements:

- `mount-app` accepts fake command handlers, fake subscription handlers, fake
  time, fake random, fake storage, fake fetch, and fake DOM capabilities.
- `dispatch` sends typed messages through the real `update`.
- `commands` returns command records emitted by the app in order.
- `subscriptions` returns the active subscription set.
- Async commands can be resolved or rejected deterministically.
- Timers and animation frames can be advanced with fake time.
- Late async completions after `dispose` are ignored and testable.

Vitest support is part of the target developer experience:

- Vitest can run `.clsk` test files directly through the Closkell Vite plugin or
  a companion `@closkell/vitest` adapter.
- `.clsk` test modules compile to ESM that registers tests with Vitest's
  `describe`, `test`, and assertion lifecycle.
- Vitest watch mode, filtering, failure reporting, and browser/jsdom
  environments work for Closkell tests.
- A JavaScript/TypeScript Vitest file can import `.clsk` modules, including
  pure functions and components, through the same Vite transform used by apps.
- A `.clsk` test file can import JS test helpers when they are explicitly
  wrapped through the JS interop boundary.

The Closkell CLI also supports running tests without Vitest:

```powershell
closkell test path/to/module_test.clsk
```

CLI and Vitest execution use the same test IR and assertion semantics, so a test
does not pass in one runner and mean something different in the other.

## Conformance Contract

A Closkell implementation conforms to this design when all of these statements
are true:

- Ordinary Closkell functions are pure and cannot hide host reads, host writes,
  mutation, timers, fetches, DOM operations, random values, storage access, or
  clipboard access.
- All host work is represented as `Cmd`, `Task`, `Sub`, explicit boot input, or
  explicit JS interop capability.
- App state is persistent immutable data.
- Public app boundaries use explicit `type` and `ann` declarations.
- `match` over an annotated tagged union is exhaustive unless the fallback arm
  is intentionally present.
- JS imports are allowed with zero runtime wrapping; unannotated imports are
  statically `Js`, and typed or unchecked boundaries are explicit in source and
  inspection output.
- `innerHTML` accepts only `TrustedHtml`.
- Template event expressions return `Msg`, `Event Msg`, or `nil`; they do not
  call event mutation methods directly.
- Templates lower to static DOM creation plus dynamic slots, not VDOM nodes.
- Slot read paths are visible in inspection output and drive granular updates.
- Components can be composed as scoped state/action modules without losing
  granular update metadata.
- Long-lived host resources are declared as state-derived `Sub` values.
- Pure functions, reducers, commands, subscriptions, templates, components, and
  mounted apps are testable through the standard Closkell test harness.
- Vitest can discover and run `.clsk` test modules through the Closkell Vite
  transform or a dedicated Closkell Vitest adapter.
- `check`, `build`, `test`, and `inspect` are fast enough for AI edit loops and
  expose deterministic JSON output.
- The compiler maintains sound persistent module-level caches and supports
  definition-level reuse where dependency fingerprints make it safe.
- Compiler inspection data is available from files and caches without requiring
  an IDE language server or always-on compiler daemon.
- Runtime disposal clears DOM, refs, event listeners, commands, subscriptions,
  keyed children, and async completions.
- The same Template IR can produce client DOM code, inspection data, and
  SSR/hydration output.

## Non-Goals

- Do not emulate React hooks.
- Do not introduce signals or proxy-based reactivity.
- Do not add class-based components.
- Do not depend on TypeScript type checking for correctness.
- Do not allow arbitrary mutation as an escape hatch.
- Do not make `useEffect`-style lifecycle code the normal way to use host
  resources.
- Do not hide important state flow behind implicit context unless inspection can
  show the exact dependency.
- Do not make an IDE language server, query compiler, or always-on daemon
  required for ordinary checking, building, testing, or AI-assisted edits.

## Design Summary

Closkell is Elm-like in effect discipline, Clojure-like in compact data syntax,
ML-like in type-guided reviewability, and Svelte-like in compile-time DOM
specialization. The key difference from React and Solid is that the compiler, not
runtime subscriptions or VDOM reconciliation, owns the dependency graph.

The design preserves the current app contract and template IR while requiring
stricter purity, explicit JS interop, scoped components, declarative
subscriptions, first-class testing, AI-friendly compiler feedback, and SSR-ready
template generation.
