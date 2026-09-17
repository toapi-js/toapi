---
title: "PostgresCache"
description: "PostgreSQL-backed distributed Cache for @toapi/cache using LISTEN/NOTIFY invalidation."
---

`PostgresCache` is a [`Cache`](/tapi/cache/reference/cache/) implementation backed by PostgreSQL. It supports distributed cache invalidation via Postgres `LISTEN`/`NOTIFY`, making it suitable for multi-host deployments that already run Postgres and would rather not add Redis.

```ts
import { PostgresCache } from "@toapi/cache/postgres-cache";
```

:::note
Requires the `pg` (`node-postgres`) package as a peer dependency.
:::

## Constructor

```ts
new PostgresCache(pool: Pool, options?: PostgresCacheOptions)
```

- **Parameters**:
  - `pool`: A `pg` `Pool`. The cache checks out a dedicated connection from this pool for `LISTEN` while it has active subscribers, and short-lived connections for reads and writes. The pool's lifecycle is owned by the caller.
  - `options`: Optional [`PostgresCacheOptions`](#postgrescacheoptions).

### `PostgresCacheOptions`

```ts
interface PostgresCacheOptions {
  createSchema?: boolean;
  schema?: string;
}
```

- `createSchema`: When `true` (the default) the required tables and indexes are created lazily on the first operation via `CREATE TABLE IF NOT EXISTS`. Set to `false` if you manage the schema yourself (e.g. via migrations).
- `schema`: The Postgres schema that holds the cache tables. Defaults to whatever the connection's `search_path` resolves to (usually `public`). When set, the tables are fully qualified as `<schema>.cache_entries` / `<schema>.cache_tags`, the schema is created if missing (unless `createSchema` is `false`), and a schema-specific invalidation channel is used so caches on different schemas never cross-notify.

## Usage

```ts
import { Pool } from "pg";
import { PostgresCache } from "@toapi/cache/postgres-cache";

const pool = new Pool({ connectionString: process.env.POSTGRES_URL });

const cache = new PostgresCache(pool);

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

Entries are stored across two tables (qualified with the configured `schema` when set):

- `cache_entries` — one row per key holding the JSON `data` (as `JSONB`), the binary `attachment` (as `BYTEA`), and the `added_at` / `expires_at` timestamps (epoch milliseconds).
- `cache_tags` — a `(key, tag)` join table. Rows reference `cache_entries(key)` with `ON DELETE CASCADE`, so a deleted entry drops its tag associations automatically.

`get` filters on `expires_at` so entries past their TTL are never returned, even before the garbage collector removes them.

### Invalidation

When `delete(tags)` is called:

1. All entries associated with the given tags are deleted via the `cache_tags` join table (their tag rows cascade away).
2. An invalidation message is broadcast with `pg_notify` on the `tag_based_cache_invalidate` channel so other connected instances are notified.

### Subscriptions

`subscribe` checks out a dedicated pooled connection and issues `LISTEN` on the `tag_based_cache_invalidate` channel. The listener is set up lazily on the first `subscribe` call and the connection is released back to the pool when the last subscriber unsubscribes.

```ts
const unsubscribe = cache.subscribe((tags, meta) => {
  console.log("Invalidated:", tags);
});

// Later:
unsubscribe();
```

### Garbage Collection

A background timer periodically deletes expired entries. The interval adapts between 5 seconds and 5 minutes based on how many rows each sweep removes, matching the [`InMemoryCache`](/tapi/cache/reference/in-memory-cache/) and [`FilesystemCache`](/tapi/cache/reference/filesystem-cache/) behaviour.

### Shutdown

Call `close()` to stop the garbage collector and release the dedicated listener connection back to the pool before shutting down. The underlying `Pool` is left untouched — end it yourself when appropriate.

```ts
await cache.close();
await pool.end();
```

## When to Use

- Multi-host deployments where cache invalidation needs to propagate across processes and you already run Postgres.
- When you need the Toapi [Caching](/tapi/server/reference/caching/) pub/sub system to work across multiple servers without introducing Redis.
