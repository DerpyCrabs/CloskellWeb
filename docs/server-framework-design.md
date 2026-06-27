# Closkell Server Framework Design Target

This document defines the backend service framework built on top of the
Closkell language core. It is separate from the browser framework.

The server framework is not a fullstack framework. Browser apps and backend
services are independent programs. They can share pure modules, types,
validators, encoders, decoders, and protocol helpers, but they do not share
implicit routes, hidden RPC, or generated server actions.

## Service Contract

A backend service module exports:

```clojure
(ann init (Fn [ServerBoot] [ServerState (Cmd ServerMsg)]))
(ann routes (Fn [ServerState] (Vector Route)))
(ann resources (Fn [ServerState] (ServerResources ServerMsg)))
```

`ServerBoot` contains host-provided values:

- command-line arguments,
- environment variables,
- resolved config paths,
- runtime mode,
- port and host,
- working directory,
- secrets supplied by the host,
- selected Bun or Node runtime details.

Pure code cannot read process globals directly.

## Routes

Routes are data. A route declares:

- method,
- path pattern,
- typed params,
- typed query,
- typed headers,
- typed cookies,
- typed body decoder,
- accepted multipart fields when relevant,
- handler.

Handlers are pure functions that return effect descriptions:

```clojure
(ann media-route
  (Route
    {:params {:path String}
     :headers {:range (Option String)}}
    (Task HttpError Response)))
```

A handler may return `[ServerState (Cmd ServerMsg) Response]` when it
intentionally updates process state. Ordinary request handling should prefer
explicit tasks and stores over mutable module state.

## Responses

Response values include:

- JSON,
- text,
- redirects,
- static files,
- byte ranges,
- binary buffers,
- streaming bodies,
- server-sent event streams,
- empty responses with status and headers.

Status codes, headers, cookies, content disposition, content range, and cache
control are explicit response data.

## Server Capabilities

Backend work is represented as server capabilities, not pure calls.

Core server capabilities include:

- filesystem read, write, stat, mkdir, rename, copy, remove, and directory walk,
- file streams and byte-range streams,
- multipart upload parsing,
- process spawning for tools such as `ffmpeg`,
- crypto, hashing, signing, and password verification,
- cookie signing, setting, and clearing,
- archive generation,
- image processing through JS libraries such as sharp,
- audio metadata through JS libraries such as music-metadata,
- HTTP client requests,
- AI SDK calls,
- MCP client calls,
- timers, intervals, file watches, and connection keepalives,
- structured logging,
- controlled process exit.

Each capability is represented as `Task`, `Cmd`, `ServerResource`, or a typed
foreign declaration. Impure operations must not be declared `foreign pure`.

## Server Resources

Long-lived backend resources are `ServerResource` values. They are scoped to:

- the service process,
- a request,
- a streaming response,
- a connection.

Examples:

- listening HTTP server,
- file watcher,
- polling interval,
- SSE client,
- upload stream,
- download stream,
- spawned process,
- MCP client,
- thumbnail generation worker.

The runtime owns acquisition, reuse, cancellation, and cleanup. Request and
connection close events cancel scoped resources.

## JS Server Libraries

The backend target compiles to JavaScript ESM for Bun or Node-compatible hosts.
Reusing JS libraries is expected.

Useful wrappers include:

- Fastify-compatible route registration,
- Bun server adapters,
- Node stream adapters,
- sharp,
- archiver,
- music-metadata,
- AI SDK,
- MCP SDK,
- JSON-with-comments parsers.

JS libraries remain host capabilities. Their impure operations are visible in
source and inspection output.

## Configuration

Configuration is explicit host input. The server framework may provide helpers
for:

- command-line arguments,
- environment variables,
- JSON and JSONC files,
- resolved data paths,
- secret values,
- runtime mode.

Config loading itself is task data. Pure modules receive parsed config values as
parameters or boot data.

## Frontend Communication

Frontend and backend communication is explicit protocol traffic:

- HTTP requests,
- server-sent events,
- WebSocket messages,
- static files,
- streamed media.

The browser framework may import shared request and response types, but it calls
the backend through ordinary command or subscription data. There is no implicit
client stub generation requirement.

## Inspection

Server framework inspection extends language inspection with:

- routes,
- request schemas,
- response schemas,
- body decoders,
- multipart schemas,
- emitted status codes,
- cookie usage,
- filesystem capability usage,
- process capability usage,
- stream/resource lifetimes,
- JS interop boundaries,
- unsafe casts,
- server tests.

## Testing

The server framework adds test harnesses for:

- route handlers,
- body decoders,
- response encoders,
- filesystem capabilities with fake filesystems,
- process capabilities with fake processes,
- fake clock and timers,
- fake crypto where appropriate,
- fake network,
- fake streams,
- SSE connection cleanup,
- multipart uploads,
- config loading.

Pure business logic remains ordinary language-level tests.

## Non-Goals

- Fullstack route coupling.
- Hidden RPC.
- Server actions generated from frontend source.
- Direct process globals in pure code.
- Direct filesystem calls in pure code.
- Treating Fastify or Bun APIs as language syntax.
- Requiring the browser framework to use the server framework.
