---
title: "TResponse"
description: "The Response subclass returned from Toapi handlers — JSON, NDJSON, and empty responses plus cache tagging support."
---

`TResponse` is a subclass of the standard Web API `Response` object. It provides helper methods for creating common response types (like JSON) and adds support for features such as cache tagging. It is re-exported from `@toapi/server` (and defined in `@toapi/common`).

## Usage

In your route handlers, return instances of `TResponse` (usually via `TResponse.json`) to preserve type safety for the client.

```ts
import { defineHandler, TResponse } from "@toapi/server";

export const GET = defineHandler({ authorize: () => true }, async () => {
  return TResponse.json({ message: "Hello World" });
});
```

## Static methods

### `TResponse.json(data, init?)`

Creates a JSON response with the correct `Content-Type` header.

- **Parameters**:
  - `data`: The JSON-serializable data to return. The type is inferred and enforced by the handler schema if present.
  - `init`: (Optional) A `TResponseInit` object, which extends the standard `ResponseInit`.
- **Returns**: `TResponse<T>`

```ts
return TResponse.json(
  { id: 1, name: "Item" },
  { status: 201 }, // Created
);
```

### `TResponse.ndjson(gen, init?)`

Creates a streaming [NDJSON](https://github.com/ndjson/ndjson-spec) response from an async generator. Each yielded value is serialized as a JSON line followed by a newline character (`\n`). Sets `Content-Type: application/x-ndjson`.

- **Parameters**:
  - `gen`: An `AsyncGenerator<T>` that yields the items to stream.
  - `init`: (Optional) A `TResponseInit` object.
- **Returns**: `TResponse<AsyncGenerator<T>>`

```ts
export const GET = defineHandler({ authorize: () => true }, async () => {
  async function* streamItems() {
    yield { id: 1, name: "First" };
    yield { id: 2, name: "Second" };
  }

  return TResponse.ndjson(streamItems());
});
```

### `TResponse.void(init?)`

Creates an empty response with no body (sets the `Content-Length: 0` header). Its `data` resolves to `undefined`.

This is useful for routes that should return `undefined` on the client — which is the type React forms natively expect.

- **Parameters**:
  - `init`: (Optional) `TResponseInit` object.
- **Returns**: `TResponse<void>`

```ts
return TResponse.void();
```

When, for example, your `POST /users` endpoint returns `TResponse.void()`, you can use it on the client as:

```tsx
<form action={client.users.post}>
```

## Cache tags

`TResponse` supports a tag-based cache invalidation system. When providing the `init` object, pass a `cache` object with a `tags` array. These tags are sent in the `X-TAPI-Tags` header and drive invalidation across the server, service worker, and client cache layers (see [Caching Strategies](/tapi/server/reference/caching/)).

```ts
// GET /books/1
return TResponse.json(book, {
  cache: { tags: [`book-${book.id}`, "books-list"] },
});
```

For mutating requests (`POST`, `PUT`, `PATCH`, `DELETE`), tags attached to the response are invalidated after the handler runs, revalidating any cached GET responses that share a tag.

## Properties

### `data`

The `TResponse` instance holds a reference to the raw data passed to `TResponse.json()` (or the generator for `ndjson`). This is useful for internal testing or server-side usage where you want to access the typed object directly without parsing the body stream.

```ts
const response = TResponse.json({ success: true });
console.log(response.data.success); // true
```

### `cache`

The resolved `{ tags?, ttl? }` cache options attached to the response, if any. The [request handler](/tapi/server/reference/create-request-handler/) reads this to decide whether and how to store or invalidate.

## Interface

`TResponseInit` extends the standard `ResponseInit` with optional `cache` and `cookies` properties.

```ts
interface TResponseInit extends ResponseInit {
  cache?: {
    tags?: string[];
    ttl?: number;
  };
  cookies?: CookieStore;
}
```

- `cache.tags`: An array of string tags used for cache invalidation. Emitted as the `X-TAPI-Tags` header.
- `cache.ttl`: Time-to-live in seconds for caching the response. Emitted as the `X-TAPI-Expires-At` header (an absolute timestamp).
- `cookies`: A [`CookieStore`](/tapi/server/reference/t-request/#cookies) whose queued cookies are written onto the response headers.

When both `tags` and `ttl` are specified, the cache entry is invalidated by whichever comes first — the TTL expiring or a tag being revalidated.

## Related

- [`TRequest`](/tapi/server/reference/t-request/) — the request counterpart.
- [Caching Strategies](/tapi/server/reference/caching/) — how `tags` and `ttl` propagate through the cache layers.
