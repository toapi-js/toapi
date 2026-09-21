# @toapi/vite-plugin

## 3.0.0

### Patch Changes

- @toapi/server@1.3.0

## 2.0.1

### Patch Changes

- Updated dependencies [04bb55e]
  - @toapi/server@1.2.0

## 2.0.0

### Patch Changes

- @toapi/server@1.1.0

## 1.0.0

### Minor Changes

- 7b2c251: Migrate four more packages to the `@toapi` scope, keeping the original
  `@farbenmeer` names as backward-compatible shims:

  - `@farbenmeer/tag-based-cache` → `@toapi/cache`
  - `@farbenmeer/router` → `@toapi/router`
  - `@farbenmeer/react-tapi` → `@toapi/react`
  - `@farbenmeer/vite-plugin-tapi` → `@toapi/vite-plugin`

  Each original package is now a thin, build-free shim whose entry points
  re-export from the corresponding `@toapi/*` package via hand-authored
  `.js`/`.d.ts` files, so existing consumers need no changes.

- 7b2c251: `@toapi/vite-plugin` now imports and generates code targeting `@toapi/server`
  instead of `@farbenmeer/tapi/server`, and its peer dependency changed from
  `@farbenmeer/tapi` to `@toapi/server`. Consumers (including via the
  `@farbenmeer/vite-plugin-tapi` shim) should depend on `@toapi/server`; the
  server module the plugin emits into the build now imports from `@toapi/server`.

### Patch Changes

- Updated dependencies [b81f517]
  - @toapi/server@1.0.0

## 0.3.0

### Patch Changes

- Updated dependencies [311f7b8]
- Updated dependencies [30b0ac8]
  - @farbenmeer/tapi@0.12.0

## 0.2.1

### Patch Changes

- 8d6788c: adjust server build output path and add vite plugin docs to website
- Updated dependencies [8d6788c]
- Updated dependencies [8d6788c]
  - @farbenmeer/tapi@0.11.0

## 0.2.0

### Minor Changes

- 2d652c2: vite-plugin-tapi: auto-load .env into process.env in dev and preview

  Mirror Vite's `loadEnv` result into `process.env` at the top of both `configureServer` and `configurePreviewServer` so server-side libraries (BetterAuth, DB clients, OAuth secrets) see their secrets without any changes to user code. Shell vars keep precedence over `.env` values. Standalone production servers (i.e. running the built `server.js` directly) are unchanged and remain responsible for their own env loading.

- 8bd884a: Refactor Vite Plugin:

  - production deployments should run srvx directly
  - dev and preview mode work as expected
  - invalidation stream is served directly by TApi's server handler (under api base path)

### Patch Changes

- cfa8d49: vite-plugin-tapi: emit client + server bundles
- Updated dependencies [8bd884a]
  - @farbenmeer/tapi@0.10.7
