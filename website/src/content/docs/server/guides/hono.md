---
title: "Hono Setup"
description: "Mount @toapi/server routes on a Hono app — the Web-standard handler drops straight onto app.all with no adapter."
---

This guide explains how to integrate Toapi with [Hono](https://hono.dev/), a fast, lightweight, web-standard web framework.

## Installation

```bash
npm install @toapi/server @toapi/client hono zod
# or
pnpm add @toapi/server @toapi/client hono zod
```

## 1. Define your API

Create a central file to define your API structure.

```ts
// src/api.ts
import { defineApi } from "@toapi/server";

export const api = defineApi();
// We'll add routes here later
// .route("/hello", import("./routes/hello"))
```

## 2. Integrate with Hono

Mount the Toapi request handler onto a Hono application instance. Since [`createRequestHandler`](/tapi/server/reference/create-request-handler/) works with standard Web API `Request` and `Response` objects, it integrates seamlessly with Hono.

```ts
// src/index.ts
import { Hono } from "hono";
import { createRequestHandler } from "@toapi/server";
import { api } from "./api";

const app = new Hono();

// Initialize the Toapi handler
// ensure basePath matches the route you mount it on
const handler = createRequestHandler(api, {
  basePath: "/api",
});

// Mount the handler on the /api prefix
// The '*' wildcard ensures all subpaths are captured
app.all("/api/*", (c) => {
  return handler(c.req.raw);
});

export default app;
```

If you are using a runtime like Bun, Deno, or Cloudflare Workers, `export default app` is usually a sufficient entry point.

## 3. Create the client

To consume your API, create the typed client.

```ts
// src/client.ts
import { createFetchClient } from "@toapi/client";
import type { api } from "./api";

// Adjust the URL to match your server's address
export const client = createFetchClient<typeof api.routes>("http://localhost:3000/api");
```

## 4. Add a route

Create a route handler file. For example, `src/routes/hello.ts`:

```ts
// src/routes/hello.ts
import { defineHandler, TResponse } from "@toapi/server";

export const GET = defineHandler({ authorize: () => true }, async () => {
  return TResponse.json({ message: "Hello from Toapi on Hono!" });
});
```

Register it in `src/api.ts`:

```ts
// src/api.ts
import { defineApi } from "@toapi/server";

export const api = defineApi().route("/hello", import("./routes/hello"));
```

## 5. Usage

Now you can start your Hono server and use the client to make requests.

```ts
// src/example-usage.ts
import { client } from "./client";

async function main() {
  const response = await client.hello.get();
  console.log(response.message); // "Hello from Toapi on Hono!"
}

main();
```

### Context integration

If you need to access Hono's `Context` (like environment variables in Cloudflare Workers) inside your Toapi handlers, you currently need to pass them via a custom storage mechanism (like `AsyncLocalStorage`) or by extending the request object before passing it to `handler`, since Toapi abstracts away the underlying framework's specific context object in favor of a standard [`TRequest`](/tapi/server/reference/t-request/).

## 6. Revalidation stream

To enable tag-based revalidation across all cache layers, expose the invalidation stream. `defineApi` automatically creates a [`PubSub`](/tapi/server/reference/pub-sub/) instance, so no extra setup is needed for single-host deployments.

The catch-all handler already serves the stream at `/api/__tapi/invalidations`. If you prefer a dedicated route, add `/api/revalidate` to your Hono app:

```ts
// src/index.ts
import { Hono } from "hono";
import { createRequestHandler, streamRevalidatedTags } from "@toapi/server";
import { api } from "./api";

const app = new Hono();

const handler = createRequestHandler(api, {
  basePath: "/api",
});

app.get("/api/revalidate", (c) => {
  return streamRevalidatedTags({
    cache: api.cache,
  });
});

app.all("/api/*", (c) => {
  return handler(c.req.raw);
});

export default app;
```

The revalidation route is registered before the catch-all so it doesn't get swallowed by the Toapi handler.

For setting up a service worker that connects to this endpoint, see the [`@toapi/worker`](/tapi/worker/) package. For details on how the cache layers interact, see [Caching Strategies](/tapi/server/reference/caching/).
