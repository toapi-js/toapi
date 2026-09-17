---
title: "@toapi/worker"
description: "Service-worker request handling for Toapi: offline-capable caching and remote tag revalidation."
---

`@toapi/worker` is the service-worker half of the Toapi caching system. It runs
inside a browser [Service Worker](https://developer.mozilla.org/en-US/docs/Web/API/Service_Worker_API)
and intercepts requests to your Toapi endpoints, transparently caching responses
in [Cache Storage](https://developer.mozilla.org/en-US/docs/Web/API/Cache) and
tracking their cache tags and expiry in IndexedDB.

Together with the server's revalidation stream this gives you:

- **Instant reads** — cached responses are served without hitting the network.
- **Tag-based revalidation** — when the server invalidates a tag, the worker
  marks every affected cache entry as stale so it is refetched on next access.
- **Offline resilience** — if the network is unavailable, an expired-but-present
  cache entry is served rather than failing.

## Installation

```bash
npm install @toapi/worker
```

The package targets the `WebWorker` type lib rather than the `DOM` lib. See the
[service-worker guide](/tapi/worker/guides/service-worker/) for the `tsconfig.json`
setup and the full build/register recipe.

## Public API

| Export | Kind | Purpose |
| --- | --- | --- |
| [`setupToapiWorker`](/tapi/worker/reference/setup-toapi-worker/) | function | Set up the whole worker in one call: registers the listeners and opens the stream. |
| [`handleToapiRequest`](/tapi/worker/reference/handle-toapi-request/) | function | Handle a single `fetch` event: serve from cache, network, or invalidate on mutation. |
| [`listenForInvalidations`](/tapi/worker/reference/listen-for-invalidations/) | function | Open the server's revalidation stream and apply remote tag invalidations. |
| [`cleanup`](/tapi/worker/reference/cleanup/) | function | Reconcile the cache and metadata stores; run once on worker startup. |
| `SetupToapiWorkerOptions` | type | Options for [`setupToapiWorker`](/tapi/worker/reference/setup-toapi-worker/). |
| `CleanupOptions` | type | Options for [`cleanup`](/tapi/worker/reference/cleanup/). |
| `Logger` | type | Re-exported from `@toapi/common`; the optional logger accepted by `handleToapiRequest`, `listenForInvalidations`, and `setupToapiWorker`. |

## Minimal service worker

The whole worker is a single call:

```ts
// service-worker.ts
import { setupToapiWorker } from "@toapi/worker";

declare const self: ServiceWorkerGlobalScope;

setupToapiWorker();
```

By default this caches same-origin requests under `/api` (excluding the
`/api/__tapi` control endpoints), listens for invalidations on
`/api/__tapi/invalidations`, and runs a cleanup pass on every worker startup. Pass options to change the base path, stream URL,
stale window, or logger:

```ts
setupToapiWorker({
  basePath: "/data",
  maximumStaleAge: 60 * 60 * 24, // 1 day
});
```

:::note
`setupToapiWorker`'s `fetch` listener only responds to same-origin requests
under `basePath`, so everything else falls through to any other `fetch`
listeners — that's how it composes with `vite-plugin-pwa`'s static-asset
precaching. See the
[vite-plugin service-worker guide](/tapi/vite-plugin/guides/service-worker/).
:::

If you need to interleave Toapi with your own `fetch` logic, you can
wire up [`cleanup`](/tapi/worker/reference/cleanup/),
[`handleToapiRequest`](/tapi/worker/reference/handle-toapi-request/), and
[`listenForInvalidations`](/tapi/worker/reference/listen-for-invalidations/) by
hand instead — see the
[service-worker guide](/tapi/worker/guides/service-worker/).

## Related

- [Service worker setup guide](/tapi/worker/guides/service-worker/)
- [`@toapi/cache`](/tapi/cache/) — the server-side tag-based cache that
  produces the tags and expiry headers this worker reads.
