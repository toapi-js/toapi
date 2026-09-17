---
title: "Link"
description: "The Link component renders an anchor and performs client-side navigation with relative and query-only href resolution."
---

The `Link` component provides declarative navigation between routes. It renders as an anchor element but intercepts clicks to perform client-side navigation.

## Usage

```tsx
import { Link } from "@toapi/router";

function Navigation() {
  return (
    <nav>
      <Link href="/">Home</Link>
      <Link href="/about">About</Link>
      <Link href="/users/123">User Profile</Link>
    </nav>
  );
}
```

## Props

### `href` (required)

- **Type**: `string`
- **Description**: the target URL for navigation. Supports absolute paths, relative paths, and query-only fragments.

```tsx
<Link href="/about">About</Link>          // Absolute path
<Link href="settings">Settings</Link>     // Relative path
<Link href="?search=query">Search</Link>  // Query parameters only
```

### `replace` (optional)

- **Type**: `boolean`
- **Description**: use `history.replaceState()` instead of `history.pushState()` for navigation
- **Default**: `false`

```tsx
<Link href="/login" replace>
  Login
</Link>
```

### `onClick` (optional)

- **Type**: `(event: React.MouseEvent<HTMLAnchorElement>) => void`
- **Description**: custom click handler called before navigation. If it calls `event.preventDefault()`, the router skips its own navigation.

```tsx
<Link
  href="/analytics"
  onClick={(e) => {
    trackEvent("navigation", "analytics-page");
  }}
>
  Analytics
</Link>
```

### `useTransition` (optional)

- **Type**: `boolean | ((scope: () => void) => void)`
- **Description**: controls how the navigation update is scheduled.
  - `undefined` (default): the update is wrapped in React's `startTransition`, so navigation stays interruptible and doesn't block urgent updates.
  - `false`: the update runs synchronously, outside of a transition.
  - a function: called with the update as its `scope` argument, so you can supply your own transition — for example the `startTransition` returned by React's `useTransition()` hook, which also gives you an `isPending` flag.

```tsx
<Link href="/settings" useTransition={false}>
  Settings
</Link>
```

### HTML Anchor Props

All standard HTML anchor element props are supported and passed through:

```tsx
<Link
  href="/profile"
  className="nav-link"
  id="profile-link"
  rel="noopener"
  data-testid="profile"
>
  Profile
</Link>
```

## Href Resolution

The Link component resolves href values based on the current route context.

### Absolute Paths

Absolute paths (starting with `/`) navigate to the exact path regardless of current location:

```tsx
<Link href="/users">Users</Link>            // Always goes to /users
<Link href="/admin/dashboard">Admin</Link>  // Always goes to /admin/dashboard
```

### Relative Paths

Relative paths are resolved based on the current route context:

```tsx
// Current route: /users/123
<Link href="edit">Edit</Link>              // Navigates to /users/123/edit
<Link href="posts">Posts</Link>            // Navigates to /users/123/posts

// In nested routes
<Route path="/users/:id">
  <Route path="profile">
    {/* Current context: /users/123/profile */}
    <Link href="edit">Edit Profile</Link>   // Navigates to /users/123/profile/edit
  </Route>
</Route>
```

### Query Parameters Only

An href starting with `?` appends query parameters to the current pathname:

```tsx
// Current path: /search
<Link href="?q=react">Search React</Link>       // Navigates to /search?q=react
<Link href="?sort=name&order=asc">Sort</Link>   // Navigates to /search?sort=name&order=asc
```

### External URLs

The Link component does not handle external URLs. Use a plain `<a>` tag for external links:

```tsx
<a href="https://farbenmeer.de/en" target="_blank" rel="noopener noreferrer" />
```

## Navigation Behavior

The Link component handles client-side navigation for internal links:

1. Calls the custom `onClick` handler (if provided), together with the navigation, inside a transition (see the `useTransition` prop above).
2. If the handler called `event.preventDefault()`, the router skips its own navigation.
3. Otherwise it uses the router's `push` or `replace` method for navigation.
4. Prevents the default browser anchor navigation.
5. Updates the URL and triggers re-renders.

```tsx
// Push navigation (default) - adds to history stack
<Link href="/page">Go to Page</Link>

// Replace navigation - replaces current history entry
<Link href="/page" replace>Replace Current Page</Link>
```

## Examples

### Dynamic Links

```tsx
function UserCard({ userId, username }) {
  return (
    <div className="user-card">
      <Link href={`/users/${userId}`}>
        <h3>{username}</h3>
      </Link>
      <Link href={`/users/${userId}/posts`}>View Posts</Link>
    </div>
  );
}
```

### Nested Route Navigation

```tsx
<Route path="/dashboard">
  <nav>
    <Link href="">Overview</Link>            {/* /dashboard */}
    <Link href="analytics">Analytics</Link>  {/* /dashboard/analytics */}
    <Link href="settings">Settings</Link>    {/* /dashboard/settings */}
  </nav>

  <Route path="users/:id">
    <nav>
      <Link href="">Profile</Link>                     {/* /dashboard/users/123 */}
      <Link href="edit">Edit</Link>                    {/* /dashboard/users/123/edit */}
      <Link href="/dashboard">Back to Dashboard</Link> {/* Absolute path */}
    </nav>
  </Route>
</Route>
```

### Active Link Styling

```tsx
import { Link, usePathname } from "@toapi/router";

function NavLink({ href, children }) {
  const pathname = usePathname();
  const isActive = pathname === href;

  return (
    <Link href={href} className={isActive ? "nav-link active" : "nav-link"}>
      {children}
    </Link>
  );
}
```

### Navigating Without a Transition

Set `useTransition={false}` when you want the URL and route state to update synchronously:

```tsx
<Link href="/checkout" useTransition={false}>
  Checkout
</Link>
```

### Showing a Loading State with `useTransition`

Pass React's own `startTransition` (from the `useTransition()` hook) as the `useTransition` prop to surface an `isPending` flag while the navigation is in flight:

```tsx
import { useTransition } from "react";
import { Link } from "@toapi/router";

function NavLink({ href, children }) {
  const [isPending, startTransition] = useTransition();

  return (
    <Link href={href} useTransition={startTransition} aria-busy={isPending}>
      {children}
      {isPending && <Spinner />}
    </Link>
  );
}
```

### Using `Link` with `useOptimistic`

`Link` always wraps its click handling in a transition (by default via `startTransition`), so an optimistic update triggered inside `onClick` is compatible with `useOptimistic`:

```tsx
import { useOptimistic } from "react";
import { Link } from "@toapi/router";

function LikeButton({ postId, likes, onLike }) {
  const [optimisticLikes, addOptimisticLike] = useOptimistic(
    likes,
    (state) => state + 1,
  );

  return (
    <Link
      href={`/posts/${postId}`}
      onClick={() => {
        addOptimisticLike(null);
        onLike(postId);
      }}
    >
      {optimisticLikes} likes
    </Link>
  );
}
```

## Related

- [Router](/tapi/router/reference/router/) — root router component
- [Route](/tapi/router/reference/route/) — define route matching
- [useRouter](/tapi/router/reference/use-router/) — programmatic navigation
- [usePathname](/tapi/router/reference/use-pathname/) — access the current pathname
