# @toapi/common

## 1.3.1

## 1.3.0

### Minor Changes

- 36be6c8: Complete rewrite of the client caching system

## 1.2.5

### Patch Changes

- 613d1fa: Give query observables an optional stable queryKey across revalidation promises. React retains the last resolved data during background refreshes, including when inline query factories run again because of parent updates or typing. Switching URLs, query parameters, or client instances still suspends for the new query. Custom observables without a key keep their existing identity behavior.

## 1.2.4

## 1.2.3

## 1.2.2

### Patch Changes

- bc9da1a: fix immortal worker cache entries

## 1.2.1

### Patch Changes

- b4e2f92: Constrain `TResponse.json` to only accept JSON-serializable values (`JSONValue`). This prevents non-JSON types such as `Date`, `Map`, `Set`, `bigint`, `undefined`, functions, or symbols from being passed as structured response data, which would otherwise be silently coerced to strings by `JSON.stringify` and break the type contract the client relies on. Form-data mocks that echoed `Object.fromEntries(formData)` (which can contain `File` values) were updated to return only string entries.

## 1.2.0

## 1.1.1

## 1.1.0

### Minor Changes

- a3a106e: Move the `Observable` type from `@toapi/client` to `@toapi/common`. `@toapi/client` continues to re-export `Observable`, so its public API is unchanged. `@toapi/react` now depends on `@toapi/common` directly and no longer has a peer dependency on `@toapi/client`.

## 1.0.1

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
