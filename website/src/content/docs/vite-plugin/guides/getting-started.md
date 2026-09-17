---
title: "Getting Started"
description: "Set up @toapi/vite-plugin, configure your API entry point, and load environment variables during development."
---

## Installation

```bash
pnpm add -D @toapi/vite-plugin
pnpm add @toapi/server srvx
```

Peer dependencies: `vite ^8` and `@toapi/server`.

## Plugin setup

Add the plugin to your Vite config:

```ts
// vite.config.ts
import { defineConfig } from "vite";
import toapi from "@toapi/vite-plugin";

export default defineConfig({
  plugins: [toapi()],
});
```

## API entry point

By default the plugin looks for `src/api.ts` and expects it to export `api`:

```ts
// src/api.ts
import { defineApi, defineHandler, TResponse } from "@toapi/server";

export const api = defineApi().route("/hello", {
  GET: defineHandler({ authorize: () => true }, async () => {
    return TResponse.json({ message: "hello" });
  }),
});
```

In dev (`vite`) and preview (`vite preview`) the API is mounted as middleware at
`/api` on Vite's server. Frontend and API share the same port.

To point at a different entry file or mount the API at another prefix, use the
[`entry` and `basePath` options](/tapi/vite-plugin/reference/options/).

## Environment variables

During development the plugin loads `.env`, `.env.local`, `.env.<mode>`, and
`.env.<mode>.local` via Vite's `loadEnv` and mirrors every key into
`process.env` before the API module is loaded. This covers server-side libraries
(database clients, OAuth secrets, BetterAuth) that read `process.env` directly.

```
.env                     # all envs
.env.local               # all envs, ignored by git
.env.development         # dev only
.env.development.local   # dev only, ignored by git
```

:::note
Existing `process.env` values take precedence, so shell variables override
`.env` files. Vite's `import.meta.env` injection still applies to client code.
:::

In production, env vars must come from the runtime (Docker, systemd, your PaaS).
The built server does not load `.env` files — see
[Deployment](/tapi/vite-plugin/guides/deployment/).
