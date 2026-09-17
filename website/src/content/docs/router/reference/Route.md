---
title: "Route"
description: "The Route component conditionally renders content based on the current pathname, with params, wildcards, and nesting."
---

The `Route` component conditionally renders its children based on the current pathname. It supports path parameters, exact matching, and nested routing.

## Usage

```tsx
import { Router, Route } from "@toapi/router";

function App() {
  return (
    <Router>
      <Route path="/users">
        <UsersPage />
      </Route>
      <Route path="/users/:id">
        <UserDetailPage />
      </Route>
    </Router>
  );
}
```

## Props

### `path` (optional)

- **Type**: `string`
- **Description**: the path pattern to match against the current pathname
- **Default**: `""` (matches any path when used as a nested route)

Path patterns support:

- **Static segments**: `/users`, `/about`
- **Parameters**: `/users/:id`, `/posts/:slug`
- **Wildcard paths**: `/files/*`, `/docs/*path`
- **Nested paths**: resolved relative to parent route context

```tsx
// Static path
<Route path="/about">
  <AboutPage />
</Route>

// Path with parameter
<Route path="/users/:id">
  <UserPage />
</Route>

// Nested relative path (resolves to parent path + "settings")
<Route path="/users/:id">
  <Route path="settings">
    <UserSettings />
  </Route>
</Route>
```

### `exact` (optional)

- **Type**: `boolean`
- **Description**: whether the path must match exactly (no additional segments allowed)
- **Default**: `false`

```tsx
// Matches "/users" and "/users/123" and "/users/123/posts"
<Route path="/users">
  <UsersLayout />
</Route>

// Matches only "/users" exactly
<Route exact path="/users">
  <UsersList />
</Route>
```

### `children` (required)

- **Type**: `ReactNode`
- **Description**: the content to render when the route matches

## Path Matching

### Static Paths

By default, a non-exact route matches its path and any deeper path:

```tsx
<Route path="/about">About Page</Route>           // Matches: /about, /about/anything
<Route path="/users/create">Create User</Route>   // Matches: /users/create, /users/create/...
```

### Path Parameters

Parameters are defined with colon syntax and capture a single URL segment:

```tsx
<Route path="/users/:id">
  {/* Access via useParams() hook */}
</Route>

<Route path="/posts/:year/:month/:slug">
  {/* Multiple parameters */}
</Route>

<Route path="/files/*">
  {/* Wildcard - matches any trailing path */}
</Route>

<Route path="/docs/*path">
  {/* Named wildcard - accessible via useParams() */}
</Route>
```

## Nested Routes

Routes can be nested to create hierarchical routing structures:

```tsx
<Route path="/users">
  <UsersLayout />

  <Route path=":id">
    <UserProfile />

    <Route path="settings">
      <UserSettings />
    </Route>

    <Route path="posts">
      <UserPosts />
    </Route>
  </Route>
</Route>
```

### Path Resolution

Nested route paths are resolved relative to their parent:

```tsx
<Route path="/api/v1">              {/* Full path: /api/v1 */}
  <Route path="users">              {/* Full path: /api/v1/users */}
    <Route path=":id">              {/* Full path: /api/v1/users/:id */}
      <Route path="posts">          {/* Full path: /api/v1/users/:id/posts */}
```

### Absolute Paths in Nested Routes

Child routes can use absolute paths (starting with `/`) to ignore parent context:

```tsx
<Route path="/groups/:groupId">
  <Route path="/groups/admin/settings">    {/* Absolute path - matches only if groupId === "admin" */}
    <AdminSettings />
  </Route>
</Route>
```

## Route Context

Each Route component provides context to its children:

- **path**: the full resolved path pattern
- **params**: object containing captured path parameters
- **matchedPathname**: the portion of the pathname that was matched

Access the parameters using the [`useParams()`](/tapi/router/reference/use-params/) hook:

```tsx
<Route path="/users/:id/posts/:postId">
  <PostDetail />
</Route>

function PostDetail() {
  const params = useParams(); // { id: "123", postId: "456" }
  return <div>User {params.id}, Post {params.postId}</div>;
}
```

## Examples

### Parameterized Routes

```tsx
<Router>
  <Route path="/users/:id">
    <UserProfile />
  </Route>
  <Route path="/posts/:slug">
    <BlogPost />
  </Route>
  <Route path="/categories/:category/posts/:id">
    <CategoryPost />
  </Route>
</Router>
```

### Layout Routes

```tsx
<Router>
  <Route path="/dashboard">
    <DashboardLayout>
      <Route exact path="">
        <DashboardHome />
      </Route>
      <Route path="analytics">
        <Analytics />
      </Route>
      <Route path="settings">
        <Settings />
      </Route>
    </DashboardLayout>
  </Route>
</Router>
```

### Route Without Path

Routes without a `path` prop match any path within their parent context:

```tsx
<Route path="/users/:id">
  <UserLayout />

  <Route>
    {/* Always renders when parent matches */}
    <UserNavigation />
  </Route>

  <Route path="profile">
    <UserProfile />
  </Route>
</Route>
```

### Wildcard Routes

Wildcard routes match arbitrary paths including slashes:

```tsx
// Unnamed wildcard - matches but doesn't capture
<Route path="/static/*">
  <StaticFileHandler />
</Route>

// Named wildcard - captures matched path in params
<Route path="/files/*path">
  <FileViewer />
</Route>

function FileViewer() {
  const params = useParams();
  // params.path = "documents/report.pdf" for /files/documents/report.pdf
  return <div>Viewing: {params.path}</div>;
}

// Combining parameters with wildcards
<Route path="/api/:version/*rest">
  <APIProxy />
</Route>

function APIProxy() {
  const params = useParams();
  // For /api/v1/users/123/posts
  // params.version = "v1"
  // params.rest = "users/123/posts"
}
```

:::note
- Wildcards must come at the end of the path.
- `*` matches at least one character (it won't match the exact base path).
- `/files/*` matches `/files/a` but NOT `/files`.
- The matched path includes all slashes and segments.
:::

## Best Practices

1. **Use exact matching** for leaf routes that shouldn't match child paths.
2. **Keep route hierarchies shallow** when possible for better performance.
3. **Use descriptive parameter names**: `:userId` instead of `:id` for clarity.
4. **Group related routes** under common parent routes.
5. **Use layout routes** for shared UI components.

## Related

- [Router](/tapi/router/reference/router/) — root router component
- [Link](/tapi/router/reference/link/) — navigate between routes
- [Switch](/tapi/router/reference/switch/) — render only the first matching route
- [useParams](/tapi/router/reference/use-params/) — access route parameters
- [usePathname](/tapi/router/reference/use-pathname/) — access the current pathname
