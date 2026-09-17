---
title: "@toapi/cache"
description: "A cache abstraction with tag-based invalidation and multiple backends (in-memory, filesystem, Redis, Postgres)."
---

`@toapi/cache` is a cache library built around **tag-based invalidation**. Instead of expiring individual keys, you invalidate entire groups of cache entries by their tags — a pattern that maps naturally to how data actually changes in real applications.

It is the reference cache implementation for the Toapi stack and is usually used to add caching to [`@toapi/server`](/tapi/server/).

## Installation

```bash
npm install @toapi/cache
```

The Redis and Postgres backends need an extra peer dependency (`@redis/client` or `pg` respectively). Both are optional — install only the one you use.

## Backends

All backends implement the same [`Cache`](/tapi/cache/reference/cache/) interface, so you can swap between them without touching application code.

- **[`InMemoryCache`](/tapi/cache/reference/in-memory-cache/)** — SQLite in-memory database. Fast, zero I/O — ideal for development, testing, and single-process deployments.
- **[`FilesystemCache`](/tapi/cache/reference/filesystem-cache/)** — SQLite file-based database. Survives process restarts, suitable for single-host production setups.
- **[`RedisCache`](/tapi/cache/reference/redis-cache/)** — Redis-backed distributed cache with pub/sub support. The right choice for multi-host deployments.
- **[`PostgresCache`](/tapi/cache/reference/postgres-cache/)** — PostgreSQL-backed distributed cache using `LISTEN`/`NOTIFY`. For multi-host deployments that already run Postgres and would rather not add Redis.

## Quick Start

```ts
import { InMemoryCache } from "@toapi/cache/in-memory-cache";

const cache = new InMemoryCache();

// Store with tags
await cache.set({
  key: "user:1",
  data: { name: "Alice" },
  ttl: 3600,
  tags: ["users", "user:1"],
});

// Retrieve
const entry = await cache.get("user:1");
// { data: { name: "Alice" }, attachment: null }

// Invalidate all entries tagged "users"
await cache.delete(["users"]);

await cache.get("user:1"); // null
```

## Usage with Toapi

Pass a cache instance to `createRequestHandler` to enable server-side caching:

```ts
import { createRequestHandler } from "@toapi/server";
import { InMemoryCache } from "@toapi/cache/in-memory-cache";

const cache = new InMemoryCache();
const handleRequest = createRequestHandler(api, { cache });
```

See [Caching](/tapi/server/reference/caching/) for the full picture.

## Reference

- [Cache Interface](/tapi/cache/reference/cache/) — the shared interface all backends implement.
- [InMemoryCache](/tapi/cache/reference/in-memory-cache/) — SQLite in-memory backend.
- [FilesystemCache](/tapi/cache/reference/filesystem-cache/) — SQLite file-based backend.
- [RedisCache](/tapi/cache/reference/redis-cache/) — Redis-backed distributed backend.
- [PostgresCache](/tapi/cache/reference/postgres-cache/) — PostgreSQL-backed distributed backend.
