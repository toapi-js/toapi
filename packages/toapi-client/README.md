# @toapi/client

The typed fetch client for Toapi: a proxy that mirrors your [@toapi/server](https://www.npmjs.com/package/@toapi/server) API, with deduplication, tag-based revalidation, and subscriptions.

📚 [Docs](https://toapi-js.github.io/toapi/client/)

## Installation

```bash
npm install @toapi/client
```

## Quick start

```ts
import { createFetchClient } from "@toapi/client";
import type { api } from "./api"; // the type of your server's defineApi(...)

const client = createFetchClient<typeof api.routes>("https://example.com/api");

const users = await client.users.get();
await client.users.post({ name: "Alice" });
const user = await client.users["123"].get();
```
