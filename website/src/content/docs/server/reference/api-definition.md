---
title: "ApiDefinition"
description: "The typed container produced by defineApi — holds your route map, cache, OpenAPI info, and logger, and drives client type inference."
---

`ApiDefinition` is the type (and class) returned by [`defineApi`](/tapi/server/reference/define-api/). It holds the map of your routes plus the cache, optional OpenAPI info, and optional logger. Its generic parameter carries the fully-inferred shape of your API, which is what the [client](/tapi/client/) consumes via `typeof api` for end-to-end type safety.

You rarely construct an `ApiDefinition` yourself — you build one by chaining [`.route()`](/tapi/server/reference/define-api/#routepath-module) calls on `defineApi()`. You do pass it to other functions in the package.

## Type

```ts
class ApiDefinition<Routes extends Record<Path, unknown>> {
  routes: Routes;
  cache: Cache;
  oas?: { title: string; version: string };
  logger?: Logger;

  invalidate(tags: string[]): Promise<void>;
  route(path, route): ApiDefinition<Routes & { [path]: ... }>;
}
```

## Properties

| Property | Type | Description |
| --- | --- | --- |
| `routes` | `Routes` | The map of registered route paths to their handler modules. Referenced as `typeof api` (or `typeof api.routes`) when constructing a [client](/tapi/client/). |
| `cache` | [`Cache`](/tapi/server/reference/pub-sub/) | The cache / pub-sub instance. Pass it to [`streamRevalidatedTags`](/tapi/server/reference/stream-revalidated-tags/) when wiring the invalidation stream by hand. |
| `oas` | `{ title, version }` \| `undefined` | The OpenAPI info, when provided to `defineApi`. Enables the `/__tapi/openapi.json` route. |
| `logger` | [`Logger`](/tapi/server/reference/define-api/#logger-interface) \| `undefined` | The error logger used by the request handler. |

## Methods

### `.route(path, module)`

Registers a route and returns the same instance re-typed to include it. See [`defineApi` → `.route()`](/tapi/server/reference/define-api/#routepath-module).

### `.invalidate(tags)`

Invalidates cached responses tagged with any of `tags` by calling `cache.delete(tags)`. Useful for purging cache entries from outside a request handler (for example a cron job or webhook).

```ts
await api.invalidate(["books"]);
```

## Consuming the type on the client

```ts
// api.ts
export const api = defineApi().route("/books", import("./routes/books"));

// client.ts
import { createFetchClient } from "@toapi/client";
import type { api } from "./api";

export const client = createFetchClient<typeof api.routes>("/api");
```

## Related

- [`defineApi`](/tapi/server/reference/define-api/) — creates the `ApiDefinition`.
- [`createRequestHandler`](/tapi/server/reference/create-request-handler/) — consumes it to serve requests.
- [`generateOpenAPISchema`](/tapi/server/reference/generate-openapi-schema/) — reads its routes to build an OpenAPI document.
