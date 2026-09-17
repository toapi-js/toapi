---
title: "InMemoryCache"
description: "SQLite in-memory Cache backend for @toapi/cache — fast, zero I/O, non-persistent."
---

`InMemoryCache` is a [`Cache`](/tapi/cache/reference/cache/) implementation backed by an in-memory SQLite database (via Node.js built-in `node:sqlite`). Data does not survive process restarts.

```ts
import { InMemoryCache } from "@toapi/cache/in-memory-cache";
```

## Constructor

```ts
new InMemoryCache()
```

No arguments required. Creates an in-memory SQLite database and starts automatic garbage collection for expired entries.

## Usage

```ts
const cache = new InMemoryCache();

await cache.set({
  key: "book:1",
  data: { title: "Dune" },
  ttl: 300,
  tags: ["books", "book:1"],
});

const entry = await cache.get("book:1");
// { data: { title: "Dune" }, attachment: null }

// Invalidate by tag
await cache.delete(["book:1"]);
```

## Garbage Collection

Expired entries are cleaned up automatically on a periodic timer. The interval adapts based on how many expired entries are found:

- Starts at 5 seconds.
- Decreases (down to a 5-second minimum) when many expired entries are found.
- Increases (up to a 5-minute maximum) when few expired entries are found.

## When to Use

- Development and testing.
- Single-process production deployments where persistence is not needed.
- Scenarios where you want the fastest possible reads/writes with no I/O overhead.
