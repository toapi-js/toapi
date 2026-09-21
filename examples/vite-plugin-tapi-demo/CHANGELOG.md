# @farbenmeer/vite-plugin-tapi-example-demo

## 0.1.7

### Patch Changes

- Updated dependencies [36be6c8]
  - @toapi/client@1.3.0
  - @toapi/server@1.3.0
  - @toapi/vite-plugin@3.0.0

## 0.1.6

### Patch Changes

- Updated dependencies [47b4b3e]
- Updated dependencies [04bb55e]
- Updated dependencies [2c8e389]
- Updated dependencies [2c8e389]
  - @toapi/client@1.2.0
  - @toapi/server@1.2.0
  - @toapi/vite-plugin@3.0.0

## 0.1.5

### Patch Changes

- Updated dependencies [a3a106e]
  - @toapi/client@1.1.0
  - @toapi/server@1.1.0
  - @toapi/vite-plugin@2.0.0

## 0.1.4

### Patch Changes

- 29bedf3: vite-plugin-tapi-demo: the reference Caddyfile now sets cache headers for the client build

  Content-hashed assets under `/assets/*` are served with
  `Cache-Control: public, max-age=31536000, immutable`, and the SPA shell (`/`,
  `index.html`) with `no-cache`; `file_server` already emits `ETag`/`Last-Modified`
  and answers `304`. A comment documents dropping `encode` when running behind a
  compressing edge to avoid double compression, and the docker e2e test asserts
  both cache headers.

  Docs: the `srvx -s` static-serving note now calls out that it has no SPA history
  fallback and sets no cache headers, pointing client-routed SPAs at a static
  server / CDN (the reference Caddyfile) instead.

- Updated dependencies [7b2c251]
- Updated dependencies [b81f517]
- Updated dependencies [7b2c251]
  - @toapi/vite-plugin@1.0.0
  - @toapi/client@1.0.0
  - @toapi/server@1.0.0

## 0.1.3

### Patch Changes

- ab4ecbc: Add a production Dockerfile that serves the api with the srvx CLI behind Caddy
  (reverse proxy + static files with an index.html not-found fallback), plus a
  Docker e2e test. Mount the demo api under `/api` instead of the root.

## 0.1.2

### Patch Changes

- Updated dependencies [311f7b8]
- Updated dependencies [30b0ac8]
  - @farbenmeer/tapi@0.12.0
  - @farbenmeer/vite-plugin-tapi@1.0.0

## 0.1.1

### Patch Changes

- Updated dependencies [8d6788c]
- Updated dependencies [8d6788c]
  - @farbenmeer/vite-plugin-tapi@0.2.1
  - @farbenmeer/tapi@0.11.0

## 0.1.0

### Minor Changes

- 8bd884a: Refactor Vite Plugin:

  - production deployments should run srvx directly
  - dev and preview mode work as expected
  - invalidation stream is served directly by TApi's server handler (under api base path)

### Patch Changes

- 2d652c2: vite-plugin-tapi: auto-load .env into process.env in dev and preview

  Mirror Vite's `loadEnv` result into `process.env` at the top of both `configureServer` and `configurePreviewServer` so server-side libraries (BetterAuth, DB clients, OAuth secrets) see their secrets without any changes to user code. Shell vars keep precedence over `.env` values. Standalone production servers (i.e. running the built `server.js` directly) are unchanged and remain responsible for their own env loading.

- Updated dependencies [2d652c2]
- Updated dependencies [8bd884a]
- Updated dependencies [cfa8d49]
  - @farbenmeer/vite-plugin-tapi@0.2.0
