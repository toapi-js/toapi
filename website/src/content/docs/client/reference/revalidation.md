---
title: "Revalidation & subscriptions"
description: "How the client keeps GET data fresh: tag-based invalidation, imperative revalidate(), server-push, and the .revalidated promise."
---

The client's cache is not a passive store — it actively keeps `GET` data fresh. There are four mechanisms, all built on the same tag index and [`Observable`](/tapi/client/reference/observable/) subscription protocol.

## 1. Tag-based revalidation after mutations

Toapi servers attach cache **tags** to `GET` responses via the `X-TAPI-Tags` header (declared server-side with `cache: { tags: [...] }`). The client records which URLs carry which tags in a tag index.

Every mutation response (`POST`/`PUT`/`PATCH`/`DELETE`) may also carry an `X-TAPI-Tags` header listing the tags it invalidated. When such a response arrives, the client looks up every cached URL that shares any of those tags and revalidates them. Cached entries with active subscribers are re-fetched and their subscribers notified with the fresh value.

```ts
// This POST responds with, say, tag "users".
// Any cached GET tagged "users" is revalidated automatically.
await client.users.post({ name: "Alice" });
```

### Awaiting revalidation: `.revalidated`

The promise returned by a mutation is augmented with a `.revalidated` property:

```ts
type Revalidating = {
  revalidated: Promise<void>;
};
```

`.revalidated` settles once the tag-based revalidation triggered by that mutation's response has completed. Await it when you need dependent `GET`s to be up to date before proceeding:

```ts
const result = client.users.post({ name: "Alice" });
await result;                // the created user
await result.revalidated;    // all matching GET caches are now fresh
```

## 2. Imperative revalidation: `.revalidate()`

Call `.revalidate(query?)` on any route to force a re-fetch of its `GET` cache entry. It resolves once the fresh response has been applied:

```ts
await client.users.revalidate();
await client.users.get({ active: true }); // now served from fresh cache
```

If a request for that URL is already in flight, `revalidate` queues a follow-up revalidation rather than issuing a redundant parallel request.

## 3. Time-based revalidation

When the server sends an `X-TAPI-Expires-At` header, the client schedules a background revalidation for that time (plus a small random jitter bounded by `maxOverdueTTL`, to avoid stampedes) — but only while the entry has active subscribers. Entries without subscribers are simply dropped after `minTTL`. Both `minTTL` and `maxOverdueTTL` are configurable via [`createFetchClient` options](/tapi/client/reference/create-fetch-client/#options).

## 4. Server-pushed invalidation

The client can receive invalidations pushed by the server, so open views stay current even when the change originates elsewhere (another user, a background job). This happens over a long-lived connection to the invalidations route:

- **`INVALIDATIONS_ROUTE`** — re-exported from `@toapi/client`, equal to `"/__tapi/invalidations"`. By default the client connects to `apiUrl + INVALIDATIONS_ROUTE`.
- The stream is newline-delimited; each line is a space-separated list of tags to revalidate.
- The client reconnects automatically with exponential backoff on network errors.
- In browsers with a controlling **service worker**, invalidations are instead delivered via `postMessage` (event type `TAPI_INVALIDATE_TAGS`), and the direct stream is not opened.

Configure or disable this with the `invalidationsUrl` option:

```ts
// custom endpoint
createFetchClient<typeof api.routes>(apiUrl, {
  invalidationsUrl: "https://example.com/api/__tapi/invalidations",
});

// disable server-push entirely
createFetchClient<typeof api.routes>(apiUrl, { invalidationsUrl: false });
```

## Subscriptions tie it together

All revalidation ultimately surfaces through subscribers. Awaiting a `.get()` gives you a one-shot value; **subscribing** keeps the entry alive and delivers every subsequent value:

```ts
const result = client.todos.get();
const unsubscribe = result.subscribe((next) => {
  next.then((todos) => render(todos));
});

// A mutation elsewhere, a server push, or a timed revalidation
// will now re-run this callback with fresh data.

unsubscribe(); // stop listening; entry is cleared after minTTL
```

See [Observable](/tapi/client/reference/observable/) for the subscription API in detail.

## Related

- [createFetchClient](/tapi/client/reference/create-fetch-client/)
- [Observable](/tapi/client/reference/observable/)
- [HttpError](/tapi/client/reference/http-error/)
