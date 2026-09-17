---
title: "Route types (GetRoute & PostRoute)"
description: "GetRoute and PostRoute describe the callable shape of a single client route, useful for typing wrappers and helpers."
---

`@toapi/client` exports two helper types that describe the callable signature of a single route method. You rarely need them when calling the client directly — the full `Client<Routes>` type already infers everything — but they are handy when writing generic wrappers, adapters, or utilities that accept "a GET-style" or "a POST-style" route callable.

## `GetRoute`

```ts
export type GetRoute<R, Q = undefined> = Q extends undefined
  ? (query?: {}, req?: RequestInit) => Promise<R> & Observable<R>
  : (query: Q, req?: RequestInit) => Promise<R> & Observable<R>;
```

Describes a `GET` callable that resolves to `R` and is subscribable ([`Observable<R>`](/tapi/client/reference/observable/)).

- **`R`** — the response type.
- **`Q`** *(optional)* — the query-parameter type. When omitted (`undefined`), the query argument is optional; when provided, it becomes required.

```ts
import type { GetRoute } from "@toapi/client";

// A route returning User[] with no required query
type ListUsers = GetRoute<User[]>;

// A route returning User[] with a required { active: boolean } query
type SearchUsers = GetRoute<User[], { active: boolean }>;
```

## `PostRoute`

```ts
export type PostRoute<R, Q = unknown, B = unknown> = (
  query: Q | FormData,
  body: B | FormData,
  req?: RequestInit,
) => Promise<R>;
```

Describes a mutation callable that resolves to `R`.

- **`R`** — the response type.
- **`Q`** *(optional)* — the query-parameter type.
- **`B`** *(optional)* — the request-body type.

```ts
import type { PostRoute } from "@toapi/client";

type CreateUser = PostRoute<User, undefined, { name: string }>;
```

:::note
These are descriptive helper types. The actual argument order the runtime client uses for mutations is `(body, req?)` with query params carried on `req.query` — see [createFetchClient](/tapi/client/reference/create-fetch-client/#post-body-req--put-body-req--patch-body-req). Reach for `GetRoute`/`PostRoute` when you need a compact type for a route callable rather than as a description of the exact call convention.
:::

## Related

- [createFetchClient](/tapi/client/reference/create-fetch-client/)
- [Observable](/tapi/client/reference/observable/)
