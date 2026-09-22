# @toapi/react

## 1.3.2

### Patch Changes

- e550f85: queryKey is required, revert to loading state only on real queryKey change

## 1.3.1

## 1.3.0

### Patch Changes

- Updated dependencies [36be6c8]
  - @toapi/common@1.3.0

## 1.2.5

### Patch Changes

- 613d1fa: Give query observables an optional stable queryKey across revalidation promises. React retains the last resolved data during background refreshes, including when inline query factories run again because of parent updates or typing. Switching URLs, query parameters, or client instances still suspends for the new query. Custom observables without a key keep their existing identity behavior.

## 1.2.4

### Patch Changes

- a2cc715: Relative imports in the published packages now carry the `.js` extension. `@toapi/react`, `@toapi/router` and `@toapi/worker` could not be imported from Node at all — the build emits import specifiers verbatim (`module: "Preserve"`), and with `"type": "module"` Node requires a full specifier, so `import "@toapi/react"` failed with `ERR_MODULE_NOT_FOUND`. This affected any consumer resolving through Node rather than a bundler: SSR, scripts, and tests running in a node environment.

  `@toapi/cache` was unaffected at runtime (its extensionless imports were type-only and erased) but is included for consistency.

  A Biome rule (`useImportExtensions` with `forceJsExtensions`) now guards against a regression, and `pnpm lint` runs it in CI.

## 1.2.3

### Patch Changes

- 4a30d80: `useQuery` no longer renders the previous query's data after the query changes. The stored value is now bound to the observable it was loaded for; when a different observable comes in, the hook suspends on the new one instead of handing out stale data until the subscription catches up. A late update from a subscription that has already been unsubscribed is ignored.

## 1.2.2

## 1.2.1

## 1.2.0

### Patch Changes

- @toapi/common@1.2.0

## 1.1.1

## 1.1.0

### Minor Changes

- a3a106e: Move the `Observable` type from `@toapi/client` to `@toapi/common`. `@toapi/client` continues to re-export `Observable`, so its public API is unchanged. `@toapi/react` now depends on `@toapi/common` directly and no longer has a peer dependency on `@toapi/client`.

### Patch Changes

- Updated dependencies [a3a106e]
  - @toapi/common@1.1.0

## 1.0.1

### Patch Changes

- @toapi/client@1.0.1

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

### Patch Changes

- Updated dependencies [b81f517]
  - @toapi/client@1.0.0

## 8.0.0

### Major Changes

- 7f8b5f0: Remove the `useLacy` hook from `@farbenmeer/react-tapi` (and the re-export from
  `@farbenmeer/bunny`). Use `useQuery` instead.

## 7.0.4

### Patch Changes

- Updated dependencies [311f7b8]
- Updated dependencies [30b0ac8]
  - @farbenmeer/tapi@0.12.0

## 7.0.3

### Patch Changes

- Updated dependencies [8d6788c]
- Updated dependencies [8d6788c]
  - @farbenmeer/tapi@0.11.0

## 7.0.2

### Patch Changes

- Updated dependencies [806bcd8]
- Updated dependencies [36868d8]
- Updated dependencies [b1b4180]
  - @farbenmeer/tapi@0.10.0

## 7.0.1

### Patch Changes

- Updated dependencies [a5228c8]
- Updated dependencies [25886ad]
- Updated dependencies [eb95338]
- Updated dependencies [4786307]
- Updated dependencies [7720a49]
  - @farbenmeer/tapi@0.9.0
  - @farbenmeer/lacy@1.0.0

## 7.0.0

### Minor Changes

- 92afc95: distributed caching and invalidation

### Patch Changes

- Updated dependencies [c79808c]
- Updated dependencies [92afc95]
  - @farbenmeer/tapi@0.8.0

## 6.0.0

### Patch Changes

- a2c3e5e: remove entries from cache that have no active subscription
- a2c3e5e: do not flicker loading state on subscription updates in useQuery
- Updated dependencies [9e51fd3]
- Updated dependencies [29cdbe7]
- Updated dependencies [a2c3e5e]
  - @farbenmeer/tapi@0.7.0

## 5.0.1

### Patch Changes

- af307a9: memoize the full observablePromise instead of just the dataPromise
- Updated dependencies [af307a9]
  - @farbenmeer/tapi@0.6.1

## 5.0.0

### Patch Changes

- Updated dependencies [3183448]
  - @farbenmeer/tapi@0.6.0

## 4.0.0

### Patch Changes

- Updated dependencies [b4f73d8]
- Updated dependencies [03c9103]
  - @farbenmeer/tapi@0.5.0

## 3.0.0

### Patch Changes

- Updated dependencies [7f687af]
- Updated dependencies [7f687af]
  - @farbenmeer/tapi@0.4.0

## 2.0.0

### Patch Changes

- Updated dependencies [980801f]
  - @farbenmeer/tapi@0.3.0

## 1.0.1

### Patch Changes

- 4152e14: Lacy is a direct dependency of react-tapi

## 1.0.0

### Minor Changes

- bb41e24: move to pnpm

### Patch Changes

- Updated dependencies [bb41e24]
  - @farbenmeer/tapi@0.2.0

## 0.1.6

### Patch Changes

- c343771: useQuery returns a lacy promise
- Updated dependencies [8a287b3]
- Updated dependencies [c696188]
  - @farbenmeer/tapi@0.1.10

## 0.1.5

### Patch Changes

- reintroduce cache
- Updated dependencies
  - @farbenmeer/tapi@0.1.7

## 0.1.4

### Patch Changes

- use bun publish

## 0.1.3

### Patch Changes

- 1622886: fix release workflow

## 0.1.2

### Patch Changes

- 155ce29: explicitly set correct tapi dependency version

## 0.1.1

### Patch Changes

- 348d6e9: remove cache implementation and add react-tapi package
- Updated dependencies [348d6e9]
  - @farbenmeer/tapi@0.1.6
