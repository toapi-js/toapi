---
title: "createRequestHandler"
description: "Turn an API definition into a Web-standard request handler — the primary server entry point for @toapi/server."
---

`createRequestHandler` is the primary server entry point. It takes the [`ApiDefinition`](/tapi/server/reference/api-definition/) produced by [`defineApi`](/tapi/server/reference/define-api/) and returns an async function that maps a standard Web API `Request` to a `Response`. Because it works with the Web-standard `Request`/`Response` objects, the returned handler drops straight into Astro, Next.js, Hono, Bun, Deno, Cloudflare Workers, or any other Web-standard runtime — no adapter required.

## Usage

```ts
// src/server.ts
import { createRequestHandler } from "@toapi/server";
import { api } from "./api";

const handler = createRequestHandler(api, {
  basePath: "/api",
});

// handler: (req: Request) => Promise<Response>
```

See the framework guides for how to wire the handler into a specific runtime: [Astro](/tapi/server/guides/astro/), [Next.js](/tapi/server/guides/nextjs/), [Hono](/tapi/server/guides/hono/), [Bun](/tapi/server/guides/bun/).

## Signature

```ts
function createRequestHandler(
  api: ApiDefinition<Routes>,
  options?: {
    basePath?: string;
    defaultTTL?: number;
  },
): (req: Request) => Promise<Response>;
```

## Parameters

### `api`

**Type**: [`ApiDefinition`](/tapi/server/reference/api-definition/)

The API definition object returned by [`defineApi`](/tapi/server/reference/define-api/), containing the complete map of routes, the cache instance, the optional OpenAPI info, and the optional logger.

### `options`

| Option       | Type     | Default             | Description                                                                                                                                                                            |
| ------------ | -------- | ------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `basePath`   | `string` | `""`                | The root path under which all routes are mounted. Must match the URL prefix where you serve the handler (e.g. `/api`). It is prepended to every route path and to the built-in routes. |
| `defaultTTL` | `number` | `1209600` (14 days) | The default time-to-live, in seconds, applied to cached responses whose handler set `cache` without an explicit `ttl`.                                                                 |

## Path matching

Route paths registered with [`defineApi().route()`](/tapi/server/reference/define-api/) support three segment kinds, compiled to a regular expression at startup:

- **Static segments** — `/users` matches exactly.
- **Named parameters** — `:id` in `/users/:id` matches a single path segment and is exposed via `req.params().id`.
- **Wildcards** — `*` matches everything remaining (including slashes). `*name` captures the remainder into a named parameter, so `/files/*path` exposes `req.params().path`.

Parameter values are URL-decoded before validation.

## Built-in routes

The handler responds to two reserved routes under `basePath` before matching your own routes:

| Route                             | Condition                             | Behaviour                                                                                                                                                                                        |
| --------------------------------- | ------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `<basePath>/__tapi/invalidations` | always                                | Opens the long-polling invalidation stream via [`streamRevalidatedTags`](/tapi/server/reference/stream-revalidated-tags/). Clients and service workers connect here to receive revalidated tags. |
| `<basePath>/__tapi/openapi.json`  | only when `oas` is set on `defineApi` | Serves the generated OpenAPI document (see [`generateOpenAPISchema`](/tapi/server/reference/generate-openapi-schema/)). The document is generated once and cached in memory.                     |

:::note
Because the invalidation stream is mounted automatically, you do not need a separate route for it when you use `createRequestHandler`. You only wire up [`streamRevalidatedTags`](/tapi/server/reference/stream-revalidated-tags/) by hand when your framework handles that path outside the Toapi handler.
:::

## Request lifecycle

For each request the handler resolves the matching route and dispatches by HTTP method:

1. **Prepare the request.** The incoming `Request` is augmented into a [`TRequest`](/tapi/server/reference/t-request/), giving lazy, validated accessors for `params()`, `query()`, `cookies()`, `data()`, and `invalidate()`.
2. **Authorize.** The handler's `authorize` function runs. If it returns a falsy value, the request is rejected with `401 Unauthorized`. The resolved value is available via `req.auth()`.
3. **Serve from cache (GET/HEAD only).** If a `cache` is configured on the API and a fresh entry exists for the request URL, the stored response is served without invoking the handler. `HEAD` requests receive the cached headers with no body.
4. **Run the handler.** The handler function executes and returns a [`TResponse`](/tapi/server/reference/t-response/). If a `response` schema is defined, the returned data is validated against it.
5. **Cache the response (GET/HEAD only).** If the response carries `cache` options and `req.auth()` was **not** called during handling, the response is stored using the response's `tags` and `ttl` (falling back to `defaultTTL`).
6. **Invalidate tags (mutations).** For `POST`, `PUT`, `PATCH`, and `DELETE`, if the response carries `cache.tags`, those tags are invalidated via the cache's pub/sub. When a session cookie is present, the originating client ID is forwarded so a client does not receive its own invalidation echo.

:::note
A response is **not** cached if the handler reads `req.auth()`. Reading auth data marks the response as user-specific and therefore not safe to store in the shared server cache. Authorize a request without consuming the auth value if you want it to be cacheable.
:::

## Error handling

Errors thrown while handling a request are logged (via the [`Logger`](/tapi/server/reference/define-api/#logger-interface) supplied to `defineApi`, or `console.error`) and converted into responses:

| Thrown value                                      | Response                                                                                        |
| ------------------------------------------------- | ----------------------------------------------------------------------------------------------- |
| `ZodError` (validation failure)                   | `400 Bad Request`, body is the array of Zod issues, `Content-Type: application/json+zodissues`. |
| [`HttpError`](/tapi/server/reference/http-error/) | The error's `status`, body `{ message, data }`, `Content-Type: application/json+httperror`.     |
| Anything else                                     | `500 Internal Server Error`.                                                                    |

Requests that match no route, or match a route with no handler for the given method, return `404 Not Found`.

## Related

- [`defineApi`](/tapi/server/reference/define-api/) — build the API definition passed in here.
- [Caching Strategies](/tapi/server/reference/caching/) — how the cache layers cooperate.
