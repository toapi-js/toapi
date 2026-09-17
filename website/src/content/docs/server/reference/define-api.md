---
title: "defineApi"
description: "The entry point for defining your Toapi surface — chain route definitions into a fully-typed map shared with the client."
---

The `defineApi` function is the entry point for defining your API structure. It creates an [`ApiDefinition`](/tapi/server/reference/api-definition/) container that lets you chain route definitions together, building a fully-typed map of your API that can be shared with the [client](/tapi/client/).

## Usage

Conventionally, this is defined in a shared file (e.g. `api.ts`) that is imported by both your server entry point and your client configuration.

```ts
// api.ts
import { defineApi } from "@toapi/server";

export const api = defineApi()
  .route("/users", import("./routes/users"))
  .route("/users/:id", import("./routes/user-detail"));
```

## Signature

```ts
function defineApi(options?: {
  cache?: Cache;
  oas?: { title: string; version: string };
  logger?: Logger;
}): ApiDefinition<{}>;
```

Returns an empty [`ApiDefinition`](/tapi/server/reference/api-definition/) instance.

## Options

| Option   | Type                                 | Default         | Description                                                                                                                                                                                                                                                                                                                                                                                                          |
| -------- | ------------------------------------ | --------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `cache`  | `Cache`                              | `new PubSub()`  | The cache / pub-sub instance used for tag-based revalidation and optional server-side response caching. Defaults to an in-process [`PubSub`](/tapi/server/reference/pub-sub/), which distributes invalidations but stores nothing. Pass a reference cache from [`@toapi/cache`](/tapi/cache/) to enable server-side caching, or a shared implementation such as `RedisCache` when running multiple server instances. |
| `oas`    | `{ title: string; version: string }` | `undefined`     | When set, an OpenAPI 3.1 document is generated from your routes and served at `<basePath>/__tapi/openapi.json`. See [`generateOpenAPISchema`](/tapi/server/reference/generate-openapi-schema/).                                                                                                                                                                                                                      |
| `logger` | `Logger`                             | `console.error` | Logger used by the request handler to report errors thrown by route handlers and cache operations. When omitted, errors are logged via `console.error`.                                                                                                                                                                                                                                                              |

### Logger interface

```ts
interface Logger {
  error?: (error: unknown) => void | Promise<void>;
  warn?: (message: string) => void | Promise<void>;
  info?: (message: string) => void | Promise<void>;
}
```

Pass any object that implements this shape — for example a Pino or Winston instance, or a custom function that ships errors to your observability platform.

:::note
The server request handler currently only calls `error`. Further methods may be added in future minor releases, so implementations should treat unknown methods as optional and ignore them.
:::

```ts
import { defineApi } from "@toapi/server";

export const api = defineApi({
  logger: {
    error: (error) => reportToSentry(error),
  },
});
```

## Methods

### `.route(path, module)`

Registers a route at a specific path.

- **Parameters**:
  - `path`: A string representing the URL path. It supports:
    - Static segments: `/users`
    - Dynamic parameters: `/users/:id`
    - Wildcards: `/files/*` or `/files/*path`
  - `module`: A route object, or a Promise that resolves to a module containing the route handlers (usually via `import()`). Deferring with `import()` keeps route implementations out of the bundle until first hit.
- **Returns**: The same `ApiDefinition` instance, re-typed to include the added route in its type signature so calls can be chained.

### `.invalidate(tags)`

Invalidates cached responses tagged with the given tags by calling `cache.delete(tags)`. Use it to purge stale entries from outside a request handler (for example a background job).

```ts
await api.invalidate(["books", "book:42"]);
```

## Path patterns

### Static paths

Matches the exact URL path.

```ts
.route("/health", import("./routes/health"))
```

### Dynamic parameters

Matches a segment of the path and passes it to the handler.

```ts
.route("/posts/:postId/comments/:commentId", import("./routes/comment"))
```

In the handler these are accessed via `req.params().postId` and `req.params().commentId`.

### Wildcards

Matches everything remaining in the path.

```ts
// Matches /files/images/logo.png
.route("/files/*path", import("./routes/files"))
```

In the handler this is accessed via `req.params().path` (which would be `"images/logo.png"`).

## Type inference

The primary purpose of `defineApi` is to build a TypeScript type representing your entire API surface. This type is inferred automatically as you chain `.route()` calls, and is consumed by [`createFetchClient`](/tapi/client/) as `typeof api` to produce the fully-typed client.

## Related

- [`defineHandler`](/tapi/server/reference/define-handler/) — implement the handlers registered here.
- [`createRequestHandler`](/tapi/server/reference/create-request-handler/) — turn the definition into a request handler.
- [`ApiDefinition`](/tapi/server/reference/api-definition/) — the type returned by `defineApi`.
