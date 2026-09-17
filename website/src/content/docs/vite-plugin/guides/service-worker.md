---
title: "Service Worker"
description: "Add offline support and tag-based cache revalidation by composing @toapi/vite-plugin with vite-plugin-pwa."
---

To get Toapi's offline / tag-based revalidation behavior, add a service worker
built by [`vite-plugin-pwa`](https://vite-pwa-org.netlify.app/) in
`injectManifest` mode. The two plugins compose cleanly: `@toapi/vite-plugin`
redirects the client build to `dist/client/`, which is exactly where VitePWA
emits `sw.js`, and the production server bundle is built separately so nothing
leaks across.

Inside the worker the two responsibilities also compose cleanly:

- **VitePWA / Workbox** precaches your **static build output** (the app shell,
  JS, CSS, images) via the manifest it injects as `self.__WB_MANIFEST`.
- **[`setupToapiWorker`](/tapi/worker/reference/setup-toapi-worker/)** handles your
  **Toapi API routes** — caching, offline fallback, and tag-based revalidation.

Because `setupToapiWorker`'s `fetch` listener only responds to same-origin
requests under its `basePath`, everything else — every static asset — falls
through to Workbox's precache route. The two never fight over a request.

## Installation

```bash
pnpm add -D vite-plugin-pwa workbox-precaching
```

`workbox-precaching` provides `precacheAndRoute`, which turns the injected
manifest into a static-asset cache. It ships with Workbox but is a separate
import, so install it explicitly (pnpm does not hoist transitive deps).

## Vite config

```ts
// vite.config.ts
import { defineConfig } from "vite";
import toapi from "@toapi/vite-plugin";
import { VitePWA } from "vite-plugin-pwa";

export default defineConfig({
  plugins: [
    toapi(),
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

Setting `devOptions.enabled: true` makes the service worker run during
`vite dev` as well. Without it the SW only runs in `vite preview` and
production.

## Service worker

Precache the static build output with Workbox, then hand your API routes to
Toapi — two calls, one file:

```ts
// src/service-worker.ts
import { precacheAndRoute } from "workbox-precaching";
import { setupToapiWorker } from "@toapi/worker";

declare const self: ServiceWorkerGlobalScope & {
  // VitePWA injects the precache manifest here in injectManifest mode.
  __WB_MANIFEST: Array<{ url: string; revision: string | null }>;
};

// 1. Cache static build files (app shell, JS, CSS, images) via VitePWA/Workbox.
precacheAndRoute(self.__WB_MANIFEST);

// 2. Cache & revalidate Toapi API routes.
setupToapiWorker();
```

`precacheAndRoute(self.__WB_MANIFEST)` registers a `fetch` route for every
static file VitePWA fingerprinted at build time, so navigations and assets work
offline. `setupToapiWorker()` adds its own `fetch` listener that only claims
same-origin requests under `/api` (skipping `/api/__tapi`), so the two coexist:
static requests are served from the precache, API requests from the Toapi cache.

If your API lives somewhere other than `/api`, pass a matching `basePath` — and
make sure it isn't a prefix VitePWA also precaches:

```ts
setupToapiWorker({ basePath: "/data" });
```

:::note
`self.__WB_MANIFEST` **must** appear literally in the source — it's the
injection point VitePWA replaces with the real manifest. If you remove it the
build fails.
:::

## TypeScript

Add `"WebWorker"` to the `lib` array in `tsconfig.json` so TypeScript
recognizes `ServiceWorkerGlobalScope` and related globals:

```json
{
  "compilerOptions": {
    "lib": ["ESNext", "DOM", "WebWorker"]
  }
}
```

## Notes

- Match `setupToapiWorker`'s `basePath` to the `basePath` you pass to `toapi()`.
  It defaults to `/api`.
- Pass `maximumStaleAge` to `setupToapiWorker` to tune the grace period, in
  seconds, past a cache entry's `expiresAt` before it is deleted by the cleanup
  pass that runs on every worker startup. It defaults to 7 days.
- Need to interleave your own `fetch` logic with Toapi's? Call
  [`cleanup`](/tapi/worker/reference/cleanup/),
  [`handleToapiRequest`](/tapi/worker/reference/handle-toapi-request/), and
  [`listenForInvalidations`](/tapi/worker/reference/listen-for-invalidations/)
  directly instead of `setupToapiWorker`.

For the full service-worker API, see the
[`setupToapiWorker`](/tapi/worker/reference/setup-toapi-worker/) reference and the
[`@toapi/worker`](/tapi/worker/) package.
