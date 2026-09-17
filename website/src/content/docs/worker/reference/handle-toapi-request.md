---
title: "handleToapiRequest"
description: "Handle a single service-worker fetch event: serve from cache, revalidate from network, or invalidate on mutation."
---

`handleToapiRequest` is the core request handler for the Toapi service worker.
Pass it the `Request` from a `fetch` event and it decides whether to serve from
Cache Storage, fetch from the network, or run a mutation and invalidate the
affected tags.

:::note[Renamed]
This function was previously called `handleTapiRequest`. That name is still
exported as a deprecated alias for the same function and will be removed in a
future major version — prefer `handleToapiRequest`. Most setups don't call it
directly at all; use [`setupToapiWorker`](/tapi/worker/reference/setup-toapi-worker/).
:::

## Signature

```ts
function handleToapiRequest(
  req: Request,
  options?: { logger?: Logger },
): Promise<Response>;
```

- **`req`** — the request to handle, typically `event.request` from a
  service-worker `fetch` event.
- **`options.logger`** — an optional [`Logger`](#logger) whose `error` method is
  called when a network refetch of an expired entry fails. Defaults to
  `console.error`.

Returns a `Promise<Response>` suitable for passing to `event.respondWith(...)`.

## Usage

```ts
import { handleToapiRequest } from "@toapi/worker";

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
```

Only route requests you actually want cached/handled through this function.
Exclude the Toapi control endpoints (such as the `/api/__tapi/invalidations`
stream) from the handler.

## Behavior

The handler branches on whether the request is a **mutation** — that is, whether
its method is `POST`, `PUT`, `PATCH`, or `DELETE`.

### Mutations

For a mutation, the request is forwarded to the network. When the response comes
back, any cache tags it carries (via the tags response header) are invalidated,
marking the corresponding cached entries as stale. The network response is
returned unchanged. If the response carries no tags, invalidation is skipped.

### Reads (safe methods)

For a non-mutation request (`GET`, `HEAD`, etc.) the handler resolves in this
order:

1. **No cached entry** — the request is served from the network. The response is
   stored in the cache only if it is `ok` and carries a tags or expires-at
   header; otherwise any stale entry for that URL is removed.
2. **Cached entry whose metadata says it is still fresh** — the cached response
   is returned directly, without touching the network. An entry counts as fresh
   when its metadata has a future `expiresAt`, or no `expiresAt` at all (the
   response carried tags but no expiry, so only an invalidation can make it
   stale).
3. **Cached entry that has expired, or has no metadata at all** — the handler
   tries to refetch from the network. If the refetch fails (for example, the
   device is offline), the error is passed to `options.logger.error` and the
   stale cached response is served as a fallback.

:::tip
Step 3 is what makes the worker offline-tolerant: an expired entry is always
preferable to a network failure, so the user still sees data.
:::

:::note[Entries without metadata]
A cache entry whose metadata record is missing — for instance because the meta
store was cleared, or a write was interrupted — is treated as expired rather
than as fresh. Without metadata there is no expiry to check and no tag that
could ever invalidate it, so serving it from cache would make it immortal. It is
still kept as an offline fallback.
:::

## `Logger`

```ts
interface Logger {
  error?: (error: unknown) => void | Promise<void>;
  warn?: (message: string) => void | Promise<void>;
  info?: (message: string) => void | Promise<void>;
}
```

`Logger` is re-exported from `@toapi/common`. Every method is optional and each
one falls back independently to the matching `console` method, so you can
override just the ones you care about. `handleToapiRequest` only uses `error`;
`warn` and `info` are used by
[`listenForInvalidations`](/tapi/worker/reference/listen-for-invalidations/) for
connection retries and stream progress.

Provide one to route worker errors into your own reporting rather than
`console.error`:

```ts
import { handleToapiRequest, type Logger } from "@toapi/worker";

const logger: Logger = {
  error: (err) => reportToSentry(err),
};

handleToapiRequest(event.request, { logger });
```

## Related

- [`listenForInvalidations`](/tapi/worker/reference/listen-for-invalidations/)
- [`cleanup`](/tapi/worker/reference/cleanup/)
- [Service worker setup guide](/tapi/worker/guides/service-worker/)
