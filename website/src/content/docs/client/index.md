---
title: "@toapi/client"
description: "The typed fetch client for Toapi: a proxy that mirrors your API, with deduplication, tag-based revalidation, and subscriptions."
---

`@toapi/client` provides a fully-typed `fetch` client for a Toapi server. You give it the type of your API definition, and it returns a client object whose shape mirrors your routes exactly — every path segment is a property, and every HTTP method is a callable. Request URLs, query parameters, request bodies, and response types are all inferred from your server's `defineApi` type, so a typo or a wrong body shape is a compile error, not a runtime surprise.

Under the hood the client is a `Proxy`. There is no generated code and no build step: property access builds the URL, and calling a method (`.get`, `.post`, `.put`, `.patch`, `.delete`) performs the request.

## Installation

```bash
npm install @toapi/client
```

## Quick start

```ts
import { createFetchClient } from "@toapi/client";
import type { api } from "./api"; // the type of your server's defineApi(...)

const client = createFetchClient<typeof api.routes>("https://example.com/api");

// GET /users
const users = await client.users.get();

// GET /users?active=true
const active = await client.users.get({ active: true });

// POST /users
await client.users.post({ name: "Alice" });

// GET /users/123
const user = await client.users["123"].get();

// DELETE /users/123
await client.users["123"].delete();
```

See [createFetchClient](/tapi/client/reference/create-fetch-client/) for the full reference.

## How routes map to methods

The property path you access on the client becomes the request path, and the final method call decides the HTTP verb.

| Server route | Client call | Request |
| --- | --- | --- |
| `GET /users` | `client.users.get()` | `GET /users` |
| `GET /users` (with query) | `client.users.get({ active: true })` | `GET /users?active=true` |
| `POST /users` | `client.users.post(body)` | `POST /users` |
| `GET /users/:id` | `client.users[id].get()` | `GET /users/{id}` |
| `PUT /users/:id` | `client.users[id].put(body)` | `PUT /users/{id}` |
| `DELETE /users/:id` | `client.users[id].delete()` | `DELETE /users/{id}` |

Dynamic segments (`:id`) and wildcard segments (`*rest`) accept a `string | number` in the client type, so `client.users[123]` and `client.users["123"]` are both valid.

### Method signatures

- **`.get(query?, req?)`** — performs a `GET`. The first argument is the query object (required only if the route declares required query params); the second is a standard `RequestInit`. Returns a promise that is also an [`Observable`](/tapi/client/reference/observable/).
- **`.post(body?, req?)` / `.put(body?, req?)` / `.patch(body?, req?)`** — performs a mutation with a JSON or `FormData` body. Query params go on `req.query`. Returns a promise augmented with `.revalidated` (see [Revalidation & subscriptions](/tapi/client/reference/revalidation/)).
- **`.delete(query?, req?)`** — performs a `DELETE`. The first argument is the query object; the body is always empty.
- **`.revalidate(query?)`** — forces a re-fetch of the matching `GET` cache entry. See below.

:::note
Bodies are JSON-encoded automatically and a `Content-Type: application/json` header is set for you, unless the body is a `FormData` instance (in which case the browser sets the multipart boundary) or you provide your own `Content-Type`.
:::

## Caching & deduplication

Every `GET` goes through an in-memory cache keyed by URL. If you call `client.users.get()` several times while a request is in flight, only one network request is made and all callers share the same promise. Once a response resolves, subsequent reads are served from the cache until the entry expires or is revalidated.

Cache lifetime is controlled by the `minTTL` and `maxOverdueTTL` options and by the `X-TAPI-Expires-At` header the server sends. Entries with no active subscribers are dropped after `minTTL`.

## Tag-based revalidation

Toapi servers attach cache **tags** to `GET` responses via the `X-TAPI-Tags` header. The client remembers which URLs carry which tags. When a mutation (`POST`/`PUT`/`PATCH`/`DELETE`) responds with tags that overlap cached `GET` requests, those requests are revalidated automatically, and any active subscribers receive the fresh data.

The client can also receive invalidations pushed from the server out-of-band, over a long-lived connection to the `/__tapi/invalidations` endpoint (or via a service worker `postMessage`). This keeps open views up to date when data changes on the server without the client having triggered the mutation itself. This is enabled by default and can be disabled with the `invalidationsUrl: false` option.

See [Revalidation & subscriptions](/tapi/client/reference/revalidation/) for details.

## Subscriptions

The promise returned by `.get()` also implements [`Observable`](/tapi/client/reference/observable/): call `.subscribe(callback)` to be notified whenever the cache entry for that URL changes. This is the primitive that [`@toapi/react`](/tapi/react/) builds its hooks on, but you can use it directly with any state-management approach.

```ts
const result = client.users.get();
const unsubscribe = result.subscribe((next) => {
  next.then((users) => console.log("users changed:", users));
});
```

## Error handling

Non-`2xx` responses reject with an [`HttpError`](/tapi/client/reference/http-error/) carrying the status code and any structured error payload. Validation failures reported by the server (`application/json+zodissues`) reject with a `ZodError`.

## Public exports

| Export | Kind | Reference |
| --- | --- | --- |
| `createFetchClient` | function | [createFetchClient](/tapi/client/reference/create-fetch-client/) |
| `Client` | type | [createFetchClient](/tapi/client/reference/create-fetch-client/) |
| `Observable` | type | [Observable](/tapi/client/reference/observable/) |
| `GetRoute`, `PostRoute` | types | [Route types](/tapi/client/reference/route-types/) |
| `HttpError` | class (re-export) | [HttpError](/tapi/client/reference/http-error/) |
| `INVALIDATIONS_ROUTE` | constant (re-export) | [Revalidation & subscriptions](/tapi/client/reference/revalidation/) |
| `Logger` | type (re-export) | [createFetchClient](/tapi/client/reference/create-fetch-client/) |
