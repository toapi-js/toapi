# @toapi/vite-plugin

Vite plugin for [Toapi](https://github.com/toapi-js/toapi). Bundles your Toapi
API alongside your Vite frontend in a single project: serves the API as
middleware in dev and preview mode, and produces a deployable server bundle
for production.

📚 [Docs](https://toapi-js.github.io/toapi/vite-plugin/)

## Installation

```bash
pnpm add -D @toapi/vite-plugin
pnpm add @toapi/server srvx
```

Peer-deps: `vite ^8`, `@toapi/server`.

## Usage

`vite.config.ts`:
```ts
import { defineConfig } from "vite";
import toapi from "@toapi/vite-plugin";

export default defineConfig({
  plugins: [toapi()],
});
```

The plugin expects `src/api.ts` to export `api` (an `ApiDefinition`):
```ts
import { defineApi, defineHandler, TResponse } from "@toapi/server";

export const api = defineApi().route("/hello", {
  GET: defineHandler({ authorize: () => true }, async () => {
    return TResponse.json({ message: "hello" });
  }),
});
```

In dev (`vite`) and preview (`vite preview`) modes, the plugin attaches a
middleware to Vite's server that handles requests at the configured
`basePath` (default `/api`). The same Vite server serves both the frontend
and the API on a single port.

## Options

| Option | Type | Default | Description |
|---|---|---|---|
| `entry` | `string` | `"src/api.ts"` | Path to the file exporting `api`. Resolved against the Vite root. |
| `basePath` | `string` | `"/api"` | Prefix for API routes. Use `""` to mount at the root. |
| `port` | `number` | — | Default port for Vite's dev/preview server. Falls back to the `PORT` env var. |
| `external` | `(string \| RegExp)[]` | `[]` | Packages to keep external in the server bundle. By default everything is bundled. |

## Environment variables

### Dev (`pnpm dev`)

The plugin loads `.env`, `.env.local`, `.env.<mode>`, and `.env.<mode>.local`
via Vite's `loadEnv` and mirrors every key into `process.env` before the api
module is loaded. Existing `process.env` values keep precedence, so shell
vars override `.env` files.

```
.env                     # all envs
.env.local               # all envs, ignored by git
.env.development         # dev only
.env.development.local   # dev only, ignored by git
```

This covers server-side libraries that read `process.env.X` directly
(BetterAuth, DB clients, OAuth secrets). Vite's `import.meta.env` injection
still applies to client code.

### Production (`srvx dist/server/server.js`)

The built server does **not** load `.env` files. In production, env vars come
from the runtime — Docker, systemd, or your PaaS. 


## Build output

`vite build` writes two clearly separated trees:

```
dist/
├── client/        — static frontend assets (HTML, JS, CSS, images)
├── server.js      — bundled server (sourcemap included)
└── server.js.map
```

The split exists for safety: server-only code (database credentials,
third-party API keys, server-side libraries) lives in `dist/server.js` and
**must not** be deployed to a public static host. Treating `dist/client/` as
the static-deploy root makes it impossible to leak the server bundle by
accident.

## Service worker

To get tapi's offline / tag-based revalidation behavior, add a service
worker built by [`vite-plugin-pwa`](https://vite-pwa-org.netlify.app/) in
`injectManifest` mode. The two plugins compose cleanly: `tapi()` redirects
the client build to `dist/client/`, which is exactly where VitePWA emits
`sw.js`, and the production server bundle is built separately so
nothing leaks across.

```bash
pnpm add -D vite-plugin-pwa
```

```ts
// vite.config.ts
import { defineConfig } from "vite";
import tapi from "@toapi/vite-plugin";
import { VitePWA } from "vite-plugin-pwa";

export default defineConfig({
  plugins: [
    tapi(),
    VitePWA({
      strategies: "injectManifest",
      srcDir: "src",
      filename: "service-worker.ts",
      injectRegister: "auto",
      devOptions: { enabled: true, type: "module" },
      // optional, pass-through to VitePWA:
      // manifest: { name: "My App", short_name: "App", ... },
    }),
  ],
});
```

```ts
// src/service-worker.ts
import {
  handleTapiRequest,
  listenForInvalidations,
  cleanup,
} from "@toapi/worker";

declare const self: ServiceWorkerGlobalScope;

self.addEventListener("activate", (event) => {
  // Drop cache entries that have been expired longer than 7 days,
  // remove orphans, and rebuild the tags index.
  event.waitUntil(cleanup({ maximumStaleAge: 60 * 60 * 24 * 7 }));
});

self.addEventListener("fetch", (event) => {
  const url = new URL(event.request.url);
  if (
    url.pathname.startsWith("/api") &&
    !url.pathname.startsWith("/api/__tapi")
  ) {
    event.respondWith(handleTapiRequest(event.request));
  }
});

listenForInvalidations({ url: "/api/__tapi/invalidations" });
```

Notes:

- Add `"WebWorker"` to your `tsconfig.json` `lib` array so TypeScript
  recognizes `ServiceWorkerGlobalScope` and friends.
- `devOptions.enabled: true` makes the SW run during `vite dev` too;
  otherwise it only runs in `vite preview` and production.
- Adjust the `/api` checks in the SW to match the `basePath` you pass to
  `tapi()`.
- `cleanup`'s `maximumStaleAge` is the grace period (in seconds) past a
  cache entry's `expiresAt` before it's actually deleted on next SW
  activation.

## Deployment

The server bundle is a fetch-handler module. Serve it in production with the
[srvx](https://srvx.h3.dev) CLI:

```bash
srvx --prod dist/server.js
```

`srvx` picks up `PORT`, `HOST`, and other settings from environment variables.

### Static assets

Static frontend assets in `dist/client/` should ideally be served by a
dedicated static host — nginx, Caddy, or an S3-compatible bucket fronted by a
CDN. Dedicated static hosts give you better caching, compression, HTTP/2,
and offload the load from your Node.js server.

If you don't want a separate static host, srvx can serve them too with the
`-s` flag:

```bash
srvx --prod -s client dist/server.js
```

The path passed to `-s` is resolved **relative to the directory containing
the entry file** (`dist/`), so `client` points at `dist/client/`.
API routes are matched first; static files fall through when no API route
handles the request.

> **Note:** `srvx -s` serves files as-is. It has **no SPA history fallback**
> (deep-linking or reloading a client-side route returns `404`) and sets **no
> `Cache-Control`/`ETag`** — content-hashed assets are not marked `immutable`
> and there is no `304` revalidation. For a client-routed SPA in production,
> front it with a static server or CDN that handles the history fallback and
> caching. See the reference `Caddyfile` in
> [`examples/vite-plugin-tapi-demo`](https://github.com/toapi-js/toapi/tree/main/examples/vite-plugin-tapi-demo).
