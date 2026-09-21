# @toapi/server

## 1.3.0

### Patch Changes

- Updated dependencies [36be6c8]
  - @toapi/common@1.3.0

## 1.2.5

## 1.2.4

## 1.2.3

## 1.2.2

### Patch Changes

- 1a2e6e8: remove dependencies on deprecated packages
- aef35b3: Flush the revalidation stream's response headers immediately.

  `streamRevalidatedTags` previously wrote nothing until the first keepalive
  fired, so `GET ${basePath}/__tapi/invalidations` left clients waiting up to ten
  seconds before the response resolved. It now sends the first keepalive right
  away (consumers already skip empty lines) and encodes keepalives as bytes
  rather than enqueueing a raw string into the byte stream.

## 1.2.1

### Patch Changes

- b4e2f92: Constrain `TResponse.json` to only accept JSON-serializable values (`JSONValue`). This prevents non-JSON types such as `Date`, `Map`, `Set`, `bigint`, `undefined`, functions, or symbols from being passed as structured response data, which would otherwise be silently coerced to strings by `JSON.stringify` and break the type contract the client relies on. Form-data mocks that echoed `Object.fromEntries(formData)` (which can contain `File` values) were updated to return only string entries.

## 1.2.0

### Minor Changes

- 04bb55e: remove createLocalClient API. It is a uselessly thin wrapper around createRequestHandler + createFetchClient.

### Patch Changes

- @toapi/common@1.2.0

## 1.1.1

## 1.1.0

### Patch Changes

- Updated dependencies [a3a106e]
  - @toapi/common@1.1.0
  - @toapi/client@1.1.0

## 1.0.1

### Patch Changes

- 6279c99: fix import from toapi/client

## 1.0.0

### Minor Changes

- b81f517: Split `@farbenmeer/tapi` into four independently-published packages under the new
  `@toapi` scope:

  - `@toapi/common` — shared code and the route/handler type contract (`Route`,
    `Handler`, `Schema`, `TRequest`, `TResponse`, `CookieStore`, `HttpError`,
    constants, `Logger`, `isMutation`, …)
  - `@toapi/client` — `createFetchClient` and client types
  - `@toapi/server` — `defineApi`, `createRequestHandler`, `createLocalClient`, OpenAPI, …
  - `@toapi/worker` — service-worker request handling

  `@farbenmeer/tapi` is retained as a thin backward-compatible shim: its `./server`,
  `./client`, and `./worker` subpaths now re-export from the corresponding `@toapi/*`
  packages, so existing consumers need no changes.

### Patch Changes

- Updated dependencies [b81f517]
  - @toapi/common@1.0.0
  - @toapi/client@1.0.0
