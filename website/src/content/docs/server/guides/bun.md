---
title: "Bun Setup"
description: "Serve @toapi/server routes with Bun's native HTTP server — pass the Web-standard handler straight to Bun.serve."
---

This guide explains how to integrate Toapi with [Bun](https://bun.sh/)'s native HTTP server, `Bun.serve`. Because [`createRequestHandler`](/tapi/server/reference/create-request-handler/) works with standard Web API `Request` and `Response` objects, it can be passed straight to `Bun.serve` as its `fetch` handler — no adapter required.

## Installation

```bash
bun add @toapi/server @toapi/client zod
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

## 2. Create the server

Pass the handler returned by `createRequestHandler` directly to `Bun.serve`.

```ts
// src/index.ts
import { createRequestHandler } from "@toapi/server";
import { api } from "./api";

// ensure basePath matches the URL prefix where Toapi is mounted
const handler = createRequestHandler(api, {
  basePath: "/api",
});

const server = Bun.serve({
  port: 3000,
  async fetch(req) {
    const path = new URL(req.url).pathname;

    // Mount Toapi under /api
    if (path.startsWith("/api")) {
      return handler(req);
    }

    // Everything else falls through (static assets, other routes, ...)
    return new Response("Not Found", { status: 404 });
  },
});

console.log(`Listening on ${server.url}`);
```

If your server serves nothing but the Toapi routes, you can pass `handler` directly and drop the `basePath`:

```ts
const handler = createRequestHandler(api);

Bun.serve({
  port: 3000,
  fetch: handler,
});
```

### Using Bun's native routing

`Bun.serve` also exposes a `routes` option that matches paths with `:param` and `*` wildcards. You can mount Toapi on a catch-all pattern and keep `fetch` as the fallback for everything else:

```ts
// src/index.ts
import { createRequestHandler } from "@toapi/server";
import { api } from "./api";

const handler = createRequestHandler(api, {
  basePath: "/api",
});

Bun.serve({
  port: 3000,
  routes: {
    "/api/*": (req) => handler(req),
  },
  fetch() {
    return new Response("Not Found", { status: 404 });
  },
});
```

This lets you mix Toapi with other native Bun routes — for example a health check or a static response — without writing path checks by hand:

```ts
Bun.serve({
  port: 3000,
  routes: {
    "/health": new Response("ok"),
    "/api/*": (req) => handler(req),
  },
  fetch() {
    return new Response("Not Found", { status: 404 });
  },
});
```

## 3. Create the client

To consume your API, create the typed client.

```ts
// src/client.ts
import { createFetchClient } from "@toapi/client";
import type { api } from "./api";

export const client = createFetchClient<typeof api.routes>("http://localhost:3000/api");
```

## 4. Add a route

Create a route handler file. For example, `src/routes/hello.ts`:

```ts
// src/routes/hello.ts
import { defineHandler, TResponse } from "@toapi/server";

export const GET = defineHandler({ authorize: () => true }, async () => {
  return TResponse.json({ message: "Hello from Toapi on Bun!" });
});
```

Register it in `src/api.ts`:

```ts
// src/api.ts
import { defineApi } from "@toapi/server";

export const api = defineApi().route("/hello", import("./routes/hello"));
```

## 5. Usage

Start the server with Bun:

```bash
bun run src/index.ts
```

And call it using the typed client:

```ts
// src/example-usage.ts
import { client } from "./client";

async function main() {
  const response = await client.hello.get();
  console.log(response.message); // "Hello from Toapi on Bun!"
}

main();
```

## 6. Revalidation stream

To enable tag-based revalidation across all cache layers, expose the invalidation stream. `defineApi` automatically creates a [`PubSub`](/tapi/server/reference/pub-sub/) instance, so no extra setup is needed for single-host deployments.

The catch-all handler already serves the stream at `/api/__tapi/invalidations`. If you prefer a dedicated path, register `/api/revalidate` **before** the catch-all `/api` branch, otherwise the Toapi handler will swallow it:

```ts
// src/index.ts
import { createRequestHandler, streamRevalidatedTags } from "@toapi/server";
import { api } from "./api";

const handler = createRequestHandler(api, {
  basePath: "/api",
});

Bun.serve({
  port: 3000,
  async fetch(req) {
    const path = new URL(req.url).pathname;

    if (path === "/api/revalidate") {
      return streamRevalidatedTags({
        cache: api.cache,
      });
    }

    if (path.startsWith("/api")) {
      return handler(req);
    }

    return new Response("Not Found", { status: 404 });
  },
});
```

The equivalent using Bun's native routing — `/api/revalidate` is listed as a concrete route so it takes precedence over the `/api/*` catch-all:

```ts
Bun.serve({
  port: 3000,
  routes: {
    "/api/revalidate": () =>
      streamRevalidatedTags({
        cache: api.cache,
      }),
    "/api/*": (req) => handler(req),
  },
  fetch() {
    return new Response("Not Found", { status: 404 });
  },
});
```

For setting up a service worker that connects to this endpoint, see the [`@toapi/worker`](/tapi/worker/) package. For details on how the cache layers interact, see [Caching Strategies](/tapi/server/reference/caching/).
