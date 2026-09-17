---
title: "Next.js Setup"
description: "Integrate @toapi/server with the Next.js App Router — a catch-all route handler, typed client, and server components."
---

This guide explains how to integrate Toapi with the Next.js App Router.

## Installation

```bash
npm install @toapi/server @toapi/client zod
# or
pnpm add @toapi/server @toapi/client zod
```

## 1. Define your API

Create a central file to define your API structure. This acts as the source of truth for your routes.

```ts
// src/api.ts
import { defineApi } from "@toapi/server";

export const api = defineApi();
// We'll add routes here later
// .route("/hello", import("./routes/hello"))
```

## 2. Create the route handler

The Next.js App Router uses a catch-all route to handle dynamic API requests. Create a file at `src/app/api/[...tapi]/route.ts`.

This handler captures all requests to `/api/*` and routes them through Toapi.

```ts
// src/app/api/[...tapi]/route.ts
import { createRequestHandler } from "@toapi/server";
import { api } from "@/api"; // Adjust import path as needed

const handler = createRequestHandler(api, {
  basePath: "/api",
});

export const GET = handler;
export const POST = handler;
export const PUT = handler;
export const PATCH = handler;
export const DELETE = handler;
```

## 3. Create the client

To consume your API from client components, create a type-safe fetch client.

```ts
// src/client.ts
"use client";

import { createFetchClient } from "@toapi/client";
import type { api } from "@/api";

// Assuming your API is mounted at /api
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

And register it in `src/api.ts`:

```ts
// src/api.ts
import { defineApi } from "@toapi/server";

export const api = defineApi().route("/hello", import("./routes/hello"));
```

## 5. Usage in components

### Client components

Use the client in your interactive components:

```tsx
// src/app/page.tsx
"use client";

import { useState, useEffect } from "react";
import { client } from "@/client";

export default function ClientPage() {
  const [msg, setMsg] = useState("");

  useEffect(() => {
    client.hello.get().then((data) => setMsg(data.message));
  }, []);

  if (!msg) return <div>Loading...</div>;

  return <h1>{msg}</h1>;
}
```

:::tip
For a first-class React integration with hooks and Suspense, see [`@toapi/react`](/tapi/react/).
:::

### Server components

For Server Components, skip the HTTP overhead by pairing [`createRequestHandler`](/tapi/server/reference/create-request-handler/) with `createFetchClient`. This calls your handlers directly.

Create a server-only client instance:

```ts
// src/server-client.ts
import { createFetchClient } from "@toapi/client";
import { createRequestHandler } from "@toapi/server";
import { api } from "@/api";

const handler = createRequestHandler(api);
export const serverClient = createFetchClient<typeof api.routes>(
  "http://localhost",
  { fetch: (url, init) => handler(new Request(url, init)) },
);
```

Then use it in your Server Component:

```tsx
// src/app/page.tsx
import { serverClient } from "@/server-client";

export default async function ServerPage() {
  const data = await serverClient.hello.get();

  return <h1>{data.message}</h1>;
}
```

## 6. Revalidation stream

To enable tag-based revalidation across all cache layers, expose the invalidation stream. `defineApi` automatically creates a [`PubSub`](/tapi/server/reference/pub-sub/) instance, so no extra setup is needed for single-host deployments.

The catch-all handler already serves the stream at `/api/__tapi/invalidations`. If you prefer a dedicated route, add one at `src/app/api/revalidate/route.ts`:

```ts
// src/app/api/revalidate/route.ts
import { streamRevalidatedTags } from "@toapi/server";
import { api } from "@/api";

export const GET = () => {
  return streamRevalidatedTags({
    cache: api.cache,
  });
};
```

For setting up a service worker that connects to this endpoint, see the [`@toapi/worker`](/tapi/worker/) package. For details on how the cache layers interact, see [Caching Strategies](/tapi/server/reference/caching/).
