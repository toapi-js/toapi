---
title: "@toapi/common"
description: "The shared type contract and utilities that @toapi/server and @toapi/client build on — Route, Handler, Schema, HttpError, and more."
---

`@toapi/common` holds the shared foundation of the Toapi stack: the type contract that describes a route (`Route`, `Handler`, `Schema`, `TRequest`, `TResponse`) plus a handful of runtime utilities (`HttpError`, `CookieStore`, `isMutation`, protocol constants, and the `Logger` interface).

:::note
Most applications never import `@toapi/common` directly. Its public exports are re-exported from [`@toapi/server`](/tapi/server/) and [`@toapi/client`](/tapi/client/) — depend on those packages instead. This page documents the shared contract at a reference level so you understand what those packages hand you.
:::

## What lives here

Because the server and client must agree on exactly what a route looks like — its params, query, body, response, and authorization — those shapes are defined once, in `@toapi/common`, and consumed by both sides. This is what makes an API end-to-end typed: the client infers its call signatures from the very same types the server implements.

## Type contract

### `Route`

Describes a single endpoint as a map of HTTP methods to [`Handler`](#handler)s. Each method is optional; a method typed as `never` for its query means it is not available on the route.

```ts
export type Route<
  Params extends Record<string, string>,
  GetResponse,
  GetQuery extends Record<string, unknown>,
  PostResponse,
  PostQuery extends Record<string, unknown>,
  PostBody,
  DeleteResponse,
  DeleteQuery extends Record<string, unknown>,
  PutResponse,
  PutQuery extends Record<string, unknown>,
  PutBody,
  PatchResponse,
  PatchQuery extends Record<string, unknown>,
  PatchBody
> = {
  GET?: Handler<GetResponse, Params, GetQuery, undefined>;
  HEAD?: Handler<undefined, Params, GetQuery, undefined>;
  POST?: Handler<PostResponse, Params, PostQuery, PostBody>;
  DELETE?: Handler<DeleteResponse, Params, DeleteQuery, undefined>;
  PUT?: Handler<PutResponse, Params, PutQuery, PutBody>;
  PATCH?: Handler<PatchResponse, Params, PatchQuery, PatchBody>;
};
```

`HEAD` mirrors the `GET` query but always resolves to an empty (`undefined`) response body. `GET`, `DELETE`, `HEAD` never carry a request body.

### `BaseRoute`

A loosely-typed version of `Route` (every generic widened to `any`) used internally where the precise per-method types are not needed.

```ts
export type BaseRoute = {
  GET?: Handler<any, any, any, undefined>;
  HEAD?: Handler<undefined, any, any, undefined>;
  POST?: Handler<any, any, any, any>;
  DELETE?: Handler<any, any, any, undefined>;
  PUT?: Handler<any, any, any, any>;
  PATCH?: Handler<any, any, any, any>;
};
```

### `Handler` and `HandlerFn`

A `Handler` pairs a [`Schema`](#schema) with the function that runs the request. `HandlerFn` is the function signature: it receives a [`TRequest`](/tapi/server/reference/t-request/) and resolves to a [`TResponse`](/tapi/server/reference/t-response/).

```ts
export type HandlerFn<Response, AuthData, Params, Query, Body> = (
  request: TRequest<AuthData, Params, Query, Body>
) => Promise<TResponse<Response>>;

export type Handler<
  Response,
  Params extends Record<string, string>,
  Query extends Record<string, unknown>,
  Body
> = {
  schema: Schema<Response, unknown, Params, Query, Body>;
  handler: HandlerFn<Response, any, Params, Query, Body>;
};
```

You do not usually construct a `Handler` by hand — `defineHandler` from [`@toapi/server`](/tapi/server/) produces one for you.

### `Schema`

Describes the runtime validation and authorization for a handler. Each field is a [Zod](https://zod.dev) schema (using `zod/v4`) that both validates input and documents the endpoint (Toapi derives its OpenAPI document from these). The `__r`, `__q`, and `__b` fields are phantom type carriers — they exist only to thread the Response, Query, and Body types through inference and are never populated at runtime.

```ts
export type Schema<
  Response,
  AuthData,
  Params extends Record<string, string>,
  Query extends Record<string, unknown>,
  Body
> = {
  __r?: Response;
  __q?: Query;
  __b?: Body;
  params?: { [key in keyof Params]: ZodType<Params[key], string> };
  query?: {
    [key in keyof Query]: ZodType<Query[key], string | string[] | undefined>;
  };
  body?: ZodType<Body>;
  response?: ZodType<Response>;
  authorize: (
    req: TRequest<never, Params, Query, never>
  ) => MaybePromise<AuthData>;
};
```

The `authorize` callback runs before the handler and returns the auth data later exposed as `request.auth()`. Throw an [`HttpError`](/tapi/common/reference/http-error/) from it to reject the request.

### `TRequest`

The augmented `Request` passed to every handler. Documented in full under the server section: see [TRequest](/tapi/server/reference/t-request/).

```ts
export type TRequest<AuthData, Params, Query, Body> = Request & {
  params: () => Params;
  query: () => Query;
  data: () => Promise<Body>;
  auth: () => NonNullable<AuthData>;
  cookies: () => CookieStore;
  invalidate: (tags: string[]) => Promise<void>;
};
```

### `TResponse`

The `Response` subclass handlers return, adding cache tagging, cookies, and typed `data`. Documented in full under the server section: see [TResponse](/tapi/server/reference/t-response/).

### `Path` and `StrictParams`

Template-literal helper types used to type route pathnames and extract their parameters. `Path` is any string starting with `/`. `StrictParams` reads a pathname like `/users/:id/*rest` and produces `{ id: string; rest: string }` — named segments (`:id`) and wildcards (`*rest`) become string keys, while a bare `*` is dropped.

```ts
export type Path = `/${string}`;
export type StrictParams<Pathname> = /* extracts params from the pathname */;
```

### `MaybePromise`

```ts
export type MaybePromise<T> = T | Promise<T>;
```

A value that may or may not be wrapped in a promise. Used wherever a callback (such as `authorize`) is allowed to be sync or async.

### `Logger`

An optional log sink you can pass to the request handler, the fetch client, or the service worker. Every method is optional and may return a promise; consumers fall back to the matching `console` method for the ones you leave out.

```ts
export interface Logger {
  error?: (error: unknown) => MaybePromise<void>;
  warn?: (message: string) => MaybePromise<void>;
  info?: (message: string) => MaybePromise<void>;
}
```

`error` receives the thrown value itself, while `warn` and `info` receive a preformatted message string. Not every consumer uses all three — the server request handler and the fetch client only report through `error`; `@toapi/worker` also uses `warn` and `info` for revalidation-stream retries and progress.

## Runtime values

### `HttpError`

The error class used to return a specific HTTP status from a handler or `authorize`. See the dedicated page: [HttpError](/tapi/common/reference/http-error/).

### `CookieStore`

A [Cookie Store API](https://developer.mozilla.org/en-US/docs/Web/API/CookieStore)-style wrapper over a request's cookies, obtained inside a handler via `request.cookies()`. It reads the incoming `Cookie` header and buffers changes, which `TResponse` flushes to `Set-Cookie` headers on the way out.

```ts
const cookies = request.cookies();

await cookies.get("__tapi-session"); // CookieListItem | undefined
await cookies.getAll();              // CookieListItem[]
await cookies.set("theme", "dark");  // or set({ name, value, path, ... })
await cookies.delete("theme");
```

| Method | Description |
| --- | --- |
| `get(nameOrOptions?)` | The first matching cookie, or the first cookie when called with no argument. |
| `getAll(nameOrOptions?)` | All cookies, or all cookies with a given name. |
| `set(nameOrOptions, value?)` | Add or update a cookie. Accepts a name + value or a `CookieInit` object (`domain`, `path`, `expires`, `sameSite`, `partitioned`). |
| `delete(nameOrOptions)` | Remove a cookie by name or `CookieStoreDeleteOptions`. |
| `write(headers)` | Flush buffered `set`/`delete` operations onto a `Headers` object as `Set-Cookie`. Called for you by `TResponse`. |

:::note
Cookies default to `Path=/` and `SameSite=Lax`. The `write` step only emits headers for cookies you actually changed or deleted during the request.
:::

### `isMutation`

Returns `true` when a request uses a mutating method (`POST`, `PUT`, `PATCH`, or `DELETE`). Used across the stack to decide whether a request should invalidate cache tags.

```ts
import { isMutation } from "@toapi/common";

isMutation(new Request("/x", { method: "POST" })); // true
isMutation(new Request("/x"));                      // false (GET)
```

### Constants

Protocol constants shared by server, client, and worker. You rarely reference these directly, but they define the wire format for cache tagging and invalidation.

```ts
export const TAGS_HEADER = "X-TAPI-Tags";
export const EXPIRES_AT_HEADER = "X-TAPI-Expires-At";
export const TAGS_CONTENT_TYPE = "text/tapi-tags";
export const SESSION_COOKIE_NAME = "__tapi-session";
export const INVALIDATION_POST_EVENT = "TAPI_INVALIDATE_TAGS";
export const INVALIDATIONS_ROUTE = "/__tapi/invalidations";
export const OPENAPI_ROUTE = "/__tapi/openapi.json";
```

| Constant | Value | Purpose |
| --- | --- | --- |
| `TAGS_HEADER` | `X-TAPI-Tags` | Header carrying the space-separated cache tags of a response. |
| `EXPIRES_AT_HEADER` | `X-TAPI-Expires-At` | Header carrying a response's absolute expiry timestamp (ms). |
| `TAGS_CONTENT_TYPE` | `text/tapi-tags` | Content type used when streaming revalidated tags. |
| `SESSION_COOKIE_NAME` | `__tapi-session` | Name of the Toapi session cookie. |
| `INVALIDATION_POST_EVENT` | `TAPI_INVALIDATE_TAGS` | Message event name used to broadcast tag invalidations. |
| `INVALIDATIONS_ROUTE` | `/__tapi/invalidations` | Internal route that streams tag invalidations to clients. |
| `OPENAPI_ROUTE` | `/__tapi/openapi.json` | Internal route serving the generated OpenAPI document. |

## Related

- [HttpError](/tapi/common/reference/http-error/) — return typed HTTP error responses.
- [TRequest](/tapi/server/reference/t-request/) — the request object handlers receive.
- [TResponse](/tapi/server/reference/t-response/) — the response object handlers return.
- [Server overview](/tapi/server/) — how these types are turned into a running API.
