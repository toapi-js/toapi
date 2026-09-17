---
title: "useParams"
description: "The useParams hook returns the dynamic route parameters captured from the current matched path, with optional typing."
---

The `useParams` hook provides access to the current route parameters extracted from the URL path. It returns an object containing all captured parameters from dynamic route segments.

## Usage

```tsx
import { useParams } from "@toapi/router";

function UserProfile() {
  const params = useParams();
  const userId = params.id; // From route like /users/:id

  return (
    <div>
      <h1>User Profile</h1>
      <p>User ID: {userId}</p>
    </div>
  );
}
```

## Return Value

- **Type**: `Record<string, string | string[]>`
- **Description**: an object where keys are parameter names (from `:paramName` or `*name` in route paths) and values are the captured URL segments
- **Parameter Values**: strings captured from the URL

## Parameter Extraction

### Single Parameters

```tsx
// Route: /users/:id
// URL: /users/123
function UserDetail() {
  const params = useParams();
  console.log(params); // { id: "123" }

  return <div>User {params.id}</div>;
}
```

### Multiple Parameters

```tsx
// Route: /users/:userId/posts/:postId
// URL: /users/123/posts/456
function PostDetail() {
  const params = useParams();
  console.log(params); // { userId: "123", postId: "456" }

  return (
    <div>
      <p>User: {params.userId}</p>
      <p>Post: {params.postId}</p>
    </div>
  );
}
```

### Wildcard Parameters

Named wildcards (`*name`) capture the entire matched trailing path, including slashes:

```tsx
// Route: /files/*path
// URL: /files/documents/report.pdf
function FileViewer() {
  const params = useParams();
  console.log(params); // { path: "documents/report.pdf" }

  return <div>Viewing: {params.path}</div>;
}
```

## Nested Route Parameters

In nested routes, child routes inherit parameters from their parent routes:

```tsx
<Route path="/users/:id">
  <Route path="posts">
    <Route path=":postId">
      <PostEditor />
    </Route>
  </Route>
</Route>

function PostEditor() {
  const params = useParams();
  // URL: /users/123/posts/456
  // params = { id: "123", postId: "456" }

  return (
    <div>
      <p>Editing post {params.postId} for user {params.id}</p>
    </div>
  );
}
```

## TypeScript

`useParams` accepts a type parameter so you can describe the shape of the `params` object for a given route:

```tsx
interface Params {
  id: string;
  postId: string;
}

function PostDetail() {
  const params = useParams<Params>();
  console.log(params); // { id: "123", postId: "456" }

  return (
    <div>
      <p>User: {params.id}</p>
      <p>Post: {params.postId}</p>
    </div>
  );
}
```

:::note
The type parameter must extend `Record<string, string | string[]>`. It is a compile-time convenience only — the router does not validate that the captured params actually match the type at runtime.
:::

## Related

- [Route](/tapi/router/reference/route/) — define route matching and parameters
- [usePathname](/tapi/router/reference/use-pathname/) — access the current pathname
- [useSearchParams](/tapi/router/reference/use-search-params/) — access search parameters
