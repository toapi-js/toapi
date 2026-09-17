---
title: "Router"
description: "The root Router component that provides routing context and manages location state."
---

The `Router` component is the root component that provides routing context to your application. It manages the current location state and provides navigation methods.

## Usage

```tsx
import { Router, Route } from "@toapi/router";

function App() {
  return (
    <Router>
      <Route path="/">
        <h1>Home</h1>
      </Route>
      <Route path="/about">
        <h1>About</h1>
      </Route>
    </Router>
  );
}
```

## Props

### `children` (required)

- **Type**: `ReactNode`
- **Description**: The application components that will have access to the router context.

### `location` (optional)

- **Type**: `{ pathname: string; search: string; hash: string }`
- **Description**: Override the current location instead of using `window.location`. Useful for testing.
- **Default**: uses `window.location`

```tsx
// Example: Testing with custom location
<Router location={{ pathname: "/test", search: "?foo=bar", hash: "#section" }}>
  <App />
</Router>
```

### `history` (optional)

- **Type**: `{ pushState: (state: any, title: string, url: string) => void; replaceState: (state: any, title: string, url: string) => void }`
- **Description**: Override the history implementation instead of using `window.history`. Useful for testing or custom navigation handling.
- **Default**: uses `window.history`

```tsx
// Example: Custom history implementation
const customHistory = {
  pushState: (state, title, url) => {
    console.log(`Navigating to: ${url}`);
    window.history.pushState(state, title, url);
  },
  replaceState: (state, title, url) => {
    console.log(`Replacing with: ${url}`);
    window.history.replaceState(state, title, url);
  },
};

<Router history={customHistory}>
  <App />
</Router>
```

### `useTransition` (optional)

- **Type**: `boolean | ((scope: () => void) => void)`
- **Description**: default `useTransition` behavior for this Router's own state updates (on `popstate`) and for `push`/`replace` calls that don't specify their own `useTransition` option. See [`Link`'s `useTransition` prop](/toapi/router/reference/link/) for the full description of each value.
- **Default**: `undefined` (wraps updates in `startTransition`)

```tsx
// Disable transitions for the whole app
<Router useTransition={false}>
  <App />
</Router>
```

## Context Providers

The Router component provides several React contexts that can be accessed by child components:

- **PathnameContext**: current pathname (e.g. `/users/123`)
- **SearchParamsContext**: current search parameters as an `ImmutableSearchParams` instance
- **HashContext**: current hash fragment (e.g. `#section`)
- **RouterContext**: navigation methods (`push`, `replace`)
- **RouteContext**: initial route context with path `/` and empty params

The Router listens for the browser's `popstate` event (back/forward navigation) and updates its internal state accordingly. State updates are wrapped in `startTransition` so navigation stays responsive.

## Pathname Handling

The Router automatically removes trailing slashes from pathnames to normalize URLs:

- `/users/` becomes `/users`
- `/` remains `/` (root path)

## Navigation Updates

When using the default `window.location` and `window.history`:

- Navigation methods (`push`, `replace`) update the URL and internal state
- State changes trigger re-renders of components using router hooks

## Examples

### Basic Setup

```tsx
import { Router, Route, Link } from "@toapi/router";

function App() {
  return (
    <Router>
      <nav>
        <Link href="/">Home</Link>
        <Link href="/about">About</Link>
        <Link href="/users">Users</Link>
      </nav>

      <main>
        <Route path="/">
          <HomePage />
        </Route>
        <Route path="/about">
          <AboutPage />
        </Route>
        <Route path="/users">
          <UsersPage />
        </Route>
      </main>
    </Router>
  );
}
```

:::caution
`@toapi/router` is a client-side router only. It is not designed for server-side rendering — render it on the client.
:::

## Best Practices

1. **Single Router**: typically, use one Router component at the root of your application.
2. **Testing**: use the `location` prop for predictable test environments.
3. **Custom History**: use a custom history for analytics, logging, or special navigation requirements.

## Related

- [Route](/toapi/router/reference/route/) — define route matching and rendering
- [Link](/toapi/router/reference/link/) — navigate between routes
- [useRouter](/toapi/router/reference/use-router/) — access navigation methods
- [usePathname](/toapi/router/reference/use-pathname/) — access the current pathname
