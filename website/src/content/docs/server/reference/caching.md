---
title: "Caching Strategies"
description: "How Toapi combines server, service-worker, and client caches with a pub/sub system for fast, reactive tag-based revalidation."
---

Toapi uses multiple layers of caches combined with a pub/sub system to deliver great performance without compromising on reactivity.

The three cache layers are:

1. Server-side cache
2. Service worker cache
3. Client-side cache

## Revalidation table

This table shows how each cache layer handles a request for a resource depending on its revalidation state. There are three revalidation states:

- **No Config** — the route does not use the `cache` config.
- **Valid** — the route specifies `cache.ttl`, `cache.tags`, or both; none of the tags have been invalidated and the TTL has not expired since the last successful response.
- **Invalid** — the route specifies `cache.ttl`, `cache.tags`, or both, and one of the tags has been invalidated or the TTL has expired since the last successful response.

|          | No Config      | Valid           | Invalid       |
| -------- | -------------- | --------------- | ------------- |
| Server   | nothing stored | serve from cache | call handler  |
| Worker   | network-first  | cache-first     | network-first |
| Client   | serve from cache | serve from cache | refresh     |

Here "refresh" means the client _immediately_ requests a fresh version of the resource as soon as it goes invalid (a tag is revalidated, the TTL expires, or `revalidate()` is called on the resource). The fresh version is cached and distributed to all subscribers of the resource (usually the views displaying the data to the user).

## Server-side cache

Toapi does not cache data on the server by default. Enable caching by passing a `cache` to [`defineApi`](/tapi/server/reference/define-api/):

```ts
import { defineApi } from "@toapi/server";
import { InMemoryCache } from "@toapi/cache/in-memory-cache";

export const api = defineApi({ cache: new InMemoryCache() }).route(/* ... */);
```

For reference cache implementations, see the [`@toapi/cache`](/tapi/cache/) package.

Toapi includes a built-in pub/sub system to distribute invalidated tags to all connected clients via long-polling connections. By default, `defineApi` automatically creates a [`PubSub`](/tapi/server/reference/pub-sub/) instance — no extra setup is needed for single-host deployments. To use a custom implementation (e.g. Redis for multi-host), pass it explicitly:

```ts
import { defineApi, PubSub } from "@toapi/server";

export const api = defineApi({ cache: new PubSub() }).route(/* ... */);
```

When you serve your API with [`createRequestHandler`](/tapi/server/reference/create-request-handler/), the long-polling endpoint is mounted automatically at `<basePath>/__tapi/invalidations`. If your framework handles that path outside the Toapi handler, wire it up yourself with [`streamRevalidatedTags`](/tapi/server/reference/stream-revalidated-tags/):

```ts
import { streamRevalidatedTags } from "@toapi/server";
import { api } from "./api";

const GET = () => streamRevalidatedTags({ cache: api.cache });
```

:::caution
The default `PubSub` implementation only works on a single host. To run Toapi across multiple hosts, use a shared cache such as `RedisCache` from [`@toapi/cache`](/tapi/cache/).
:::

## Service worker cache

Toapi includes tools to set up a service worker that caches Toapi responses. Create a service worker, listen to the `fetch` event, decide whether the request should be handled by Toapi, and if so call `handleToapiRequest`:

```ts
import { handleToapiRequest } from "@toapi/worker";

self.addEventListener("fetch", (event) => {
  const url = new URL(event.request.url);
  if (url.host === process.env.BASE_URL && /\/api/.test(url.pathname)) {
    event.respondWith(handleToapiRequest(event.request));
  }
});
```

This stores responses in a cache and serves them until they expire or are revalidated through tag-based revalidation.

The service worker can also listen to the invalidation stream from the server using `listenForInvalidations`:

```ts
import { listenForInvalidations } from "@toapi/worker";

listenForInvalidations({ url: process.env.INVALIDATION_ROUTE });
```

Make sure `process.env.INVALIDATION_ROUTE` points to the route served by [`streamRevalidatedTags`](/tapi/server/reference/stream-revalidated-tags/) (mounted at `<basePath>/__tapi/invalidations` by default). When listening for invalidations, the worker marks all of its cached entries as expired and notifies its clients to reload them as soon as it connects to the invalidation stream.

To bound long-term cache growth, call `cleanup({ maximumStaleAge })` at the top level of the service worker, so it runs on every worker startup. It deletes cache entries whose `expiresAt` is older than `maximumStaleAge` seconds, removes cache entries that no longer have a meta record, and rebuilds the tags index from the surviving meta records:

```ts
import { cleanup } from "@toapi/worker";

cleanup({ maximumStaleAge: 60 * 60 * 24 * 7 }).catch(console.error);
```

See the [`@toapi/worker`](/tapi/worker/) package for the full service-worker toolkit.

## Client cache

Toapi has another layer of caching built into the client. This cache ensures that a request using the Toapi client returns the exact same response instance on subsequent calls:

```ts
client.books.get() === client.books.get();
```

This matters so frameworks such as React do not get stuck in infinite render loops when using the Toapi client.

The client cache is cleared on every page load — it does not persist data across page loads (that is the job of the service worker).

Data in the client cache automatically refreshes when its TTL expires plus a random jitter (the maximum jitter is configurable via the `maxOverdueTTL` option to `createFetchClient`, default 1000ms) to avoid thundering-herd problems.

This means a response with a TTL of 10 seconds:

```ts
TResponse.json({ /* ... */ }, { cache: { ttl: 10 } });
```

will actually be refreshed in the background every 10–11 seconds.

Responses for which the server has specified tags:

```ts
TResponse.json({ /* ... */ }, { cache: { tags: ["books"] } });
```

will automatically refresh whenever:

- A mutating request (`POST`, `PUT`, `PATCH`, `DELETE`) is made to a route that shares a tag with the response.
- The service worker receives an invalidation event for a shared tag.
- The `defaultTTL` configured on the server is exceeded.

Subscribe to refreshed data using the `subscribe` method:

```ts
client.books.get().subscribe((data) => {
  console.log(data);
});
```

The client only caches entries while they have active subscriptions. When the last subscription is removed, the entry is dropped from the cache after a short delay configurable via the `minTTL` option to `createFetchClient`, default 5000ms.

## Related

- [`TResponse`](/tapi/server/reference/t-response/) — attach `cache.tags` and `cache.ttl` to responses.
- [`streamRevalidatedTags`](/tapi/server/reference/stream-revalidated-tags/) — the invalidation stream endpoint.
- [`PubSub` and the `Cache` interface](/tapi/server/reference/pub-sub/) — the pub/sub contract.
- [`@toapi/cache`](/tapi/cache/) — reference server-side cache implementations.
- [`@toapi/worker`](/tapi/worker/) — service-worker caching and offline support.
