---
title: "TRequest"
description: "The enhanced Request passed into Toapi handlers — typed, validated access to auth, params, query, body, and cookies."
---

`TRequest` is an enhanced version of the standard Web API `Request` object. It is passed as the argument to your route handlers and authorization functions, providing type-safe access to validated request data. It is re-exported (as a type) from `@toapi/server` and defined in `@toapi/common`.

## Interface

`TRequest` extends the standard `Request` interface, adding helper methods to access processed data.

```ts
type TRequest<AuthData, Params, Query, Body> = Request & {
  auth: () => AuthData;
  params: () => Params;
  query: () => Query;
  data: () => Promise<Body>;
  cookies: () => CookieStore;
  invalidate: (tags: string[]) => Promise<void>;
};
```

## Methods

### `auth()`

Returns the authentication data returned by the `authorize` function in your handler definition.

- **Returns**: `AuthData`

```ts
const user = req.auth();
console.log(user.id);
```

:::note
Calling `req.auth()` inside a `GET`/`HEAD` handler marks the response as user-specific, so the [request handler](/tapi/server/reference/create-request-handler/#request-lifecycle) will **not** store it in the shared server cache. Only read auth in handlers whose output genuinely depends on the current user.
:::

### `params()`

Returns the validated path parameters extracted from the URL.

- **Returns**: `Params` object
- Access route parameters defined with colon syntax (e.g. `/users/:id`) or wildcards. The keys match the parameter names in your route definition, validated against the handler's `params` schema.

```ts
// Route: /users/:id
const { id } = req.params();
```

### `query()`

Returns the validated query-string parameters.

- **Returns**: `Query` object
- Access parsed and validated URL search parameters, typed according to the `query` schema provided in [`defineHandler`](/tapi/server/reference/define-handler/). Repeated keys are collected into arrays.

```ts
// URL: /search?q=toapi&page=1
const { q, page } = req.query();
```

### `data()`

Returns the validated request body.

- **Returns**: `Promise<Body>`
- Asynchronously parses and validates the request body against the handler's `body` schema. This returns a Promise because reading the body stream is async. Calling it when no `body` schema was declared throws a `500`.

```ts
const { title, description } = await req.data();
```

### `cookies()`

Returns the [`CookieStore`](#cookiestore) for the current request, giving access to request cookies.

- **Returns**: `CookieStore`

```ts
const sessionCookie = await req.cookies().get("session");
```

### `invalidate(tags)`

Invalidates cached responses tagged with the given tags. Useful in mutation handlers (`POST`, `PUT`, `PATCH`, `DELETE`) when you want to purge stale cache entries.

- **Parameters**: `tags: string[]` — the cache tags to invalidate.
- **Returns**: `Promise<void>`
- Calls `cache.delete` with the provided tags. If a session cookie is present, the originating client ID is forwarded so per-client caches can be targeted and a client does not receive its own invalidation echo.

```ts
// After updating a post, invalidate related cache entries
await req.invalidate(["posts", `post:${id}`]);
```

## Standard request methods

Since `TRequest` inherits from `Request`, all standard request properties and methods remain available — though you should prefer the typed helpers where they exist.

- `req.headers`
- `req.method`
- `req.url`
- `req.signal`
- `req.formData()`
- `req.blob()`

```ts
// Checking raw headers
const userAgent = req.headers.get("User-Agent");
```

## CookieStore

`req.cookies()` returns a `CookieStore` (also exported from `@toapi/server`). Use `get(name)` to read a request cookie; queued writes can be flushed onto a response via the `cookies` field of [`TResponseInit`](/tapi/server/reference/t-response/#interface).

## Usage example

```ts
export const POST = defineHandler(
  {
    authorize: (req) => {
      // Access standard request props in authorize
      const token = req.headers.get("Authorization");
      return verify(token);
    },
    params: { id: z.string() },
    body: z.object({ name: z.string() }),
  },
  async (req) => {
    // Access typed helpers in the handler
    const user = req.auth(); // From authorize()
    const { id } = req.params(); // From URL path
    const { name } = await req.data(); // From request body

    return TResponse.json({ success: true });
  },
);
```

## Related

- [`defineHandler`](/tapi/server/reference/define-handler/) — where the schemas that type this request are declared.
- [`TResponse`](/tapi/server/reference/t-response/) — the response counterpart.
