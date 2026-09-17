---
title: "Astro Setup"
description: "Integrate @toapi/server with Astro API routes — catch-all handler, typed client, and the revalidation stream."
---

This guide explains how to integrate Toapi with Astro API routes.

## Installation

```bash
npm install @toapi/server @toapi/client zod
# or
pnpm add @toapi/server @toapi/client zod
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

## 2. Create the route handler

Astro uses file-based routing. To handle all requests under `/api`, create a catch-all route file at `src/pages/api/[...tapi].ts`.

```ts
// src/pages/api/[...tapi].ts
import { createRequestHandler } from "@toapi/server";
import { api } from "../../api";
import type { APIRoute } from "astro";

const handler = createRequestHandler(api, {
  basePath: "/api",
});

// The ALL export handles all HTTP methods (GET, POST, etc.)
export const ALL: APIRoute = ({ request }) => {
  return handler(request);
};
```

## 3. Create the client

Create a client for browser-side usage.

```ts
// src/client.ts
import { createFetchClient } from "@toapi/client";
import type { api } from "./api";

export const client = createFetchClient<typeof api.routes>("/api");
```

## 4. Add a route

Create a route handler file. For example, `src/routes/hello.ts`:

```ts
// src/routes/hello.ts
import { defineHandler, TResponse } from "@toapi/server";

export const GET = defineHandler({ authorize: () => true }, async () => {
  return TResponse.json({ message: "Hello from Toapi!" });
});
```

Register it in `src/api.ts`:

```ts
// src/api.ts
import { defineApi } from "@toapi/server";

export const api = defineApi().route("/hello", import("./routes/hello"));
```

## 5. Usage

### Client-side usage

Use the client in your `<script>` tags or within UI framework components (React, Vue, Svelte, etc.) that run on the client.

```astro
---
// src/pages/client-demo.astro
---

<button id="fetch-btn">Fetch Data</button>
<div id="result"></div>

<script>
  import { client } from "../client";

  const btn = document.getElementById("fetch-btn");
  const result = document.getElementById("result");

  btn?.addEventListener("click", async () => {
    const data = await client.hello.get();
    if (result) result.innerText = data.message;
  });
</script>
```

### Server-side usage (frontmatter)

When fetching data inside Astro component frontmatter (which runs on the server during build or SSR), pair [`createRequestHandler`](/tapi/server/reference/create-request-handler/) with `createFetchClient` to call handlers directly without an HTTP round-trip.

First, set up a server client helper:

```ts
// src/server-client.ts
import { createFetchClient } from "@toapi/client";
import { createRequestHandler } from "@toapi/server";
import { api } from "./api";

const handler = createRequestHandler(api);
export const serverClient = createFetchClient<typeof api.routes>(
  "http://localhost",
  { fetch: (url, init) => handler(new Request(url, init)) },
);
```

Then use it in your Astro pages:

```astro
---
// src/pages/index.astro
import { serverClient } from "../server-client";

const data = await serverClient.hello.get();
---

<h1>{data.message}</h1>
```

## 6. Revalidation stream

To enable tag-based revalidation across all cache layers, expose the invalidation stream. `defineApi` automatically creates a [`PubSub`](/tapi/server/reference/pub-sub/) instance, so no extra setup is needed for single-host deployments.

The catch-all handler from step 2 already serves the stream at `/api/__tapi/invalidations`. If you prefer a dedicated route, add one at `src/pages/api/revalidate.ts`:

```ts
// src/pages/api/revalidate.ts
import { streamRevalidatedTags } from "@toapi/server";
import { api } from "../../api";
import type { APIRoute } from "astro";

export const GET: APIRoute = () => {
  return streamRevalidatedTags({
    cache: api.cache,
  });
};
```

For setting up a service worker that connects to this endpoint, see the [`@toapi/worker`](/tapi/worker/) package. For details on how the cache layers interact, see [Caching Strategies](/tapi/server/reference/caching/).
