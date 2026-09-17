---
title: "cleanup"
description: "Reconcile the Toapi worker's cache and metadata stores, dropping long-expired and orphaned entries."
---

`cleanup` reconciles the service worker's Cache Storage and its IndexedDB
metadata stores. It bounds long-term cache growth and heals any drift between the
cache, the meta store, and the tags index. It is intended to be run once at the
top level of the service worker, each time the worker starts up.

## Signature

```ts
function cleanup(options: CleanupOptions): Promise<void>;

interface CleanupOptions {
  /**
   * Grace period in seconds past a cache entry's `expiresAt` before the
   * entry is dropped. Entries expired within this window are kept; entries
   * expired longer than this window are deleted from both the cache and
   * the meta store.
   */
  maximumStaleAge: number;
}
```

- **`maximumStaleAge`** — how many seconds an entry may remain past its
  `expiresAt` before it is dropped. An entry is deleted only once it has been
  expired for longer than this window; recently expired entries are kept so they
  can still serve as an offline fallback.

## Usage

```ts
import { cleanup } from "@toapi/worker";

declare const self: ServiceWorkerGlobalScope;

// Keep entries for up to 7 days past expiry.
cleanup({ maximumStaleAge: 60 * 60 * 24 * 7 }).catch(console.error);
```

`cleanup` returns a promise; nothing awaits it for you, so attach a `.catch(...)`
(or pass a logger-backed handler) to avoid an unhandled rejection.

## Behavior

`cleanup` performs three things:

1. **Drops long-expired entries.** It cursors the meta store and deletes every
   record whose `expiresAt` is older than `maximumStaleAge` seconds, removing the
   matching entries from Cache Storage as well. Records with no expiry, or
   expired within the grace window, are kept.
2. **Drops orphans.** Any entry in Cache Storage that has no surviving meta
   record — for instance, left behind by a write that was interrupted — is
   deleted.
3. **Rebuilds the tags index.** The tags store is rebuilt from the surviving meta
   records, healing any drift between the meta store and the tags index.

:::note
The meta pruning and tags-store rebuild happen inside a single IndexedDB
read-write transaction, so the meta store and the tags index can never disagree
partway through cleanup. Cache Storage deletions run outside that transaction
because the Cache API is async and would otherwise auto-commit it.
:::

## Related

- [`handleToapiRequest`](/tapi/worker/reference/handle-toapi-request/)
- [`listenForInvalidations`](/tapi/worker/reference/listen-for-invalidations/)
- [Service worker setup guide](/tapi/worker/guides/service-worker/)
