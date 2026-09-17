---
title: "Setup"
description: "Get started with @toapi/router by installing the package and setting up your first routes."
---

Getting started with `@toapi/router` is straightforward. This guide walks you through installing the package and setting up your first routes.

## Installation

Install the package using your preferred package manager:

```bash
# npm
npm install @toapi/router

# pnpm
pnpm add @toapi/router

# yarn
yarn add @toapi/router
```

## Basic Setup

To start using the router, wrap your application (or the part of it that needs routing) with the `Router` component. This component provides the necessary context for all other routing components and hooks.

It's typically best to place the `Router` as high up in your component tree as possible, usually in `App.tsx` or `main.tsx`.

```tsx
// src/App.tsx
import { Router } from "@toapi/router";
import { Navigation } from "./Navigation";
import { Routes } from "./Routes";

function App() {
  return (
    <Router>
      <div className="app">
        <Navigation />
        <main>
          <Routes />
        </main>
      </div>
    </Router>
  );
}

export default App;
```

## Defining Routes

Once the `Router` is in place, define routes using the `Route` component. Routes render their children when the current URL matches their `path`.

Adding a `Switch` at the top level of your routes makes it possible to render a fallback component for unmatched paths.

```tsx
// src/Routes.tsx
import { Route, Switch } from "@toapi/router";

export function Routes() {
  return (
    <Switch>
      {/* Matches the root path */}
      <Route path="/" exact>
        <h1>Welcome Home</h1>
      </Route>

      {/* Matches /about */}
      <Route path="/about">
        <h1>About Us</h1>
      </Route>

      {/* Matches /users and any sub-paths like /users/123 */}
      <Route path="/users">
        <h1>Users</h1>
      </Route>

      {/* Fallback route for unmatched paths */}
      <Route>
        <h1>Not Found</h1>
      </Route>
    </Switch>
  );
}
```

## Adding Navigation

To navigate between routes without reloading the page, use the `Link` component. It renders a standard `<a>` tag but handles the navigation client-side, updating the browser's history and URL.

```tsx
// src/Navigation.tsx
import { Link } from "@toapi/router";

export function Navigation() {
  return (
    <nav>
      <Link href="/">Home</Link>
      <Link href="/about">About</Link>
      <Link href="/users">Users</Link>
    </nav>
  );
}
```

## TypeScript Support

The router is written in TypeScript and ships with type definitions included. No additional `@types` packages are needed.

## Next Steps

Now that you have the basics set up, you can explore more advanced features:

- [Nested Routes & Parameters](/tapi/router/reference/route/) — build hierarchical layouts and use dynamic path parameters.
- [Programmatic Navigation](/tapi/router/reference/use-router/) — navigate using hooks instead of Links.
- [Search Parameters](/tapi/router/reference/use-search-params/) — manage URL query strings immutably.
