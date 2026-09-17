---
title: "streamRevalidatedTags"
description: "Expose Toapi's long-polling invalidation stream so clients and service workers receive revalidated cache tags in real time."
---

`streamRevalidatedTags` returns a streaming `Response` that pushes revalidated cache tags to a connected client over a long-lived connection. Service workers and clients subscribe to this stream to learn when tagged resources become stale, so they can refresh cached data immediately (see [Caching Strategies](/tapi/server/reference/caching/)).

:::note
When you serve your API with [`createRequestHandler`](/tapi/server/reference/create-request-handler/), this stream is already mounted at `<basePath>/__tapi/invalidations`. You only call `streamRevalidatedTags` directly when your framework routes that path outside the Toapi handler.
:::

## Usage

```ts
import { streamRevalidatedTags } from "@toapi/server";
import { api } from "./api";

export const GET = () => streamRevalidatedTags({ cache: api.cache });
```

## Signature

```ts
function streamRevalidatedTags(options: { cache: Cache }): Response;
```

## Parameters

### `options.cache`

**Type**: [`Cache`](/tapi/server/reference/pub-sub/)

The cache / pub-sub instance whose invalidations should be streamed. Pass the same instance that your API uses — `api.cache` — so subscribers receive the tags invalidated by your mutation handlers.

## How it works

- On connect, the handler assigns a random client ID and stores it in a session cookie (`__tapi-session`, `HttpOnly; SameSite=Strict`).
- It subscribes to the cache's pub/sub. Whenever tags are invalidated, they are written to the stream as a newline-terminated, space-separated list (e.g. `books book-42\n`).
- Invalidations that originated from this same client (matched by client ID) are skipped, so a client never receives an echo of its own mutations.
- A keepalive newline is sent every 10 seconds to hold the connection open.
- When the connection is cancelled, the keepalive interval is cleared and the subscription is removed.

The response is sent with `Content-Type: text/tapi-tags`.

## Related

- [`createRequestHandler`](/tapi/server/reference/create-request-handler/) — mounts this stream automatically.
- [`PubSub` and the `Cache` interface](/tapi/server/reference/pub-sub/) — the pub/sub contract behind the stream.
- [Caching Strategies](/tapi/server/reference/caching/) — how invalidations propagate to each cache layer.
