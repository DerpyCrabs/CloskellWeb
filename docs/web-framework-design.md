# Closkell Browser Target

The browser target combines compiler support, a Vite plugin, and the JavaScript
runtime in `runtime-js`. It is the default CLI target and is also selected by
`--target browser`.

## App Shape

`build --app --target browser` requires an entry module that exports:

```clojure
(def init ...)
(defn update [state msg] ...)
(defn view [state] ...)
```

`subscriptions` is optional. `init` and `update` return `[State (Cmd Msg)]`.
`view` returns `Html`. The compiler wraps the module with a browser entry that
imports the runtime, optional CSS, and starts the app at the selected root id.

Browser app build flags:

```text
--app
--root <id>
--css <path>
--vendor-runtime
--sourcemap
```

The Vite plugin lets apps import `.clsk` files directly and exposes the
inspection report at `/__closkell/inspect` during dev.

## Browser Types

The browser target adds:

- `Html`
- `TrustedHtml`
- `BrowserBoot`
- `Cmd Msg`
- `Sub Msg`
- `Event Msg`

The `BrowserBoot` type alias is:

```clojure
{:currentUrl String
 :host String
 :path String
 :query String}
```

The `build --app` wrapper passes `currentUrl`. `host`, `path`, and `query`
belong to explicitly constructed boot values or to values derived from
`currentUrl`.

## Templates

`#html` lowers to Template IR and then to specialized DOM update code. The
runtime updates DOM slots directly; it does not allocate virtual DOM nodes.

Implemented template features include:

- static DOM creation,
- dynamic text and attributes,
- `textContent` and nullable text properties,
- dynamic `class` strings, vectors, sets, records, and maps,
- dynamic `style` strings, records, maps, camelCase keys, kebab-case keys, and
  CSS custom properties,
- boolean and presence attributes,
- `ref` registration,
- event handlers,
- conditional fragments,
- component calls,
- keyed lists with optional loop index,
- state-read metadata for slot skipping,
- hydration of matching server/static DOM through `data-closkell-template`.

Bare `ref`, `class`, and `style` attributes are rejected. Static boolean
strings such as `disabled="false"` are rejected because browsers treat boolean
attributes as enabled by presence.

`innerHTML` requires `TrustedHtml`; plain strings are rejected.

## Events

Event handlers are expressions in template scope. They can read a typed `event`
object and return a message, `nil`, or event-control data.

Event-control constructors:

```clojure
(Event.prevent {:kind :submit})
(Event.stop {:kind :clicked})
(Event.prevent-stop {:kind :hotkey})
```

Direct browser event mutation such as `event.preventDefault` is rejected by the
browser target.

## State Paths And Slot Updates

Template lowering records the state paths read by each slot. At runtime, app
dispatch compares old and new state, computes changed paths, and skips slots
whose reads do not overlap the changed paths.

Path overlap rules:

- `state.summary` overlaps `state.summary.value`.
- `state.summary.value` overlaps `state.summary`.
- `state.summary.label` does not overlap `state.summary.value`.

The lowering also projects reads through local `let` bindings, destructuring,
component calls, and simple helper calls when their read summaries are known.

## Commands

`Cmd Msg` is a data description of one-shot browser work. Command helper and
schema families:

- `Cmd.none`, `Cmd.batch`, and `Cmd.map`;
- timers and animation frames;
- time reads and random numbers;
- storage get/set/remove;
- HTTP requests;
- file download, import, selected-file reads, and blob reads;
- DOM refs: focus, click, measure, resize watch, input selection, and scroll;
- canvas draw and text measurement;
- browser navigation, location assignment, URL opening, document title,
  scrolling, cookies, clipboard, theme helpers, and EventSource open/close;
- media element helpers;
- Bluetooth heart-rate connection and disconnect;
- simulation heart-rate commands for development/tests.

Command records must include concrete `:kind` values when they are checked
against a `Cmd Msg` annotation. Completion messages are checked against the
declared message type when schemas are registered.

## Subscriptions

`Sub Msg` is a data description of long-lived browser resources. Subscription
families:

- `Sub.none`
- `Sub.batch`
- `Sub.timer/every`
- `Sub.media-query`
- `Sub.window/event`
- `Sub.window/event-with`
- `Sub.dom-ref/resize`
- simulation and Bluetooth heart-rate subscriptions

The runtime diffs subscriptions, starts new resources, keeps unchanged
resources, and cleans them up when they disappear or the app is disposed.

## Feature Composition

The browser target supports explicit parent/child composition:

- `scope-update`
- `scope-subscriptions`
- `scope-view`
- `Cmd.map`
- `Cmd.batch`
- `Sub.batch`

Child state is ordinary parent state. Child messages are wrapped into parent
message records explicitly. There is no mutable component-local state system.

## Runtime

The runtime exports both general and compiled-template APIs. The generated code
mainly uses compiled-template helpers such as `createCompiledTemplateComponent`,
slot setters, keyed list setters, command handler registration, and
`startCompiledApp`.

Runtime invariants:

- app `dispose()` removes mounted DOM, refs, listeners, command resources, and
  subscriptions;
- late async command completions are ignored after disposal;
- removed keyed rows dispose their component instances;
- event listeners and refs are tied to template lifetime;
- devtools hooks receive app, state, command, subscription, template, and
  disposal events.

## Testing

Browser-facing test surfaces:

- `closkell test` for pure/runtime tests exported from `.clsk` modules,
- project-level Playwright suites for the example apps,
- runtime harness helpers exported by `runtime-js`, including render, rerender,
  DOM query helpers, event dispatch, collected messages, commands, and
  subscriptions.

## Excluded Model

- React hooks.
- Signal or Proxy tracking.
- Virtual DOM reconciliation as the update strategy.
- Hidden browser globals in pure code.
- Hidden fullstack RPC.
- Server actions generated from frontend source.
