# @toapi/router Documentation

A lightweight, React-based client-side router with support for nested routes, path parameters, and immutable search parameter handling.

📚 [Docs](https://toapi-js.github.io/toapi/router/)

## Overview

The `@toapi/router` package provides a very lightweight routing solution for React applications with the following key features:

- **Declarative routing** with React components
- **Nested routes** with parameter inheritance
- **Path parameters** with colon syntax (`:id`)
- **Wildcard routes** for catch-all paths (`*` and `*name`)
- **Immutable search parameters** for predictable state management
- **Client-side navigation** with history management
- **TypeScript support** for better development experience
- **Testing-friendly** with customizable location and history

## Quick Start

```tsx
import { Router, Route, Link } from "@toapi/router";

function App() {
  return (
    <Router>
      <nav>
        <Link href="/">Home</Link>
        <Link href="/users">Users</Link>
        <Link href="/about">About</Link>
      </nav>

      <main>
        <Route path="/">
          <HomePage />
        </Route>

        <Route path="/users">
          <UsersLayout />
          <Route exact>
            <UsersList />
          </Route>
          <Route path=":id">
            <UserProfile />
          </Route>
        </Route>

        <Route path="/about">
          <AboutPage />
        </Route>
      </main>
    </Router>
  );
}
```

## Components

### [Router](./docs/Router.md)
The root component that provides routing context to your application.

- Manages current location state
- Provides navigation methods
- Supports custom location and history for testing
- Normalizes pathnames (removes trailing slashes)

### [Route](./docs/Route.md)
Conditionally renders content based on the current pathname.

- Path matching with parameters (`/users/:id`)
- Wildcard matching (`/files/*` or `/files/*path`)
- Exact matching option
- Nested route support
- Parameter inheritance from parent routes

### [Link](./docs/Link.md)
Declarative navigation component that renders as an anchor element.

- Client-side navigation with history management
- Supports absolute and relative paths
- Query parameter handling

## Hooks

### [useRouter](./docs/useRouter.md)
Provides programmatic navigation methods.

```tsx
const router = useRouter();
router.push("/users/123");     // Navigate with history
router.replace("/login");      // Replace current entry
```

### [usePathname](./docs/usePathname.md)
Access the current pathname for conditional rendering and active states.

```tsx
const pathname = usePathname();
const isActive = pathname === "/users";
```

### [useSearchParams](./docs/useSearchParams.md)
Access and manipulate URL search parameters with immutable methods.

```tsx
const searchParams = useSearchParams();
const query = searchParams.get("q");
const newParams = searchParams.set("filter", "active");
```

### [useParams](./docs/useParams.md)
Extract parameters from dynamic route segments.

```tsx
// Route: /users/:id/posts/:postId
const params = useParams(); // { id: "123", postId: "456" }
```

### [useHash](./docs/useHash.md)
Access the current URL hash fragment for tab navigation and anchor linking.

```tsx
const hash = useHash();
const activeTab = hash.slice(1) || "overview";
```

## Key Features

### Nested Routing

Create hierarchical route structures with parameter inheritance:

```tsx
<Route path="/organizations/:orgId">
  <OrganizationLayout />

  <Route path="teams/:teamId">
    <TeamLayout />

    <Route path="members/:memberId">
      <MemberProfile />
    </Route>
  </Route>
</Route>
```

### Path Parameters

Define dynamic segments with colon syntax:

```tsx
<Route path="/users/:id">          {/* /users/123 */}
<Route path="/posts/:slug">        {/* /posts/hello-world */}
<Route path="/api/:version">       {/* /api/v1 */}
```

### Wildcard Routes

Match arbitrary paths with wildcards:

```tsx
<Route path="/files/*">            {/* Matches /files/a, /files/a/b/c, etc. */}
<Route path="/docs/*path">         {/* Matches and captures as params.path */}
<Route path="/api/:version/*rest"> {/* Combines params with wildcards */}
```

The `*` wildcard matches everything including slashes, making it perfect for catch-all routes. Use `*name` to capture the matched path as a parameter accessible via `useParams()`.

### Immutable Search Parameters

Safely update URL search parameters without mutations:

```tsx
const searchParams = useSearchParams();
const withFilter = searchParams.set("category", "electronics");
const withSort = withFilter.set("sort", "price");
```

### Testing Support

Provide custom location and history for predictable tests:

```tsx
<Router
  location={{ pathname: "/users/123", search: "?tab=profile", hash: "#bio" }}
  history={mockHistory}
>
  <App />
</Router>
```

## Common Patterns

### Active Navigation Links

```tsx
function NavLink({ href, children }) {
  const pathname = usePathname();
  const isActive = pathname === href;

  return (
    <Link
      href={href}
      className={isActive ? 'nav-link active' : 'nav-link'}
    >
      {children}
    </Link>
  );
}
```

### Search and Filtering

```tsx
function ProductSearch() {
  const router = useRouter();
  const pathname = usePathname();
  const searchParams = useSearchParams();

  const updateFilter = (key: string, value: string) => {
    const newParams = searchParams.set(key, value);
    router.push(`${pathname}?${newParams.toString()}`);
  };

  return (
    <select onChange={(e) => updateFilter("category", e.target.value)}>
      <option value="">All Categories</option>
      <option value="electronics">Electronics</option>
    </select>
  );
}
```


## API Reference

For detailed API documentation, see the individual component and hook documentation files linked above.
