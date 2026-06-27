# Closkell Browser Framework Design Target

This document defines the browser framework built on top of the Closkell
language core. It owns frontend concepts such as `Html`, `#html`, DOM events,
browser commands, browser subscriptions, styling, hydration, and no-VDOM DOM
updates.

The browser framework is not part of the language core. It is a framework
library, compiler extension, and runtime package.

## Design Influences

Elm contributes the browser app boundary: state, messages, `init`, `update`,
`view`, commands, subscriptions, explicit modules, and runtime-owned effects.

re-frame contributes data-oriented decomposition: feature modules, event/effect
separation, derived queries, and an inspectable dataflow graph.

Closkell keeps Elm's purity discipline and re-frame's dataflow clarity while
using compile-time DOM specialization instead of a virtual DOM, signals, or
proxy tracking.

## Browser App Contract

A browser app module exports:

```clojure
(ann init (Fn [BrowserBoot] [State (Cmd Msg)]))
(ann update (Fn [State Msg] [State (Cmd Msg)]))
(ann subscriptions (Fn [State] (Sub Msg)))
(ann view (Fn [State] Html))
```

Rules:

- `init` is pure and returns initial state plus command data.
- `update` is pure and total over its declared message union.
- `subscriptions` is pure and derives browser resources from state.
- `view` is pure and returns `Html`.
- Event handlers return `Msg`, `Event Msg`, or `nil`.
- Browser work is command or subscription data, not direct host access.

`BrowserBoot` contains explicit host-provided startup values such as current
URL, route params, runtime mode, hydrated state, feature flags, or server
dehydration payloads.

## Feature Modules

Large browser apps decompose into feature modules. A feature module may export:

```clojure
(type State ...)
(type Msg ...)
(ann init (Fn [BrowserBoot] [State (Cmd Msg)]))
(ann update (Fn [State Msg] [State (Cmd Msg)]))
(ann subscriptions (Fn [State] (Sub Msg)))
(ann view (Fn [State] Html))
```

A parent owns child state as an ordinary field and wraps child messages with
explicit tagged union variants. Helpers such as `scope-update`,
`scope-subscriptions`, and `scope-view` are framework helpers that preserve
state paths, command message mapping, subscription message mapping, and template
slot metadata.

There is no mutable component-local state.

## Selectors

Browser selectors are pure typed derived queries:

```clojure
(ann visible-log (Selector AppState (Vector WorkoutEntry)))
(defn visible-log [state]
  (filtered-log state.entries state.logTypeFilter))
```

Selectors:

- are pure functions,
- cannot return commands or subscriptions,
- cannot read host state,
- expose read paths,
- expose selector dependencies,
- may be memoized by the framework runtime,
- feed template slot read-path analysis.

Selectors are framework-level abstraction over pure language functions. The
language core does not know browser state or template slots.

## Templates

`#html` is a browser framework reader form. It is not core language syntax.

`#html` lowers to Template IR with:

- static DOM creation,
- dynamic text slots,
- dynamic attribute slots,
- dynamic class and style slots,
- dynamic event slots,
- refs,
- component slots,
- conditional slots,
- keyed list slots,
- disposal metadata,
- devtools metadata.

The generated runtime updates only slots whose read paths overlap changed state
paths. It does not allocate virtual DOM nodes.

Path overlap rules:

- `state.summary` overlaps `state.summary.value`.
- `state.summary.value` overlaps `state.summary`.
- `state.summary.label` does not overlap `state.summary.value`.

## Event Handlers

Template event handlers are pure expressions evaluated with a typed read-only
event payload.

```clojure
on:submit={(Event.prevent {:kind :load})}

on:keydown={(if (= event.key "Enter")
                (Event.prevent {:kind :commit
                                :value event.currentTarget.value})
                nil)}
```

Direct calls such as `event.preventDefault` and `event.stopPropagation` are not
allowed in Closkell source. Event control is data returned to the runtime.

## Browser Commands

`Cmd Msg` describes one-shot browser work. Command helpers are pure
constructors.

Examples include:

- HTTP requests,
- storage reads and writes,
- DOM ref focus and measure,
- file import and export,
- clipboard write,
- canvas draw,
- time and random values,
- media query setup,
- scroll actions,
- Bluetooth operations,
- history writes.

Command records use concrete `:kind` values. Completion messages are checked
against the app's message type.

## Browser Subscriptions

`Sub Msg` describes long-lived browser resources derived from current state.

Examples include:

- timers,
- animation frames,
- media queries,
- resize observers,
- window events,
- server-sent events,
- WebSocket streams.

The browser runtime diffs subscriptions by stable id and kind, starts new
resources, preserves unchanged resources, stops removed resources, and disposes
everything when the app is disposed.

## JS Interop

Browser JS libraries may be imported with normal Closkell interop. Pure
deterministic functions can use `foreign pure`. Browser effects use command or
subscription wrappers.

`innerHTML` requires `TrustedHtml`. Sanitizers such as DOMPurify can expose
`TrustedHtml` only through explicit foreign declarations.

## Styling

Templates support:

- static `class` and `style`,
- dynamic class strings,
- dynamic class maps, vectors, and sets,
- dynamic style strings,
- dynamic style records and maps,
- CSS custom properties,
- normalized camelCase and kebab-case style keys.

Boolean attributes are bare or dynamic booleans. Misleading static booleans
such as `disabled="false"` are errors. `ref` registers a runtime ref and is not
emitted as a DOM attribute.

## Runtime

The browser runtime surface includes:

- `startApp`,
- `hydrateApp`,
- command handler registration,
- subscription handler registration,
- compiled template component helpers,
- slot update helpers,
- devtools hooks and overlay.

Runtime invariants:

- no `update` call after disposal,
- late async command completions after disposal are ignored,
- event listeners and refs are removed with template instances,
- keyed child rows are disposed when removed,
- slot values are cached to avoid redundant DOM writes,
- devtools events describe app, state, command, subscription, template, and
  disposal activity.

## SSR And Hydration

The same Template IR can support client DOM, server rendering, hydration, and
inspection.

Server rendering produces HTML plus optional serialized initial state and
template metadata. Browser-only commands and subscriptions do not execute
during server rendering.

Hydration attaches runtime instances to existing DOM and then uses normal slot
updates. `view` remains pure and DOM-free in both environments.

## Inspection

Browser framework inspection extends language inspection with:

- app contract exports,
- message unions,
- command schemas,
- subscription schemas,
- selector graph,
- component graph,
- template slots,
- state paths to slots,
- changed-path summaries,
- browser capability usage,
- trusted and unchecked HTML boundaries.

## Testing

The browser framework adds test harnesses for:

- reducers,
- selectors,
- command constructors,
- subscriptions,
- presentational components,
- mounted apps,
- event control,
- slot updates and skipped slots,
- cleanup after disposal.

Pure tests remain language tests. DOM and mounted app tests are framework tests.

## Non-Goals

- React hooks.
- Signals.
- Proxy-based reactivity.
- Class components.
- Mutable component-local state.
- VDOM reconciliation as the framework core.
- Browser globals in pure Closkell code.
- Hidden context as a default abstraction.
- Fullstack coupling to backend route declarations.
- Hidden RPC or server actions generated from frontend code.

## Research Sources

- [Elm Guide: The Elm Architecture](https://guide.elm-lang.org/architecture/)
- [Elm Guide: Commands and Subscriptions](https://guide.elm-lang.org/effects/)
- [Elm Guide: JavaScript Interop](https://guide.elm-lang.org/interop/)
- [Elm Guide: Modules](https://guide.elm-lang.org/webapps/modules)
- [Elm `Browser` package documentation](https://package.elm-lang.org/packages/elm/browser/latest/Browser)
- [re-frame: The re-frame data loop](https://day8.github.io/re-frame/a-loop/)
- [re-frame: Application state](https://day8.github.io/re-frame/application-state/)
- [re-frame API: Event handlers](https://day8.github.io/re-frame/api-re-frame.core/)
- [re-frame: Effects](https://day8.github.io/re-frame/EffectfulHandlers/)
- [re-frame: Coeffects](https://day8.github.io/re-frame/Coeffects/)
- [re-frame: Subscriptions](https://day8.github.io/re-frame/subscriptions/)
- [re-frame: App structure](https://day8.github.io/re-frame/App-Structure/)
