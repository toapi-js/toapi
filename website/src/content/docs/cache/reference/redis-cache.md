---
title: "RedisCache"
description: "Redis-backed distributed Cache for @toapi/cache with pub/sub invalidation across hosts."
---

`RedisCache` is a [`Cache`](/tapi/cache/reference/cache/) implementation backed by Redis. It supports distributed cache invalidation via Redis pub/sub, making it suitable for multi-host deployments.

```ts
import { RedisCache } from "@toapi/cache/redis-cache";
```

:::note
Requires the `@redis/client` package as a peer dependency.
:::

## Constructor

```ts
new RedisCache(redis: RedisClientType)
```

- **Parameters**:
  - `redis`: A connected `RedisClientType` instance from `@redis/client`.

## Usage

```ts
import { createClient } from "@redis/client";
import { RedisCache } from "@toapi/cache/redis-cache";

const redis = createClient();
await redis.connect();

const cache = new RedisCache(redis);

await cache.set({
  key: "book:1",
  data: { title: "Dune" },
  ttl: 300,
  tags: ["books", "book:1"],
});

const entry = await cache.get("book:1");
// { data: { title: "Dune" }, attachment: null }
```

## How It Works

### Storage

Entries are stored as separate Redis keys:

- `data:{key}` — JSON-serialized data, with Redis TTL expiration.
- `attachment:{key}` — Binary data, with Redis TTL expiration.
- `tag:{tag}` — A Redis set containing all keys associated with that tag.

### Invalidation

When `delete(tags)` is called:

1. All keys associated with the given tags are looked up via the `tag:{tag}` sets.
2. The corresponding `data:*`, `attachment:*`, and `tag:*` keys are deleted.
3. An invalidation message is published to the `invalidate` Redis channel so other connected instances are notified.

### Subscriptions

`subscribe` uses a dedicated Redis connection (created via `redis.duplicate()`) to listen on the `invalidate` pub/sub channel. The subscription is set up lazily on the first `subscribe` call and torn down when the last subscriber unsubscribes.

```ts
const unsubscribe = cache.subscribe((tags, meta) => {
  console.log("Invalidated:", tags);
});

// Later:
unsubscribe();
```

## When to Use

- Multi-host deployments where cache invalidation needs to propagate across processes.
- When you need the Toapi [Caching](/tapi/server/reference/caching/) pub/sub system to work across multiple servers.
