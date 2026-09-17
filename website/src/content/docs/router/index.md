---
title: "@toapi/router"
description: "A lightweight React router with nested routes, path parameters, and immutable search params."
---

`@toapi/router` is a minimal client-side React router. It covers the most common routing needs without any magic — just components, hooks, and TypeScript.

## Installation

```bash
npm install @toapi/router
```

`@toapi/router` requires React 19 (it uses the `use()` hook and the new context-as-provider syntax). React and TypeScript are peer dependencies.

## Features

- **Declarative routing** — define routes as React components. No config files, no filesystem conventions.
- **Nested routes** — compose routes hierarchically with automatic parameter inheritance from parent segments.
- **Path parameters** — dynamic segments with colon syntax (`:id`), plus wildcard routes (`*` and `*name`).
- **Immutable search params** — update URL search parameters without mutation; each `.set()` returns a new params object.
- **Testing-friendly** — inject a custom `location` and `history` into `<Router>` for fully deterministic tests.
- **Tiny** — no dependencies beyond React, and no runtime overhead from a complex matching engine.

## Quick Start

```tsx
import { Router, Route, Link } from "@toapi/router";

function App() {
  return (
    <Router>
      <nav>
        <Link href="/">Home</Link>
        <Link href="/users">Users</Link>
      </nav>

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
    </Router>
  );
}
```

## Components

| Component | Description |
|---|---|
| [`Router`](/tapi/router/reference/router/) | Root provider component that manages location state and navigation. |
| [`Route`](/tapi/router/reference/route/) | Conditionally renders content based on the current pathname. |
| [`Link`](/tapi/router/reference/link/) | Client-side navigation rendered as an anchor element. |
| [`Switch`](/tapi/router/reference/switch/) | Renders only the first matching `Route` — useful for fallbacks and 404s. |

## Hooks

| Hook | Description |
|---|---|
| [`useRouter()`](/tapi/router/reference/use-router/) | Programmatic navigation via `.push()` and `.replace()`. |
| [`usePathname()`](/tapi/router/reference/use-pathname/) | Current pathname string. |
| [`useParams()`](/tapi/router/reference/use-params/) | Dynamic route parameters from the current segment. |
| [`useSearchParams()`](/tapi/router/reference/use-search-params/) | Immutable search parameter access and updates. |
| [`useHash()`](/tapi/router/reference/use-hash/) | Current URL hash fragment. |

## Guides

- [Setup](/tapi/router/guides/setup/) — install the package and set up your first routes.
- [Testing](/tapi/router/guides/testing/) — test components and routes deterministically.
