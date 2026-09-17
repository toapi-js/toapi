---
title: "@toapi/vite-plugin"
description: "Bundle a Toapi API alongside your Vite frontend: served as middleware in dev/preview, and a deployable server bundle in production."
---

`@toapi/vite-plugin` bundles a [Toapi](/tapi/server/) REST API alongside
your Vite frontend in a single project. In dev and preview modes the API is
served as middleware on the same port as the frontend — no CORS configuration
needed. For production, `vite build` emits a deployable server bundle.

## Installation

```bash
pnpm add -D @toapi/vite-plugin
pnpm add @toapi/server srvx
```

Peer dependencies: `vite ^8` and `@toapi/server`.

## Quick start

Add the plugin to your Vite config:

```ts
// vite.config.ts
import { defineConfig } from "vite";
import toapi from "@toapi/vite-plugin";

export default defineConfig({
  plugins: [toapi()],
});
```

The plugin expects `src/api.ts` to export `api` (an `ApiDefinition`):

```ts
// src/api.ts
import { defineApi, defineHandler, TResponse } from "@toapi/server";

export const api = defineApi().route("/hello", {
  GET: defineHandler({ authorize: () => true }, async () => {
    return TResponse.json({ message: "hello" });
  }),
});
```

In dev (`vite`) and preview (`vite preview`) modes, the plugin attaches a
middleware to Vite's server that handles requests at the configured `basePath`
(default `/api`). The same Vite server serves both the frontend and the API on a
single port.

## Features

- **Single-port dev** — frontend and API are served on the same port in dev and
  preview, so there is no CORS to configure.
- **Isolated build output** — `vite build` writes `dist/client/` (static assets)
  and `dist/server.js` (server bundle) separately, so server secrets never leak
  to the static host.
- **Env file support** — `.env` files are loaded and mirrored into `process.env`
  during development, covering server-side libraries that read `process.env`
  directly.
- **Service-worker ready** — composes cleanly with `vite-plugin-pwa` to add
  offline support and tag-based cache revalidation.

## Guides

- [Getting Started](/tapi/vite-plugin/guides/getting-started/) — set up the plugin
  and configure your API entry point.
- [Service Worker](/tapi/vite-plugin/guides/service-worker/) — add offline support
  with `vite-plugin-pwa`.
- [Deployment](/tapi/vite-plugin/guides/deployment/) — serve the production bundle
  with `srvx`.
- [Reference](/tapi/vite-plugin/reference/options/) — plugin options and build output.
