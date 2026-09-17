---
title: "setupToapiWorker"
description: "Wire up the whole Toapi service worker — cleanup, request handling, and the revalidation stream — in a single call."
---

`setupToapiWorker` is the one-call way to set up a Toapi service worker. It
registers the `fetch` listener, runs a cleanup pass, and opens the revalidation
stream, so you don't have to wire up [`cleanup`](/tapi/worker/reference/cleanup/),
[`handleToapiRequest`](/tapi/worker/reference/handle-toapi-request/), and
[`listenForInvalidations`](/tapi/worker/reference/listen-for-invalidations/)
yourself.

## Signature

```ts
function setupToapiWorker(options?: SetupToapiWorkerOptions): void;

interface SetupToapiWorkerOptions {
  /** Base path whose requests are cached. Default: "/api". */
  basePath?: string;
  /** Revalidation stream URL. Default: `${basePath}/__tapi/invalidations`. */
  invalidationsUrl?: string;
  /** Grace period in seconds past expiry before cleanup drops an entry. Default: 7 days. */
  maximumStaleAge?: number;
  /** Optional logger for worker errors, warnings, and progress. Default: the matching `console` methods. */
  logger?: Logger;
}
```

- **`basePath`** — same-origin requests whose pathname starts with this prefix
  are routed through [`handleToapiRequest`](/tapi/worker/reference/handle-toapi-request/).
  The `${basePath}/__tapi` control endpoints (the invalidation stream, the
  OpenAPI document) are always excluded. Defaults to `"/api"`.
- **`invalidationsUrl`** — the URL of the server's revalidation stream. Defaults
  to `${basePath}/__tapi/invalidations`.
- **`maximumStaleAge`** — how many seconds an entry may remain past its
  `expiresAt` before [`cleanup`](/tapi/worker/reference/cleanup/) drops it.
  Cleanup runs once each time the worker starts up. Defaults to 7 days.
- **`logger`** — an optional [`Logger`](/tapi/worker/reference/handle-toapi-request/#logger),
  passed on to [`handleToapiRequest`](/tapi/worker/reference/handle-toapi-request/)
  and [`listenForInvalidations`](/tapi/worker/reference/listen-for-invalidations/).
  Any method you leave out falls back to the matching `console` method.

## Usage

```ts
// service-worker.ts
import { setupToapiWorker } from "@toapi/worker";

declare const self: ServiceWorkerGlobalScope;

setupToapiWorker();
```

With a custom base path and stale window:

```ts
setupToapiWorker({
  basePath: "/data",
  maximumStaleAge: 60 * 60 * 24, // 1 day
});
```

## Composing with other `fetch` listeners

`setupToapiWorker` adds a `fetch` listener that only calls `respondWith` for
same-origin requests under `basePath`. Every other request falls through to any
other `fetch` listeners you (or another plugin) have registered. That is what
makes it compose with `vite-plugin-pwa`'s static-asset precaching: Toapi handles
the API routes, VitePWA/Workbox handles the app shell and static files. See the
[vite-plugin service-worker guide](/tapi/vite-plugin/guides/service-worker/) for
the full recipe.

## Equivalent manual setup

`setupToapiWorker()` is exactly equivalent to:

```ts
import {
  cleanup,
  handleToapiRequest,
  listenForInvalidations,
} from "@toapi/worker";

declare const self: ServiceWorkerGlobalScope;

self.addEventListener("fetch", (event) => {
  const url = new URL(event.request.url);
  if (url.origin !== self.location.origin) return;
  if (
    url.pathname.startsWith("/api") &&
    !url.pathname.startsWith("/api/__tapi")
  ) {
    event.respondWith(handleToapiRequest(event.request));
  }
});

listenForInvalidations({ url: "/api/__tapi/invalidations" }).catch(
  console.error,
);
cleanup({ maximumStaleAge: 60 * 60 * 24 * 7 }).catch(console.error);
```

Reach for the individual functions when you need to interleave Toapi with your
own `fetch` logic or cleanup scheduling; otherwise prefer `setupToapiWorker`.

## Related

- [`handleToapiRequest`](/tapi/worker/reference/handle-toapi-request/)
- [`listenForInvalidations`](/tapi/worker/reference/listen-for-invalidations/)
- [`cleanup`](/tapi/worker/reference/cleanup/)
- [Service worker setup guide](/tapi/worker/guides/service-worker/)
