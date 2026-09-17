---
title: "Service Worker Setup"
description: "Learn how to set up a Toapi service worker for caching and revalidation."
---

This guide explains how to create a Toapi service worker, build it, and
register it on your page.

## 1. Create the Service Worker

Create a `service-worker.ts` file in your project (e.g. at the root or in
`src/`) and call [`setupToapiWorker`](/tapi/worker/reference/setup-toapi-worker/):

```ts
// service-worker.ts
import { setupToapiWorker } from "@toapi/worker";

declare const self: ServiceWorkerGlobalScope;

setupToapiWorker();
```

That single call registers the `fetch` listener, runs a cleanup pass, and
opens the revalidation stream. By default it caches same-origin requests under
`/api` (excluding the `/api/__tapi` control endpoints) and listens for
invalidations on `/api/__tapi/invalidations`. Pass options to adjust the
base path, stream URL, stale window, or logger:

```ts
setupToapiWorker({
  basePath: "/data",
  maximumStaleAge: 60 * 60 * 24, // 1 day
});
```

When the service worker connects to the invalidation stream, it
automatically marks every cached entry as expired so the next access
revalidates it. If the stream drops it reconnects with a growing backoff,
expiring everything again on each reconnect.

On every worker startup it also runs a cleanup pass that bounds long-term
cache growth: entries whose `expiresAt` is older than `maximumStaleAge`
seconds are deleted, cache entries that no longer have a meta record are
removed, and the tags index is rebuilt from the surviving meta records.

### Wiring it up by hand

If you need to interleave Toapi with your own `fetch` logic, call the
underlying functions directly instead of `setupToapiWorker`:

```ts
// service-worker.ts
import {
  handleToapiRequest,
  listenForInvalidations,
  cleanup,
} from "@toapi/worker";

declare const self: ServiceWorkerGlobalScope;

self.addEventListener("fetch", (event) => {
  const url = new URL(event.request.url);
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

// Drop cache entries that have been expired longer than 7 days,
// delete orphans, and rebuild the tags index from meta.
cleanup({ maximumStaleAge: 60 * 60 * 24 * 7 }).catch(console.error);
```

Adjust the pathname checks to match your API base path and revalidation
endpoint.

## 2. Add TypeScript Types

Service workers use APIs from the `WebWorker` type lib rather than the
standard `DOM` lib. Add `"WebWorker"` to your `tsconfig.json` so
TypeScript recognizes `ServiceWorkerGlobalScope`, `FetchEvent`, and
related types:

```json
{
  "compilerOptions": {
    "lib": ["ESNext", "DOM", "DOM.Iterable", "WebWorker"]
  }
}
```

If your project already has a `lib` array, just append `"WebWorker"` to
it.

## 3. Build into `public/sw.js`

The service worker file needs to be bundled into a single JavaScript file
and placed where your web server can serve it from the root path. Use
your bundler of choice to build `service-worker.ts` into `public/sw.js`.

For example with esbuild:

```bash
esbuild service-worker.ts --bundle --outfile=public/sw.js
```

Or add it as a build script in your `package.json`:

```json
{
  "scripts": {
    "build:sw": "esbuild service-worker.ts --bundle --outfile=public/sw.js"
  }
}
```

### Using Vite + `@toapi/vite-plugin`

If your project uses [`@toapi/vite-plugin`](/tapi/vite-plugin/),
the cleanest way to build the service worker is with
[`vite-plugin-pwa`](https://vite-pwa-org.netlify.app/) in
`injectManifest` mode, alongside `tapi()`. See the
[vite-plugin docs](/tapi/vite-plugin/) for the full recipe.

## 4. Register the Service Worker

Add a script to your HTML page that registers the service worker. This
should run on every page that uses Toapi:

```html
<script>
  if ("serviceWorker" in navigator) {
    navigator.serviceWorker.register("/sw.js");
  }
</script>
```

Place this at the end of your `<body>` or in a `<script>` tag in
`<head>` with `defer`. A full example:

```html
<!doctype html>
<html>
  <head>
    <title>My App</title>
  </head>
  <body>
    <div id="app"></div>
    <script type="module" src="/main.js"></script>
    <script>
      if ("serviceWorker" in navigator) {
        navigator.serviceWorker.register("/sw.js");
      }
    </script>
  </body>
</html>
```

Once registered, the service worker will intercept API requests on
subsequent navigations, serve cached responses when available, and
automatically refetch stale data when tags are invalidated through the
revalidation stream.

## Related

- [`setupToapiWorker`](/tapi/worker/reference/setup-toapi-worker/)
- [`handleToapiRequest`](/tapi/worker/reference/handle-toapi-request/)
- [`listenForInvalidations`](/tapi/worker/reference/listen-for-invalidations/)
- [`cleanup`](/tapi/worker/reference/cleanup/)
