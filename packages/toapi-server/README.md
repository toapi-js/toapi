# @toapi/server

Define fully-typed REST APIs and handle requests on the server — validation, auth, and caching built in. The API definition doubles as the single source of truth for the fully-typed [@toapi/client](https://www.npmjs.com/package/@toapi/client).

📚 [Docs](https://toapi-js.github.io/toapi/server/)

## Installation

```bash
npm install @toapi/server zod
```

## Quick start

```ts
// src/api.ts
import { defineApi, defineHandler, TResponse } from "@toapi/server";
import { z } from "zod";

export const api = defineApi().route("/hello", {
  GET: defineHandler(
    {
      authorize: () => true,
      response: z.object({ message: z.string() }),
    },
    async () => TResponse.json({ message: "Hello, world!" }),
  ),
});
```

```ts
// src/server.ts
import { createRequestHandler } from "@toapi/server";
import { api } from "./api";

export const handler = createRequestHandler(api, { basePath: "/api" });
```
