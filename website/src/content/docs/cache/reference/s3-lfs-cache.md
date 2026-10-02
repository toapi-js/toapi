---
title: "S3LfsCache"
description: "A Cache wrapper for @toapi/cache that offloads large attachments to S3 while keeping entries and tags in another backend."
---

`S3LfsCache` wraps another [`Cache`](/toapi/cache/reference/cache/) and moves large attachments into an S3 bucket ("large file storage"). Data, small attachments, tags and invalidation stay in the wrapped cache, so Redis or Postgres don't have to hold multi-megabyte blobs.

```ts
import { S3LfsCache } from "@toapi/cache/s3-lfs-cache";
```

:::note
Requires the `@aws-sdk/client-s3` package as a peer dependency.
:::

## Constructor

```ts
new S3LfsCache(base: Cache, client: S3Client, options?: S3LfsCacheOptions)
```

- **Parameters**:
  - `base`: The cache that stores entries, tags and small attachments, and handles invalidation and subscriptions. Any backend works, e.g. [`RedisCache`](/toapi/cache/reference/redis-cache/) or [`PostgresCache`](/toapi/cache/reference/postgres-cache/).
  - `client`: An `S3Client` from `@aws-sdk/client-s3`. Its lifecycle is owned by the caller.
  - `options`: Optional [`S3LfsCacheOptions`](#s3lfscacheoptions).

### `S3LfsCacheOptions`

```ts
interface S3LfsCacheOptions {
  bucket?: string;
  cutoffBytes?: number;
}
```

- `bucket`: The bucket that large attachments are written to. Set this: S3 requests fail without a bucket.
- `cutoffBytes`: Attachments larger than this are stored in S3. Attachments of this size or smaller stay in the base cache. Defaults to `1_000_000` (1 MB).

## Usage

```ts
import { S3Client } from "@aws-sdk/client-s3";
import { createClient } from "@redis/client";
import { RedisCache } from "@toapi/cache/redis-cache";
import { S3LfsCache } from "@toapi/cache/s3-lfs-cache";

const redis = createClient();
await redis.connect();

const cache = new S3LfsCache(new RedisCache(redis), new S3Client(), {
  bucket: "my-app-cache",
});

await cache.set({
  key: "report:2026",
  data: { name: "report.pdf" },
  attachment: largePdfBytes,
  ttl: 3600,
  tags: ["reports"],
});

const entry = await cache.get("report:2026");
// { data: { name: "report.pdf" }, attachment: Uint8Array(...) }
```

For S3-compatible servers such as MinIO or [local-s3](https://github.com/shyim/local-s3), pass `endpoint` and `forcePathStyle: true` to the `S3Client`.

## How It Works

### Storage

Every entry written through `S3LfsCache` is stored in the base cache with its data wrapped in an envelope that records where the attachment lives:

- If the attachment is larger than `cutoffBytes`, it is uploaded to S3 under the cache key, and the base entry stores a reference to that object (including the `VersionId` if the bucket has versioning enabled). The base entry's own attachment is `null`.
- Otherwise the attachment is stored in the base cache as usual.

`get` reads the base entry and, if it references an S3 object, downloads the object and returns it as the attachment. Entries in the base cache that were not written by `S3LfsCache` are treated as a miss.

### Invalidation

`delete` / `invalidate` and `subscribe` are forwarded to the base cache. Invalidating an entry removes its reference, so the S3 object becomes unreachable, but **the object itself is not deleted**. Configure a lifecycle rule on the bucket that expires objects after your longest TTL to clean them up.

## When to Use

- Responses with large binary attachments (files, images, generated documents) that would put too much load on the base cache.
- Deployments that already use S3 or an S3-compatible store.

To skip caching large attachments entirely instead, use [`NoLfsCache`](/toapi/cache/reference/no-lfs-cache/).
