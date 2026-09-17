---
title: "FilesystemCache"
description: "File-based SQLite Cache backend for @toapi/cache — persists across process restarts."
---

`FilesystemCache` is a [`Cache`](/tapi/cache/reference/cache/) implementation backed by a file-based SQLite database (via Node.js built-in `node:sqlite`). Data persists across process restarts.

```ts
import { FilesystemCache } from "@toapi/cache/filesystem-cache";
```

## Constructor

```ts
new FilesystemCache(filename: string)
```

- **Parameters**:
  - `filename`: `string` — Path to the SQLite database file. Created automatically if it does not exist. If the file already exists, it is reused.

## Usage

```ts
const cache = new FilesystemCache("./cache.db");

await cache.set({
  key: "book:1",
  data: { title: "Dune" },
  ttl: 300,
  tags: ["books", "book:1"],
});

const entry = await cache.get("book:1");
// { data: { title: "Dune" }, attachment: null }
```

## Garbage Collection

Same adaptive garbage collection as [InMemoryCache](/tapi/cache/reference/in-memory-cache/) — expired entries are cleaned up periodically with an interval that adjusts between 5 seconds and 5 minutes based on activity.

## When to Use

- Single-process deployments where you need cache persistence across restarts.
- Situations where an external service like Redis is not available or justified.
