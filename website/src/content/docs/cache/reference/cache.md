---
title: "Cache"
description: "The Cache interface — the common contract shared by every @toapi/cache backend."
---

The `Cache` interface is the common contract shared by all cache implementations. You can swap between [InMemoryCache](/tapi/cache/reference/in-memory-cache/), [FilesystemCache](/tapi/cache/reference/filesystem-cache/), [RedisCache](/tapi/cache/reference/redis-cache/), and [PostgresCache](/tapi/cache/reference/postgres-cache/) without changing any calling code.

```ts
import type { Cache } from "@toapi/cache";
```

## Interface

```ts
interface Cache {
  get(key: string): Promise<CacheEntry | null>;
  set(input: CacheEntry & { key: string; ttl: number; tags: string[] }): Promise<void>;
  delete(tags: string[], meta?: { clientId?: string }): Promise<void>;
  subscribe(callback: Subscription): () => void;
}
```

## Methods

### `get(key)`

Retrieves a cache entry by its key. Returns `null` if the key does not exist or has expired.

- **Parameters**:
  - `key`: `string` — The cache key.
- **Returns**: `Promise<CacheEntry | null>`

### `set(input)`

Stores a cache entry.

- **Parameters**:
  - `input.key`: `string` — The cache key.
  - `input.data`: `Json | null` — JSON-serializable data to store.
  - `input.attachment`: `Uint8Array | null` — Optional binary data to store alongside the JSON data.
  - `input.ttl`: `number` — Time-to-live in seconds.
  - `input.tags`: `string[]` — Tags for later invalidation.
- **Returns**: `Promise<void>`

### `delete(tags, meta?)`

Deletes all cache entries that have any of the given tags and notifies subscribers.

- **Parameters**:
  - `tags`: `string[]` — Tags to invalidate.
  - `meta`: `Json` — Optional metadata forwarded to subscribers.
- **Returns**: `Promise<void>`

### `subscribe(callback)`

Registers a callback that is invoked whenever `delete` is called. Returns an unsubscribe function.

- **Parameters**:
  - `callback`: `(tags: string[], meta?: Json) => void`
- **Returns**: `() => void` — Call this to unsubscribe.

## Related Types

### `CacheEntry`

```ts
interface CacheEntry {
  data?: Json | null;
  attachment?: Uint8Array | null;
}
```

Each entry can hold JSON `data`, a binary `attachment`, or both.

### `Json`

```ts
type Json = string | number | boolean | null | Json[] | { [key: string]: Json };
```

The type of any JSON-serializable value accepted as `data`.

### `Subscription`

```ts
interface Subscription {
  (tags: string[], meta?: Json): void;
}
```

The callback signature passed to [`subscribe`](#subscribecallback). It receives the invalidated `tags` and any `meta` forwarded by `delete`.
