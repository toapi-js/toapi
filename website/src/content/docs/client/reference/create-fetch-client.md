---
title: "createFetchClient"
description: "Create a type-safe fetch client for a Toapi server: a proxy mirroring your API with caching, revalidation, and subscriptions."
---

The `createFetchClient` function creates a type-safe client for your API to be used in browser (or any `fetch`-capable) environments. It uses a `Proxy` to dynamically build request URLs based on the property access path, matching the structure defined in `defineApi`.

## Usage

```ts
import { createFetchClient } from "@toapi/client";
import type { api } from "./api"; // Your API definition type

const client = createFetchClient<typeof api.routes>("https://api.example.com");

// Usage
await client.users.get();
await client.users["123"].post({ name: "New Name" });
```

## Signature

```ts
function createFetchClient<Routes>(
  apiUrl: string,
  options?: Options,
): Client<Routes>;
```

The `Routes` type parameter is the type of your server's **routes map** (`typeof api.routes`). `ApiDefinition` wraps that map, so pass `api.routes` — not `api` itself. It drives inference for every path, query, body, and response type on the returned `Client`.

## Parameters

### `apiUrl`

**Type**: `string`

The base URL of your API server (e.g., `https://example.com/api`). Every request path is appended to this value.

### `options`

**Type**: `object` (optional)

| Property           | Type                                                    | Default                            | Description                                                                                                                                       |
| ------------------ | ------------------------------------------------------- | ---------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------- |
| `fetch`            | `(url: string, init: RequestInit) => Promise<Response>` | global `fetch`                     | Custom fetch implementation. Useful for mocking, server-side rendering, or adding global middleware/interceptors.                                 |
| `minTTL`           | `number`                                                | `5000`                             | Milliseconds a cache entry with no active subscribers is retained before being dropped.                                                           |
| `maxOverdueTTL`    | `number`                                                | `1000`                             | Upper bound (ms) of the random jitter added when scheduling background revalidations, to avoid stampedes.                                         |
| `logger`           | [`Logger`](#logger)                                     | `console`                          | Object with an optional `error(err)` method used to report fetch/revalidation errors.                                                             |
| `invalidationsUrl` | `string \| false`                                       | `apiUrl + "/__tapi/invalidations"` | URL of the server-sent invalidation stream. Pass `false` to disable server-push revalidation entirely, or a string to point at a custom endpoint. |

## Return value

Returns a `Client<Routes>` — a proxy whose properties mirror your API structure. See [How routes map to methods](/tapi/client/#how-routes-map-to-methods) for the full mapping.

## Client methods

For any route path, you can call standard HTTP methods.

### `.get(query?, req?)`

Performs a `GET` request.

- **`query`** _(optional)_ — an object of query parameters. Required only if the route schema declares required query params; otherwise optional.
- **`req`** _(optional)_ — a standard `RequestInit`.
- **Returns** — a `Promise` resolving to the typed response data, augmented with a [`.subscribe()`](/tapi/client/reference/observable/) method for cache updates.

```ts
const users = await client.users.get({ active: true });
```

### `.post(body?, req?)` &nbsp;·&nbsp; `.put(body?, req?)` &nbsp;·&nbsp; `.patch(body?, req?)`

Performs a mutation request.

- **`body`** — the request body (a JSON-serializable value or a `FormData` instance).
- **`req`** _(optional)_ — a `RequestInit` that may also carry `query` params via `req.query`.
- **Returns** — a `Promise` resolving to the response data, augmented with a `.revalidated` promise that settles once tag-based revalidation triggered by the response has completed.

```ts
await client.users.post({ name: "Alice" });

// with query params
await client.users.post({ name: "Alice" }, { query: { notify: true } });
```

### `.delete(query?, req?)`

Performs a `DELETE` request (no body).

- **`query`** _(optional)_ — query parameters.
- **`req`** _(optional)_ — request configuration.
- **Returns** — a `Promise` augmented with `.revalidated`, like the other mutations.

### `.revalidate(query?)`

Forces a re-fetch of the cached `GET` entry for the same URL and resolves once it completes. Use it to imperatively refresh data. See [Revalidation & subscriptions](/tapi/client/reference/revalidation/).

## Features

### Smart caching & deduplication

The client caches in-flight and resolved `GET` requests by URL. If you call `client.users.get()` multiple times while a request is pending, only one network call is made and all callers share the result.

### Tag-based revalidation

The client tracks cache tags sent by the server via the `X-TAPI-Tags` header on `GET` responses (specified server-side with `cache: { tags: [...] }`). When a mutation response includes tags matching cached `GET` requests, those requests are invalidated and re-fetched if they have active subscribers. The client can also receive tags pushed from the server over the `invalidationsUrl` stream.

### Subscriptions

The promise returned by `.get()` is augmented with a `.subscribe()` method, useful for integrating with React hooks or other state management to listen for cache updates.

```ts
const promise = client.users.get();
const unsubscribe = promise.subscribe((next) => {
  next.then((data) => console.log("Data updated:", data));
});
```

## `Logger`

`Logger` is re-exported from `@toapi/common`. It is a minimal interface used for log reporting:

```ts
interface Logger {
  error?: (error: unknown) => void | Promise<void>;
  warn?: (message: string) => void | Promise<void>;
  info?: (message: string) => void | Promise<void>;
}
```

Pass one via `options.logger` to route client-side fetch and revalidation errors to your own logging system.

## Related

- [Observable](/tapi/client/reference/observable/)
- [Route types](/tapi/client/reference/route-types/)
- [HttpError](/tapi/client/reference/http-error/)
- [Revalidation & subscriptions](/tapi/client/reference/revalidation/)
