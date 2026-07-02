# Closkell Language Core

This document describes the implementation in this repository.

Closkell is a Lisp-shaped language that compiles to JavaScript ESM. The core
owns parsing, macro expansion, type checking, purity validation, inspection,
module tests, and JS emission. Browser and server concepts are compiler targets
layered on top of the core.

## Compiler Crates

- `syntax`: Lisp syntax, literals, source spans, diagnostics, and the `#html`
  reader form consumed by the browser target.
- `macro_expand`: deterministic macro expansion for `defmacro`,
  quasiquote/unquote, `gensym`, `with-gensyms`, and `compile-error`.
- `typecheck`: local inference, top-level annotations, type declarations,
  command/subscription checking, and target-specific type hooks.
- `effects`: purity/effect validation for registered command helpers and
  forbidden host symbols.
- `template_ir`: browser template lowering and state-read metadata.
- `js_emit`: JavaScript ESM emission, source maps, runtime import planning, and
  app wrappers.
- `cli`: `check`, `build`, `expand`, `fmt`, `inspect`, `test`, and
  `dev --watch`.

## Modules

A `.clsk` file is a module. Top-level forms:

```clojure
(import path [names...])
(type Name schema)
(ann name schema)
(foreign pure importedName schema)
(foreign task importedName schema)
(foreign command importedName schema)
(def name expr)
(defn name [params...] body...)
(defmacro name [params...] body...)
```

Imports can reference same-tree Closkell modules or JS/runtime module names.
Imported names may be aliased with `(name as localName)`.

`type`, `ann`, and `foreign` are compile-time declarations. They are checked and
included in `inspect`, but they do not emit runtime declarations by themselves.

## CLI Targets

The CLI accepts `--target core|browser|server` for `check`, `build`, `inspect`,
and `dev --watch`. The default target is `browser`.

- `core`: rejects `#html`, browser helpers, and server helpers.
- `browser`: enables `Html`, `TrustedHtml`, `BrowserBoot`, `Cmd`, `Sub`,
  browser template checking, browser command schemas, and browser emit rules.
- `server`: enables `Request`, `Response`, `Route`, `ServerResource`,
  `ServerResources`, `ServerBoot`, and `HttpError` helpers.

`build --app --target browser` expects `init`, `update`, and `view`.
`build --app --target server` expects `main`.
`build --app --target core` is rejected.

## Syntax

The parser supports:

```clojure
nil true false
42 60_000 "text" :keyword symbol
[a b c]
{:field value "dynamic-key" value}
#{a b c}
(fn [x] (+ x 1))
(let [x 1 y 2] (+ x y))
(if condition then else)
(match value pattern result _ fallback)
```

Rules:

- `if` conditions are checked as `Bool`; there is no truthiness rule.
- Record field access uses dotted syntax such as `state.user.name`.
- Dynamic maps use helpers such as `hash-map`, `map-get`, `map-assoc`, and
  `map-dissoc`.
- `#html` is parsed by `syntax`, but it is only valid when the browser target
  enables it.

## Types

Implemented type forms include:

- `Number`, `String`, `Bool`, `Nil`, `Keyword`, and `Js`.
- `Option T`, where `nil` is the none value.
- `Result Ok Err`.
- `Decoder T`.
- `Vector T`, `List T`, `Set T`, and `Map K V`.
- `Cmd Msg`, `Sub Msg`, `Task Err Ok`, and `Event Msg`.
- Tuple syntax `[A B C]`.
- Structural records such as `{:id String :label String}`.
- Tagged unions with `(Union ...)`.
- Function types with `(Fn [Args...] Return)`.
- Parametric aliases, for example `(type RemoteData a ...)`.

`Js` is an explicit boundary type. `unsafe-cast` marks a source-level escape
hatch, and unsafe casts are collected by `inspect`.

## Patterns

Patterns are supported in `match`, `let`, anonymous `fn` parameters, and named
`defn` parameters. Pattern forms:

- wildcard `_`
- literals and keywords
- records
- vectors
- fixed `(list ...)`
- `(cons head tail)`
- `(as pattern name)`
- `(some pattern)`, `(ok pattern)`, and `(err pattern)`

Template component metadata is most precise when component parameters are plain
symbols. Destructuring works in many places; reusable `#html` components keep
the clearest update metadata with symbol parameters.

## Macros

Macros expand before type checking. Macro-time forms include `do`, `let`,
`gensym`, `with-gensyms`, quasiquote/unquote, unquote-splicing, and
`compile-error`.

Imported macros from same-tree modules are available during expansion. Macro
definitions do not remain as runtime exports.

## Purity And Effects

The compiler does not allow registered host effects to hide inside ordinary
pure code. Browser and server targets add their own forbidden symbols and
allowed command/resource helpers.

Effectful work is represented as data:

- browser commands and subscriptions in the browser target,
- task/command values and foreign declarations,
- server route/response/resource values in the server target.

The boundary is implementation-driven: if a host helper is not registered in
the target, the compiler cannot give it special effect semantics.

## Implemented Library Surface

Implemented helpers:

- numbers: arithmetic, comparison, `min`, `max`, `sum`, `abs`, `round`,
  `floor`, `ceil`, `mod`/`%`, `to-number`, and `to-fixed`;
- strings: `str`, `trim`, `lower-case`, `pad-start`, `to-radix`,
  `string-slice`, `regex-test?`, `includes?`, and `locale-compare`;
- vectors/lists: `first`, `second`, `nth`, `last`, `find`, `filter`, `map`,
  `map-indexed`, `reduce`, `reduce-indexed`, `any?`, `every?`, `slice`,
  `drop-last`, `take-last`, and sorting helpers;
- sets/maps: set literals, `set`, `contains?`, `conj`, `disj`, `set-values`,
  `hash-map`, `map-get`, `map-assoc`, `map-dissoc`, `map-entries`,
  `map-keys`, and `map-values`;
- results/options: `ok`, `err`, `ok?`, `err?`, `some?`, `result-value`,
  `result-error`, and `unwrap-or`;
- JSON/decoding: `json-parse`, `json-stringify`, `json-parse-result`, and
  `Decoder` helpers;
- date formatting through `date-format` when inputs are explicit.

Collections are emitted as fresh JS values on update. The compiler/runtime do
not rely on mutating persistent data in place.

## Inspection

`closkell inspect` emits JSON containing:

- imports and exports,
- public signatures,
- type declarations and annotations,
- command and subscription schemas,
- JS interop declarations,
- component graphs,
- state-path-to-slot metadata,
- changed-path summaries,
- unsafe casts,
- collected tests,
- unused declarations,
- lowered templates.

This JSON is used by the Vite plugin and editor tooling.

## Tests

`closkell test <file>` compiles a module to temporary ESM and runs exported
tests under Node. Tests can be exported as a `tests` vector or authored with the
helpers from `closkell/test`.

The example projects have their own runtime and Playwright suites; they are not
part of the language core.
