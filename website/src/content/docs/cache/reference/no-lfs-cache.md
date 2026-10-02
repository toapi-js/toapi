---
title: "NoLfsCache"
description: "A Cache wrapper for @toapi/cache that skips caching entries with large attachments."
---

`NoLfsCache` wraps another [`Cache`](/toapi/cache/reference/cache/) and doesn't cache entries whose attachment is larger than a cutoff. Everything else is passed through to the wrapped cache unchanged.

```ts
import { NoLfsCache } from "@toapi/cache/no-lfs-cache";
```

## Constructor

```ts
new NoLfsCache(base: Cache, options?: NoLfsCacheOptions)
```

- **Parameters**:
  - `base`: The cache that all other operations are forwarded to.
  - `options`: Optional [`NoLfsCacheOptions`](#nolfscacheoptions).

### `NoLfsCacheOptions`

```ts
interface NoLfsCacheOptions {
  cutoffBytes?: number;
}
```

- `cutoffBytes`: Entries with an attachment larger than this are not cached. Defaults to `1_000_000` (1 MB).

## Usage

```ts
import { createClient } from "@redis/client";
import { NoLfsCache } from "@toapi/cache/no-lfs-cache";
import { RedisCache } from "@toapi/cache/redis-cache";

const redis = createClient();
await redis.connect();

const cache = new NoLfsCache(new RedisCache(redis), { cutoffBytes: 500_000 });
```

## How It Works

`set` silently returns without writing when the attachment exceeds `cutoffBytes`. `get`, `delete` / `invalidate` and `subscribe` are forwarded to the base cache.

## When to Use

- Keeping large binary responses out of a memory-bound cache such as Redis.
- When large attachments are rare or cheap to regenerate, so caching them isn't worth it.

To cache large attachments in S3 instead, use [`S3LfsCache`](/toapi/cache/reference/s3-lfs-cache/).
