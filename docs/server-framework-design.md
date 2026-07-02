# Closkell Server Target

The server target compiles Closkell modules for Node-compatible backend code.
The `projects/derp-media-server` example exercises that design with a
Closkell-authored Fastify server, routes, filesystem helpers, streaming
responses, SSE handlers, upload handlers, auth middleware, and thumbnail/media
helpers built to `.closkell/build/server/main.mjs`.

The reusable framework layer in `crates/js_server` provides server-target type
checking, app wrapping, and route/response/resource data helpers.
Project-specific host adapters, such as Fastify registration and raw Node
stream handling, live in application Closkell modules.

Server-target compilation is selected with `--target server`.

## Reusable Framework Boundary

Provided by `crates/js_server`:

- server-only type aliases and helper types,
- route helper type checking and JS emission,
- response helper type checking and JS emission,
- server resource helper type checking and JS emission,
- rejection of server helpers from non-server targets,
- `build --app --target server` wrapper for `main`.

Handled outside the reusable framework layer:

- an HTTP listener adapter,
- request parsing and validation,
- file/multipart/stream execution,
- cookie/session helpers,
- filesystem/process wrappers,
- automatic client generation,
- hidden RPC or server actions.

The media server implements several of these concerns in project-local
Closkell modules by importing Fastify, Node APIs, and JS libraries directly.
Those adapters remain application code rather than reusable `crates/js_server`
framework primitives.

## App Shape

`build --app --target server` expects an exported `main`.

If `main` takes one argument, the generated module exports and passes:

```clojure
ServerBoot
```

`ServerBoot`:

```clojure
{:argv (Vector String)
 :cwd String
 :env Js
 :mode String
 :runtime String}
```

The JS wrapper emits:

- `__closkellServerBoot`
- `__closkellServerResult`

## Server Types

The server target adds:

- `ServerBoot`
- `HttpError`
- `Request`
- `Response`
- `(Route RequestType TaskType)`
- `(ServerResource Msg)`
- `(ServerResources Msg)`

`Request` is opaque at the reusable framework boundary. Typed request bodies,
params, query, and headers remain application/adapter concerns.

## Route Helpers

Route helpers:

```clojure
(Route.route method path request handler)
(Route.get path handler)
(Route.post path handler)
(Route.put path handler)
(Route.patch path handler)
(Route.delete path handler)
```

`Route.get` and the method-specific helpers use opaque `Request` and expect a
handler returning `(Task HttpError Response)`.

The emitted JS route value is data:

```js
{ kind: Symbol.for("server/route"), method, path, request, handler }
```

## Response Helpers

Response helpers:

```clojure
(Response.json body)
(Response.json body status)
(Response.text text)
(Response.text text status)
(Response.empty)
(Response.empty status)
(Response.redirect location)
(Response.redirect location status)
(Response.file path)
(Response.file path options)
(Response.status status response)
```

The emitted JS response values are data with `Symbol.for("response/...")`
kinds. `Response.status` returns a copy of the response with a changed status.

## Resource Helpers

Resource helpers:

```clojure
(Server.resource name config)
(Server.resource name config onEvent)
(Server.resources resources)
```

`Server.resources` accepts a vector or list of resources. The emitted JS values
are data with `Symbol.for("server/resource")` and
`Symbol.for("server/resources")` kinds.
