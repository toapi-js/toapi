# Toapi Monorepo

A collection of TypeScript libraries for building type-safe REST APIs and web applications.

## Packages

### [@toapi/server](./packages/toapi-server)
Define REST APIs with fully typed routes, tag-based caching and OpenAPI generation — without a compile step.

### [@toapi/client](./packages/toapi-client)
Fully typed fetch client for a Toapi API, with client-side caching and revalidation.

### [@toapi/common](./packages/toapi-common)
Types and helpers shared between the Toapi server, client and worker packages.

### [@toapi/react](./packages/toapi-react)
React bindings for Toapi with Suspense integration and auto-revalidation.

### [@toapi/router](./packages/toapi-router)
Lightweight React-based client-side router with nested routes and path parameters.

### [@toapi/cache](./packages/toapi-cache)
Flexible caching system with tag-based invalidation supporting in-memory, filesystem, Postgres and Redis backends.

### [@toapi/vite-plugin](./packages/toapi-vite-plugin)
Vite plugin that serves a Toapi API in dev and preview, and bundles it for production.

### [@toapi/worker](./packages/toapi-worker)
Service worker runtime that caches Toapi responses and keeps them in sync with the server.

### [@farbenmeer/prisma-migrate-test](./packages/prisma-migrate-test)
Run Prisma migrations against throwaway SQLite/PGlite databases in tests.

## Rust

### [rust/](./rust)
Toapi for Rust: define the API on an axum server, generate OpenAPI from it, generate a typed client (`api.users.id(1).get().await`), and keep caches fresh with tag-based invalidation over the same wire protocol.

## Examples

### [env-file](./examples/env-file)
Example of using environment variables from a .env-file with Vite and Toapi.

### [todo-list](./examples/todo-list)
Example of a simple todo list application using Vite and Toapi.

### [contact-book](./examples/contact-book)
Example of a contact book application using Vite, Toapi and `@toapi/router`.

### [tables](./examples/tables)
Example of rendering server data with `@toapi/react`.

### [vite-plugin-tapi-demo](./examples/vite-plugin-tapi-demo)
Minimal `@toapi/vite-plugin` setup, including a Docker deployment.
