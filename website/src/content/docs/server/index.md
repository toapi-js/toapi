---
title: "@toapi/server"
description: "Define fully-typed REST APIs and handle requests on the server with @toapi/server — validation, auth, and caching built in."
---

`@toapi/server` is the server half of the Toapi stack. You use it to **define your API surface**, **implement request handlers**, and **turn them into a request handler** that runs on any Web-standard runtime. The API definition doubles as the single source of truth for the fully-typed [`@toapi/client`](/tapi/client/) — client and server share the same types with no code generation and no build step.

## Installation

```bash
npm install @toapi/server zod
```

`zod` is a peer dependency used for request and response validation.

## Features

- **Type-safe by construction** — the shape you build with [`defineApi`](/tapi/server/reference/define-api/) is inferred into a single TypeScript type that the client consumes directly. No codegen, no compile step.
- **Auth by default** — every handler defined with [`defineHandler`](/tapi/server/reference/define-handler/) must supply an `authorize` function, making it nearly impossible to ship an unauthenticated endpoint by accident.
- **Built-in validation** — request `params`, `query`, and `body`, plus the `response`, are validated with [Zod](https://zod.dev/). Define the schema once and get types and runtime validation together.
- **Automatic OpenAPI** — an OpenAPI 3.1 document is generated from your route definitions. Pass `oas: { title, version }` to `defineApi` to serve it at `<basePath>/__tapi/openapi.json`.
- **Tag-based caching** — responses can carry cache `tags` and a `ttl`, driving invalidation across the server, service worker, and client cache layers.

## Quick start

Define the API surface in a shared file that both the server and the client import:

```ts
// src/api.ts
import { defineApi } from "@toapi/server";

export const api = defineApi().route("/hello", import("./routes/hello"));
```

Implement a handler:

```ts
// src/routes/hello.ts
import { defineHandler, TResponse } from "@toapi/server";
import { z } from "zod";

export const GET = defineHandler(
  {
    authorize: () => true,
    response: z.object({ message: z.string() }),
  },
  async () => {
    return TResponse.json({ message: "Hello, world!" });
  },
);
```

Turn the API into a request handler and mount it on your runtime of choice:

```ts
// src/server.ts
import { createRequestHandler } from "@toapi/server";
import { api } from "./api";

export const handler = createRequestHandler(api, { basePath: "/api" });
```

Consume it from the browser with the shared types — see [`@toapi/client`](/tapi/client/):

```ts
// Client usage
import { createFetchClient } from "@toapi/client";
import type { api } from "./api";

const client = createFetchClient<typeof api.routes>("/api");
const { message } = await client.hello.get();
```

:::tip
The client is typed entirely from `typeof api` — there is no generated SDK to keep in sync.
:::

## API reference

- [`createRequestHandler`](/tapi/server/reference/create-request-handler/) — turn an API definition into a Web-standard request handler (the primary server entry point).
- [`defineApi`](/tapi/server/reference/define-api/) — build the typed map of your routes.
- [`defineHandler`](/tapi/server/reference/define-handler/) — implement a single endpoint with validation and auth.
- [`generateOpenAPISchema`](/tapi/server/reference/generate-openapi-schema/) — produce an OpenAPI 3.1 document from your API.
- [`streamRevalidatedTags`](/tapi/server/reference/stream-revalidated-tags/) — expose the long-polling invalidation stream.
- [`TResponse`](/tapi/server/reference/t-response/) — the response helper returned from handlers.
- [`TRequest`](/tapi/server/reference/t-request/) — the enhanced request passed into handlers.
- [`HttpError`](/tapi/server/reference/http-error/) — throw typed HTTP error responses.
- [`PubSub` and the `Cache` interface](/tapi/server/reference/pub-sub/) — the pub/sub layer behind tag revalidation.
- [`ApiDefinition`](/tapi/server/reference/api-definition/) — the type produced by `defineApi`.

## Guides

- [Astro](/tapi/server/guides/astro/) — mount Toapi on Astro server endpoints.
- [Next.js](/tapi/server/guides/nextjs/) — integrate Toapi with the App Router.
- [Hono](/tapi/server/guides/hono/) — mount Toapi routes on a Hono server.
- [Bun](/tapi/server/guides/bun/) — mount Toapi routes on a native Bun server.

## Related packages

- [`@toapi/client`](/tapi/client/) — the fully-typed fetch client.
- [`@toapi/cache`](/tapi/cache/) — reference `Cache` implementations for server-side caching.
- [`@toapi/worker`](/tapi/worker/) — service-worker caching and offline support.
