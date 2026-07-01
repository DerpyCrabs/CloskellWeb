# Media Server

Self-hosted media library with a **Closkell** + Vite web UI and a **Fastify** API on **Node.js**. Browse, play, and edit files; optional password auth; token-based shares; workspaces with multi-pane layout; knowledge-base folders with search and Obsidian-style markdown. Changes propagate to open tabs via **SSE**.

## Features (high level)

- Workspaces: snap zones, viewers (image, video, PDF, text), audio player, persisted layout (admin and share views).
- Shares: tokens, optional passcodes, editable shares with per-permission toggles and upload quota.
- Knowledge bases: full-text search, recent files, `![[image]]` from `images/`.
- File ops in editable folders: upload, move/copy, rename, delete, inline text edit; grid/list, thumbnails (FFmpeg optional), drag-and-drop.
- Auth: session cookies, rate-limited login, optional admin hostname allowlist; shares stay reachable regardless.

## Quick start

**Needs:** Node.js and npm. **Optional:** FFmpeg for video thumbnails, audio-only video playback and tests.

```bash
npm install
```

Create `config.jsonc` (JSON with comments; falls back to `config.json`):

```jsonc
{
  "mediaDir": "/path/to/your/media",
  "editableFolders": ["notes", "documents"],
  "auth": {
    "enabled": true,
    "password": "your-secret",
    "adminAccessDomains": ["127.0.0.1"],
  },
}
```

```bash
npm run dev
```

Open [http://localhost:3000](http://localhost:3000).

## Configuration

Path: `CONFIG_PATH` or `--config-path=...`. Options can also be set via environment variables (and `.env`).

| Config                    | Env                         | Purpose                                                                     |
| ------------------------- | --------------------------- | --------------------------------------------------------------------------- |
| `mediaDir`                | `MEDIA_DIR`                 | Media root for legacy/single-root configs                                   |
| `mediaDirs`               |                             | Multiple named media roots, each with optional editable folders             |
| `editableFolders`         | `EDITABLE_FOLDERS`          | Comma-separated paths under single-root `mediaDir` where writes are allowed |
| `shareLinkDomain`         | `SHARE_LINK_DOMAIN`         | Base URL for share links (host or full URL)                                 |
| `auth.enabled`            | `AUTH_ENABLED`              | `true` / `1`                                                                |
| `auth.password`           | `AUTH_PASSWORD`             | Login password                                                              |
| `auth.adminAccessDomains` | `AUTH_ADMIN_ACCESS_DOMAINS` | Comma-separated hostnames for admin UI/API                                  |
| `auth.secureCookies`      | `AUTH_SECURE_COOKIES`       | Require HTTPS for login cookies; defaults to production only                |

`dataPath` (shares DB, etc.) is config-file only; defaults next to the config file.

Use `mediaDirs` when serving multiple media roots:

```jsonc
{
  "mediaDirs": [
    { "path": "D:/Media/Movies", "name": "Movies", "editableFolders": ["Incoming"] },
    { "path": "E:/Shows", "editableFolders": ["Downloads", "Notes"] },
  ],
}
```

When more than one media root is configured, the browser root shows each media directory
as a folder. Paths are prefixed by the root name, for example `Movies/Incoming`.
`name` is derived from the directory basename when possible, but must be set explicitly
if the basename is empty, duplicates another media root, or conflicts with a virtual
folder such as `Favorites`, `Most Played`, or `Shares`.

## Production

```bash
npm run build
npm run start
```

Listens on `0.0.0.0` by default.

## Development

- Closkell check: `npm run check:closkell`
- E2E: `npm run test` (single worker) or `npm run test:batch` (CI-style batches)
- Unit: `npm run test:unit`

## Stack

Closkell, Closkell web framework, Vite, Tailwind CSS v4, Fastify, Node.js, TypeScript tests, Playwright.

## License

MIT
