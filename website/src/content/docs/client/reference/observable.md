---
title: "Observable"
description: "The Observable type augments GET promises with a subscribe() method for receiving cache updates over time."
---

`Observable<T>` is the type that augments the promise returned by a client `.get()` call. On top of being an awaitable promise, an observable lets you **subscribe** to future values for the same cache entry, so long-lived views can react when the underlying data changes.

## Type

```ts
export type Observable<T> = {
  readonly queryKey?: string;
  subscribe(callback: (value: Promise<T>) => void): () => void;
};
```

A `.get()` therefore returns `Promise<T> & Observable<T>`:

```ts
const result = client.users.get();

// use it as a promise
const users = await result;

// or subscribe to updates
const unsubscribe = result.subscribe((next) => {
  next.then((users) => render(users));
});
```

## `queryKey`

The fetch client supplies the request URL as `queryKey`, identifying the cached query. Compare it by value; different URLs (including query parameters) have different keys.

`useQuery` uses this identity to keep resolved data visible during background refreshes. It is optional for custom observables: without it, the observable promise itself identifies the query.

## `subscribe(callback)`

- **`callback`** — invoked with a `Promise<T>` each time the cache entry for this URL is invalidated and re-fetched. The callback receives a promise (not a resolved value) because the fresh data may still be loading; awaiting it also lets you observe errors.
- **Returns** — an unsubscribe function. Call it to stop receiving updates.

Subscribing does **not** push the current value immediately — you already have it from the `.get()` promise you subscribed on. The callback only fires later, when something invalidates the URL: a tag-matching mutation, an explicit `.revalidate()` call, or a scheduled TTL-based revalidation.

```ts
const result = client.todos.get();

const unsubscribe = result.subscribe((next) => {
  next
    .then((todos) => setTodos(todos))
    .catch((err) => console.error(err));
});

// later, when the view unmounts:
unsubscribe();
```

Unlike earlier versions of this client, subscribing does not keep the cache entry alive by itself — entries are only removed by an error, an explicit invalidation, or TTL expiry, regardless of whether anyone is subscribed.

## Relationship to `@toapi/react`

[`@toapi/react`](/toapi/react/) wraps this `subscribe`/unsubscribe protocol in React hooks so components re-render automatically on cache changes. `Observable` is the underlying primitive; use it directly when integrating with other frameworks or state stores.

## Related

- [createFetchClient](/toapi/client/reference/create-fetch-client/)
- [Revalidation & subscriptions](/toapi/client/reference/revalidation/)
