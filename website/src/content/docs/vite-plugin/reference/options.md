---
title: "Reference"
description: "Plugin options for @toapi/vite-plugin (entry, basePath, port, external) and the isolated dist/client + dist/server.js build output."
---

The default export is a function that returns a Vite `Plugin`. It accepts an
optional options object:

```ts
// vite.config.ts
import { defineConfig } from "vite";
import toapi from "@toapi/vite-plugin";

export default defineConfig({
  plugins: [
    toapi({
      entry: "src/api.ts",
      basePath: "/api",
      // port: 3000,
      // external: [],
    }),
  ],
});
```

## Options

| Option | Type | Default | Description |
|---|---|---|---|
| `entry` | `string` | `"src/api.ts"` | Path to the file exporting `api`. Resolved against the Vite root. |
| `basePath` | `string` | `"/api"` | Prefix for API routes. Use `""` to mount at the root. |
| `port` | `number` | — | Default port for Vite's dev/preview server. Falls back to the `PORT` env var. |
| `external` | `(string \| RegExp)[]` | `[]` | Packages to keep external in the server bundle. By default everything is bundled. |

## Build output

`vite build` writes two clearly separated trees:

```
dist/
├── client/        — static frontend assets (HTML, JS, CSS, images)
├── server.js      — bundled server (sourcemap included)
└── server.js.map
```

The split exists for safety: server-only code (database credentials,
third-party API keys, server-side libraries) lives in `dist/server.js` and
**must not** be deployed to a public static host. Treating `dist/client/` as the
static-deploy root makes it impossible to leak the server bundle by accident.

By default the entire server dependency tree is bundled into `dist/server.js`.
Use the `external` option to keep specific packages out of the bundle — for
example, native modules that cannot be bundled — and provide them from
`node_modules` at runtime instead.

For how to run the server bundle in production, see
[Deployment](/tapi/vite-plugin/guides/deployment/).
