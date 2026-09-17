---
title: "useSearchParams"
description: "The useSearchParams hook returns an ImmutableSearchParams instance — URLSearchParams whose mutators return a new object."
---

The `useSearchParams` hook provides access to the current URL search parameters (query string). It returns an `ImmutableSearchParams` instance that extends the standard [`URLSearchParams`](https://developer.mozilla.org/en-US/docs/Web/API/URLSearchParams) with immutable mutation methods.

## Usage

It's really just `URLSearchParams`, but the mutating methods return a **new** instance instead of modifying the current one. This makes it safe to derive an updated query string for a [`Link`](/tapi/router/reference/link/) or [`useRouter`](/tapi/router/reference/use-router/) call without accidentally mutating shared state.

To change one search parameter while preserving the rest:

```tsx
import { Link, useSearchParams } from "@toapi/router";

function Example() {
  const searchParams = useSearchParams();

  return <Link href={"?" + searchParams.set("query", "whatever")} />;
}
```

To delete a parameter:

```tsx
const searchParams = useSearchParams();

return <Link href={"?" + searchParams.delete("query")} />;
```

For array parameters (append rather than replace):

```tsx
const searchParams = useSearchParams();

return <Link href={"?" + searchParams.append("options[]", "value")} />;
```

## Return Value

- **Type**: `ImmutableSearchParams` (a subclass of `URLSearchParams`)

All the read methods of `URLSearchParams` work as usual — `get`, `getAll`, `has`, `keys`, `values`, `entries`, `forEach`, `toString`, and `size`.

### Immutable mutators

Unlike `URLSearchParams`, these methods do **not** modify the instance. They each return a new `ImmutableSearchParams` with the change applied:

| Method | Behavior |
|---|---|
| `set(key, value)` | Returns a new instance with `key` set to `value`. |
| `append(name, value)` | Returns a new instance with `value` appended under `name`. |
| `delete(key)` | Returns a new instance with `key` removed. |

Because they return a new instance, calls can be chained:

```tsx
const searchParams = useSearchParams();
const next = searchParams.set("category", "electronics").set("sort", "price");
```

### The `search` getter

`ImmutableSearchParams` adds a `search` getter that returns the query string **including** the leading `?`, or an empty string when there are no parameters:

```tsx
const searchParams = useSearchParams();
searchParams.search; // "?category=electronics&sort=price" — or "" when empty
```

:::tip
`toString()` returns the query string **without** the leading `?`, while the `search` getter includes it. Use whichever fits how you're building the target URL.
:::

## Related

- [useRouter](/tapi/router/reference/use-router/) — programmatic navigation
- [usePathname](/tapi/router/reference/use-pathname/) — access the current pathname
- [Link](/tapi/router/reference/link/) — declarative navigation
