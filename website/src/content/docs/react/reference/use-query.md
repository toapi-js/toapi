---
title: "useQuery"
description: "Reference for the useQuery hook: read a Toapi client query into a React component with Suspense and automatic updates on revalidation."
---

`useQuery` reads a query from the [`@toapi/client`](/tapi/client/) and returns its resolved value, keeping that value up to date as the underlying cache entry is revalidated. It is the primary hook in [`@toapi/react`](/tapi/react/).

```ts
import { useQuery } from "@toapi/react";
```

## Signature

```ts
function useQuery<T>(
  query: ObservablePromise<T> | (() => ObservablePromise<T>),
  options?: {
    startTransition?: typeof React.startTransition;
  },
): T;

type ObservablePromise<T> = Promise<T> & Observable<T>;
```

### Parameters

- **`query`** — either an `ObservablePromise<T>` or a factory returning one. This is exactly what a client `.get()` produces: a `Promise<T>` that also implements [`Observable<T>`](/tapi/client/reference/observable/) (it has a `.subscribe()` method). Pass the promise directly for a static query, or a factory function when the query depends on props/state — the factory is memoized on its identity, so build it with `useCallback` or inline it carefully to control when a new request is issued.
- **`options.startTransition`** — the `startTransition` function used to apply background updates. Defaults to React's `React.startTransition`. Override it to route updates through a custom transition (for example one wired to `useTransition`'s pending state).

### Return value

`useQuery` returns the resolved value `T` directly — not a promise, and not a `{ data, isLoading }` wrapper.

- On first render it calls React's `use()` on the query, so the component **suspends** until the value resolves. Wrap the component in a [`Suspense`](https://react.dev/reference/react/Suspense) boundary to show a fallback, and in an error boundary to catch rejections (such as an [`HttpError`](/tapi/client/reference/http-error/)).
- On subsequent updates it returns the latest value delivered by the subscription, applied inside `startTransition` so the UI stays responsive during the refresh.

:::note
`useQuery` returns the unwrapped value. If you instead want the raw promise — for example to hand it to a child component and let that child own its own `Suspense` boundary — pass the client query straight to React's `use()` yourself. `useQuery` is the convenience layer that suspends and subscribes in one call.
:::

## What it does

Internally `useQuery` does two things with the object you pass:

1. **Suspends via `use()`.** Until the first value arrives, the hook calls `React.use(query)`, which suspends the component until the promise resolves (or throws to the nearest error boundary).
2. **Subscribes to the client's PubSub.** It calls `query.subscribe(...)` and re-renders whenever the client emits a new value for that entry. The client emits when the query is invalidated through Toapi's [tag-based cache invalidation](/tapi/client/reference/revalidation/), when the server pushes an out-of-band invalidation, or when you manually call a `revalidate` method such as `client.books.revalidate()`. The subscription is torn down automatically on unmount.

This means a component reading `client.books.get()` will re-render with fresh data after any mutation that affects the `books` tag — without you wiring up any additional state.

## Usage

### Basic read with Suspense

```tsx
import { Suspense } from "react";
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

export function Page() {
  return (
    <Suspense fallback={<p>Loading books…</p>}>
      <BookList />
    </Suspense>
  );
}
```

### A query that depends on state

Queries can just use ordinary search parameters:

```tsx
import { useState } from "react";
import { useQuery } from "@toapi/react";
import { client } from "./tapi-client";

function SearchResults({ initial = "" }: { initial?: string }) {
  const [q, setQ] = useState(initial);

  const results = useQuery(client.books.get({ q }));

  return (
    <>
      <input value={q} onChange={(e) => setQ(e.target.value)} />
      <ul>
        {results.map((book) => (
          <li key={book.id}>{book.name}</li>
        ))}
      </ul>
    </>
  );
}
```

### Automatic updates after a mutation

Background revalidation keeps the last resolved value visible while the new response loads, even when a parent re-renders or the user types into the component. Inline query factories are supported without memoization. A different URL, query parameter, or client instance is a different query and suspends for its own data. Use React component keys for navigation identity, not refresh timestamps or response versions.

Because `useQuery` subscribes to the client, a component reading a query updates itself once a mutation invalidates the matching tags — no manual refetch needed. Just make sure `POST /books` invalidates at least one of the tags associated with `GET /books`.

```tsx
import { useQuery } from "@toapi/react";
import { client } from "./tapi-client";

function Books() {
  const books = useQuery(client.books.get());

  async function addBook() {
    // The POST response carries tags that overlap the GET above,
    // so the list re-renders with the new book automatically.
    await client.books.post({ name: "New Book" });
  }

  return (
    <>
      <button onClick={addBook}>Add book</button>
      <ul>
        {books.map((book) => (
          <li key={book.id}>{book.name}</li>
        ))}
      </ul>
    </>
  );
}
```

:::tip
To force a refresh without a mutation — for example on a manual "refresh" button — call the query's `revalidate` method, e.g. `client.books.revalidate()`. Any mounted `useQuery` reading that entry will re-render with the fresh data.
:::

## See also

- [`@toapi/react`](/tapi/react/) — package overview and installation.
- [`@toapi/client`](/tapi/client/) — the client that produces the queries you pass here.
- [`Observable`](/tapi/client/reference/observable/) — the subscription primitive `useQuery` builds on.
- [Revalidation & subscriptions](/tapi/client/reference/revalidation/) — how tag-based invalidation drives updates.
