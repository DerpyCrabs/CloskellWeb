# Closkell Language Design Target

This document defines the framework-neutral Closkell language core. It is not a
browser framework design and not a server framework design.

Closkell is a pure functional, Lisp-shaped language that compiles to JavaScript
ESM. Its job is to provide a small, reviewable, strongly checked core for pure
data transformation and explicit host boundaries. Browser and server behavior
is supplied by libraries and runtimes built on top of this core.

## Core Boundary

The language core owns:

- modules and imports,
- lexical binding,
- expressions,
- pattern matching,
- immutable records and collections,
- type declarations and annotations,
- tagged unions,
- hygienic macros,
- purity checking,
- explicit JS interop declarations,
- compiler diagnostics,
- inspection JSON,
- JavaScript ESM emission.

The language core does not own:

- `Html`,
- `#html` semantics,
- DOM events,
- CSS class or style normalization,
- browser commands,
- browser subscriptions,
- hydration,
- HTTP route registration,
- request/reply objects,
- filesystem access,
- process spawning,
- cookies,
- server streams,
- server resources.

Frameworks may add reader forms, types, macros, and runtime helpers. Those
extensions must remain visible in source and inspection output.

## Purity

Ordinary Closkell functions are referentially transparent. They cannot mutate
objects, mutate DOM, read browser globals, read process globals, start timers,
fetch, write storage, read files, write files, inspect the clock, read random
values, spawn processes, or perform hidden I/O.

Host work is represented by explicit values supplied by framework libraries:

- task-like async descriptions,
- command-like one-shot effect descriptions,
- resource descriptions for long-lived host resources,
- boot inputs provided by a host runtime,
- explicit JS interop declarations.

The core language enforces the boundary. Concrete host capabilities live in
framework packages such as `closkell/browser` and `closkell/server`.

## Modules

A `.clsk` file is a module. Top-level forms are:

```clojure
(import path [names...])
(type Name schema)
(ann name schema)
(def name expr)
(defn name [params...] body...)
(defmacro name [params...] body...)
(foreign pure importedName schema)
(foreign task importedName schema)
```

Top-level side effects are forbidden. A top-level `def` may bind pure
constants, pure functions, macros, types, annotations, and framework data
constructors. It must not execute host work.

Imports are explicit. Exported application and framework boundaries should have
annotations.

## Syntax

The core syntax includes:

```clojure
nil true false
42 60_000 "text" :keyword symbol
[a b c]
{:field value "dynamic" value}
#{a b c}
(fn [x] (+ x 1))
(let [x 1 y 2] (+ x y))
(if condition then else)
(match value pattern result _ fallback)
```

Rules:

- `if` conditions are `Bool`; there is no truthiness.
- Record field access uses `value.field`.
- Dynamic lookup uses explicit helper functions.
- Pattern binding is allowed in `let`, `fn`, `defn`, and `match`.
- Reader forms such as `#html` are framework extensions, not core language
  constructs.

## Types

The type system supports:

- `Number`, `String`, `Bool`, `Nil`, and `Js`.
- `Option T`, with `nil` as the none value.
- `Result Ok Err`.
- `Vector T`, `List T`, `Set T`, `Map K V`.
- Tuple syntax `[A B C]`.
- Structural records `{:field Type}`.
- Tagged unions.
- Function types with `Fn`.
- Parametric aliases and annotation-local type variables.

Example:

```clojure
(type RemoteData a
  (Union
    {:kind :idle}
    {:kind :loading}
    {:kind :ready :value a}
    {:kind :failed :error String}))
```

`Js` is a type-checker boundary only. It has no runtime wrapper. A value enters
typed Closkell data through a typed foreign declaration, a decoder, or an
explicit `unsafe-cast`.

## Data And Equality

Records, vectors, lists, sets, and maps are persistent immutable values. Update
helpers return fresh values and never mutate their inputs.

`=` means Closkell value equality. `identical?` means JavaScript identity or
`Object.is`. Numeric comparison functions are numeric unless their names say
otherwise.

Value equality covers records, vectors, lists, sets, maps, keywords, `Option`,
and `Result`.

## Standard Library

The core standard library contains pure helpers for:

- `Option`,
- `Result`,
- immutable records,
- vectors and lists,
- sets and maps,
- strings,
- numbers,
- dates when all inputs are explicit,
- JSON encoding and decoding,
- URL parsing and formatting when all inputs are explicit.

Host reads and writes are not pure standard library functions. Clipboard,
current URL, history, storage, filesystem, DOM, current time, random values,
process environment, and network access belong to framework capabilities.

## JS Interop

Closkell can import JavaScript packages directly:

```clojure
(import "yaml" [(parse as parseYaml)])
(foreign pure parseYaml (Fn [String] Js))
```

Rules:

- Unannotated JS imports have type `Js`.
- `foreign pure` is allowed only for deterministic functions that do not read
  or write host state.
- `foreign task` is for async JS work represented as task data.
- Impure host APIs are exposed by framework capability wrappers.
- TypeScript declarations are not trusted as the Closkell type system.
- `unsafe-cast` is allowed only as an explicit source-level boundary.

## Macros

Macros expand before type checking. Macro expansion is deterministic and
hygienic.

Macro bodies support compile-time helpers such as `do`, `let`, `with-gensyms`,
`gensym`, quasiquote, unquote, and `compile-error`.

Macros may generate framework forms, but generated source remains inspectable
after expansion.

## Compiler And Inspection

The semantic compiler boundary is HIR. Syntax AST remains the parsed source
shape. HIR owns stable ids, resolved names, type checking, purity checks,
effect summaries, and public module summaries.

Inspection JSON is part of the language contract. It includes:

- exports,
- imports,
- types and annotations,
- inferred public signatures,
- macros and expansions,
- purity summaries,
- effect and capability summaries supplied by frameworks,
- JS interop boundaries,
- unsafe casts,
- unused reachable definitions and imports,
- test cases.

Diagnostics are deterministic and machine-readable. They include file, span,
severity, diagnostic code, concise message, expected and actual types when
relevant, and safe fix metadata when the compiler can produce it.

## Testing

Closkell test modules are ordinary `.clsk` modules. Pure tests do not require a
browser or server runtime.

Frameworks can add test harnesses for browser apps, components, routes,
streams, and host capabilities. Those harnesses are framework APIs, not core
language features.

## Conformance

The language core conforms to this target when:

- ordinary functions are pure,
- host work cannot hide inside pure code,
- top-level forms do not execute host work,
- data values are immutable,
- public contracts are type checkable,
- JS interop boundaries are explicit,
- framework extensions are visible in source and inspection output,
- compiler output is deterministic,
- deleting compiler caches cannot change program meaning.
