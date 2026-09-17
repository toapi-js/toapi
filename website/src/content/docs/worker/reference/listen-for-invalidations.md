---
title: "listenForInvalidations"
description: "Open the Toapi revalidation stream and apply remote tag invalidations to the service-worker cache."
---

`listenForInvalidations` connects the service worker to the server's revalidation
stream. It keeps a long-lived connection open and, whenever the server pushes a
set of invalidated tags, marks the matching cached entries as stale so they are
refetched on next access. It also notifies open page clients so hooks like
`@toapi/react` can react to the invalidation.

## Signature

```ts
function listenForInvalidations(options: {
  url: string;
  timeout?: number;
  logger?: Logger;
}): Promise<void>;
```

- **`url`** — the URL of the server's revalidation stream endpoint (for example
  `/api/__tapi/invalidations`).
- **`timeout`** — how long in milliseconds to wait before reconnecting after the
  stream fails. Each reconnect multiplies it by 1.5, so a backend that stays
  down is polled less and less often. Defaults to `5000`.
- **`logger`** — an optional [`Logger`](/tapi/worker/reference/handle-toapi-request/#logger).
  Its `error`, `warn`, and `info` methods report a fatal stream failure, failed
  connection attempts, and connection/invalidation progress respectively. Each
  method that you leave out falls back to the matching `console` method.

The returned promise resolves when the stream ends or the function gives up; you
normally call it once at the top level of your service worker and do not await
it.

## Usage

```ts
import { listenForInvalidations } from "@toapi/worker";

listenForInvalidations({ url: "/api/__tapi/invalidations" });
```

Call it at the top level of the worker, not inside an event listener — it should
run as soon as the worker starts and stay connected for the worker's lifetime.

## Behavior

### Connecting (with retry)

The function opens the stream with `fetch(url)`. If the fetch throws, it retries
with exponential backoff (starting at 500 ms and doubling each attempt) for up to
1000 attempts before giving up and logging an error.

### Validating the stream

Once a response is received, it must be `ok`, carry the Toapi tags content type,
and have a readable body. If any of these checks fail, the worker treats the
endpoint as unavailable, **deletes its cache and unregisters itself** — this
prevents a service worker from serving stale data against a backend that no
longer speaks the Toapi protocol.

### On connect: expire everything

Immediately after a successful connection, the worker marks **all** existing
cache entries as expired and posts the affected tags to every open client. This
guarantees that after any reconnect (e.g. the page was reopened after being
offline) nothing stale is served without first checking the network.

:::note
Expiring on connect means the first read of each cached resource after the
worker (re)connects will revalidate against the network. Entries are not
deleted — if the network is unavailable, the expired entry is still served by
[`handleToapiRequest`](/tapi/worker/reference/handle-toapi-request/).
:::

### Streaming invalidations

The worker then reads the stream line by line. Each non-empty line is a
space-separated list of tags. For every line the worker invalidates those tags in
the cache and posts them to all open clients.

### Reconnection

If reading the stream fails for **any** reason — the connection dropped, the
server restarted, the device went offline — the worker logs a warning and
schedules a reconnect by calling `listenForInvalidations` again after `timeout`
milliseconds. Each reconnect increases `timeout` by 50%, so a backend that stays
unreachable is retried with a growing backoff instead of every 5 seconds
forever.

:::note
Because every reconnect re-runs the "expire everything" step above, a dropped
and restored connection leaves no stale data behind: after reconnecting, each
cached resource is revalidated on its next read.
:::

## Client messages

Both on initial connect and on each streamed invalidation, the worker sends a
`postMessage` to every open client containing the invalidated tags. Page-side
integrations (such as `@toapi/react`) listen for these messages to refresh the
relevant queries. You generally do not need to handle these messages yourself
unless you are building a custom client integration.

## Related

- [`handleToapiRequest`](/tapi/worker/reference/handle-toapi-request/)
- [`cleanup`](/tapi/worker/reference/cleanup/)
- [Service worker setup guide](/tapi/worker/guides/service-worker/)
