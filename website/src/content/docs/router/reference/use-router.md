---
title: "useRouter"
description: "The useRouter hook returns push and replace methods for programmatic, context-aware client-side navigation."
---

The `useRouter` hook provides access to navigation methods for programmatic routing. It returns an object with `push` and `replace` methods to navigate between routes.

## Usage

```tsx
import { useRouter } from "@toapi/router";

function LoginForm() {
  const router = useRouter();

  const handleLogin = async (credentials) => {
    try {
      await login(credentials);
      router.push("/dashboard");
    } catch (error) {
      console.error("Login failed:", error);
    }
  };

  return (
    <form onSubmit={handleLogin}>
      {/* form fields */}
    </form>
  );
}
```

## Return Value

The hook returns an object with the following methods:

### `push(url: string, options?: { useTransition?: boolean | ((scope: () => void) => void) })`

- **Description**: navigate to a new route by adding a new entry to the browser's history stack
- **Parameters**:
  - `url` (string): the destination URL (absolute path, relative path, or full URL with query parameters)
  - `options.useTransition` (optional): controls how the resulting state update is scheduled.
    - `undefined` (default): wrapped in React's `startTransition`.
    - `false`: applied synchronously, outside of a transition.
    - a function: called with the update, so you can supply your own transition (e.g. the `startTransition` from React's `useTransition()` hook).
- **Returns**: `void`

```tsx
const router = useRouter();

// Navigate to absolute path
router.push("/users");

// Navigate with parameters
router.push("/users/123");

// Navigate with query parameters
router.push("/search?q=react");

// Navigate with hash
router.push("/docs#installation");

// Navigate with everything
router.push("/products?category=electronics&sort=price#top");

// Navigate synchronously, without a transition
router.push("/checkout", { useTransition: false });
```

### `replace(url: string, options?: { useTransition?: boolean | ((scope: () => void) => void) })`

- **Description**: navigate to a new route by replacing the current entry in the browser's history stack
- **Parameters**:
  - `url` (string): the destination URL (absolute path, relative path, or full URL with query parameters)
  - `options.useTransition` (optional): same as `push`'s `options.useTransition`.
- **Returns**: `void`

```tsx
const router = useRouter();

// Replace current history entry
router.replace("/login");

// Useful for redirects where you don't want users to go back
router.replace("/dashboard");
```

## Context-Aware Resolution

Like [`Link`](/tapi/router/reference/link/), `useRouter` resolves the URL you pass relative to the current route context. Absolute paths (starting with `/`) navigate exactly; relative paths resolve against the matched parent route; and query-only (`?…`) or hash-only (`#…`) hrefs are appended to the current location.

```tsx
// Inside a route matched at /users/123
const router = useRouter();
router.push("edit");     // -> /users/123/edit
router.push("?tab=bio"); // -> /users/123?tab=bio
router.push("/");        // -> /
```

## Navigation Methods Comparison

| Method | History Stack | Use Case |
|--------|---------------|----------|
| `push` | Adds new entry | Normal navigation, allows the back button |
| `replace` | Replaces current entry | Redirects, login flows, error corrections |

## Examples

### Navigating Without a Transition

Pass `useTransition: false` when a caller needs the pathname/search state to update synchronously, right after `push`/`replace` returns:

```tsx
function CheckoutButton() {
  const router = useRouter();

  return (
    <button
      onClick={() => {
        router.push("/checkout", { useTransition: false });
      }}
    >
      Checkout
    </button>
  );
}
```

### Showing a Loading State with `useTransition`

Pass React's `startTransition` (from the `useTransition()` hook) as `options.useTransition` to get an `isPending` flag for the duration of the navigation:

```tsx
import { useTransition } from "react";
import { useRouter } from "@toapi/router";

function DashboardLink() {
  const router = useRouter();
  const [isPending, startTransition] = useTransition();

  return (
    <button
      disabled={isPending}
      onClick={() => {
        router.push("/dashboard", { useTransition: startTransition });
      }}
    >
      {isPending ? "Loading…" : "Go to Dashboard"}
    </button>
  );
}
```

### Combining with `useOptimistic`

`useOptimistic` updates must happen inside a transition. Wrap the whole handler — the optimistic update and the navigation — in `startTransition` yourself, and pass `useTransition: false` to `push`/`replace` so it doesn't start a second, nested transition:

```tsx
import { startTransition, useOptimistic } from "react";
import { useRouter } from "@toapi/router";

function ArchiveButton({ itemId, onArchive }) {
  const router = useRouter();
  const [isArchived, setOptimisticArchived] = useOptimistic(false);

  return (
    <button
      onClick={() => {
        startTransition(() => {
          setOptimisticArchived(true);
          onArchive(itemId);
          router.push("/items", { useTransition: false });
        });
      }}
    >
      {isArchived ? "Archiving…" : "Archive"}
    </button>
  );
}
```

## Related

- [Link](/tapi/router/reference/link/) — declarative navigation as an anchor element
- [usePathname](/tapi/router/reference/use-pathname/) — access the current pathname
- [useParams](/tapi/router/reference/use-params/) — access route parameters
- [useSearchParams](/tapi/router/reference/use-search-params/) — access search parameters
