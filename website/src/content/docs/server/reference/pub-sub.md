---
title: "PubSub and the Cache interface"
description: "The in-process pub/sub used for Toapi tag revalidation, and the Cache contract that server-side cache implementations satisfy."
---

`PubSub` is the default cache/pub-sub instance used by [`defineApi`](/tapi/server/reference/define-api/). It distributes tag invalidations to connected clients but stores no response data of its own. It implements the `Cache` interface, which every server-side cache — including the reference implementations in [`@toapi/cache`](/tapi/cache/) — satisfies.

## PubSub

When you call `defineApi()` without a `cache`, an in-process `PubSub` is created for you. It is enough for single-host deployments that need tag-based revalidation but no server-side response caching.

```ts
import { defineApi, PubSub } from "@toapi/server";

// Equivalent to defineApi()
export const api = defineApi({ cache: new PubSub() }).route(/* ... */);
```

Behaviour:

- `get()` always returns `null` — nothing is stored.
- `set()` is a no-op.
- `delete(tags, meta)` notifies every subscriber with the invalidated tags and optional metadata (such as the originating `clientId`).
- `subscribe(callback)` registers a subscriber and returns an unsubscribe function.

:::caution
`PubSub` only coordinates within a single process. To share invalidations across multiple hosts, use a shared implementation such as `RedisCache` from [`@toapi/cache`](/tapi/cache/).
:::

## The Cache interface

Pass any object implementing the `Cache` interface to [`defineApi`](/tapi/server/reference/define-api/) to enable server-side response caching. The [request handler](/tapi/server/reference/create-request-handler/) reads and writes through it, and [`streamRevalidatedTags`](/tapi/server/reference/stream-revalidated-tags/) subscribes to it.

```ts
type Json =
  | string
  | number
  | boolean
  | null
  | Json[]
  | { [key: string]: Json };

interface CacheEntry {
  data?: Json | null;
  attachment?: Uint8Array | null;
}

type Subscription = (tags: string[], meta?: Json) => void;

interface Cache {
  get(key: string): Promise<CacheEntry | null>;
  set(
    input: CacheEntry & { key: string; ttl: number; tags: string[] },
  ): Promise<void>;
  delete(tags: string[], meta?: Json): Promise<void>;
  subscribe(callback: Subscription): () => void;
}
```

| Member | Description |
| --- | --- |
| `get(key)` | Look up a cached entry by key (the request URL). Returns `null` on a miss. The stored `data` holds the response headers and `attachment` holds the raw body bytes. |
| `set(input)` | Store a fresh response under `key` with a `ttl` (seconds) and a set of `tags`. |
| `delete(tags, meta)` | Invalidate every entry carrying any of `tags` and notify subscribers. `meta` may carry a `clientId` so a client can ignore its own invalidations. |
| `subscribe(callback)` | Register a subscriber invoked on every `delete`. Returns an unsubscribe function. |

## Related

- [`defineApi`](/tapi/server/reference/define-api/) — accepts a `cache` implementing this interface.
- [`streamRevalidatedTags`](/tapi/server/reference/stream-revalidated-tags/) — subscribes to the cache to stream invalidations.
- [`@toapi/cache`](/tapi/cache/) — ready-to-use `Cache` implementations (in-memory, Redis, …).
- [Caching Strategies](/tapi/server/reference/caching/) — the bigger picture across all cache layers.
