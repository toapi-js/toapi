---
title: "@toapi/react"
description: "React bindings for the Toapi client: a useQuery hook that streams live query data into your components via Suspense and PubSub subscriptions."
---

`@toapi/react` connects the [`@toapi/client`](/tapi/client/) fetch client to React. It exposes a single hook, [`useQuery`](/tapi/react/reference/use-query/), that reads a query from the client and keeps the rendered value up to date as the underlying cache entry changes.

The client's `.get()` calls already return a promise that is also an [`Observable`](/tapi/client/reference/observable/): it can be awaited like any promise, and it can be subscribed to for live updates. `useQuery` bridges both halves into React. It hands the promise to React's `use()` so the component suspends until data is available, and it subscribes to the client's internal PubSub so that when the entry is revalidated — through Toapi's [tag-based cache invalidation](/tapi/client/reference/revalidation/) or a manual `client.books.revalidate()` — the component re-renders with the fresh data. No extra query cache, store, or provider is required.

## Installation

```bash
pnpm add @toapi/react
```

`@toapi/react` has two peer dependencies you must have installed in your app:

- [`@toapi/client`](/tapi/client/) — the source of the queries you pass to `useQuery`.
- `react` — version 19 or newer, because `useQuery` relies on the `use()` hook and Suspense.

## Quick start

Create a client once (see [`createFetchClient`](/tapi/client/reference/create-fetch-client/)), then read from it inside a component with `useQuery`.

```tsx
import { useQuery } from "@toapi/react";
import { client } from "./tapi-client";

function BookList() {
  const books = useQuery(client.books.get());

  return (
    <ul>
      {books.map((book) => (
        <li key={book.id}>{book.name}</li>
      ))}
    </ul>
  );
}
```

Because `useQuery` suspends while the query is loading, wrap it in a `Suspense` boundary to render a fallback:

```tsx
import { Suspense } from "react";

function Page() {
  return (
    <Suspense fallback={<p>Loading books…</p>}>
      <BookList />
    </Suspense>
  );
}
```

## How it pairs with @toapi/client

The value you pass to `useQuery` is whatever `client.<path>.get()` returns — a promise that also implements [`Observable`](/tapi/client/reference/observable/). `useQuery` uses both facets:

- **Suspense.** The promise is passed to React's `use()`, so the component suspends until the first value resolves and re-throws on error to the nearest error boundary.
- **Live updates.** `useQuery` calls `.subscribe()` on the same object. When a mutation with overlapping tags runs, when the server pushes an invalidation, or when you call `client.books.revalidate()`, the subscription fires and the component re-renders with the new data inside a `startTransition`.

:::tip
`useQuery` accepts either the query object itself (`client.books.get()`) or a factory function returning one (`() => client.books.get({ q })`). Pass a factory when the query depends on props or state so a new request is built when those inputs change.
:::

See [useQuery](/tapi/react/reference/use-query/) for the full signature and options.

## Public exports

| Export | Kind | Reference |
| --- | --- | --- |
| `useQuery` | hook | [useQuery](/tapi/react/reference/use-query/) |
