---
title: "Switch"
description: "The Switch component renders only the first matching Route — ideal for exclusive routing and 404 fallbacks."
---

The `Switch` component groups `Route` components and renders only the **first** one that matches the current URL. This is useful for exclusive routing where you want to ensure only one specific route renders at a time, such as picking between different page layouts or handling "Page Not Found" scenarios.

## Usage

```tsx
import { Switch, Route } from "@toapi/router";

function App() {
  return (
    <Switch>
      <Route path="/" exact>
        <Home />
      </Route>
      <Route path="/about">
        <About />
      </Route>
      <Route path="/users">
        <Users />
      </Route>
    </Switch>
  );
}
```

## Props

### `children` (required)

- **Type**: `ReactElement<RouteProps> | ReactElement<RouteProps>[]`
- **Description**: a single `Route` or an array of `Route` components. The `Switch` component expects its direct children to be `Route` components so it can extract their props (`path`, `exact`) for matching.

## Behavior

### Exclusive Matching

Unlike using multiple `Route` components directly (which render inclusively whenever they match), `Switch` looks through its children in order and stops at the **first** match.

```tsx
// Without Switch: both might render depending on the exact prop
<Route path="/">Home</Route>
<Route path="/about">About</Route>

// With Switch: only one renders
<Switch>
  <Route path="/about">About</Route> {/* Checked first */}
  <Route path="/">Home</Route>       {/* Checked second */}
</Switch>
```

### Route Context

When a match is found, `Switch` renders the matching `Route`'s children inside a `RouteContext`. This ensures that nested components have access to the correct path parameters and matched path information via [`useParams()`](/tapi/router/reference/use-params/).

## Examples

### 404 / Catch-All Route

Because `Switch` stops after the first match, you can place a generic route at the bottom of the list to handle any URLs that didn't match the previous routes.

```tsx
<Switch>
  <Route path="/" exact>
    <Home />
  </Route>
  <Route path="/dashboard">
    <Dashboard />
  </Route>
  {/* Matches everything else */}
  <Route>
    <NotFound />
  </Route>
</Switch>
```

### Route Ordering

Order matters significantly within a `Switch`. More specific paths should generally be placed before less specific ones.

```tsx
<Switch>
  {/* Specific path first */}
  <Route path="/users/new">
    <CreateUser />
  </Route>

  {/* Dynamic path second */}
  <Route path="/users/:id">
    <UserProfile />
  </Route>

  {/* General path last */}
  <Route path="/users">
    <UserList />
  </Route>
</Switch>
```

## Related

- [Route](/tapi/router/reference/route/) — the component used to define routes within a Switch
- [Router](/tapi/router/reference/router/) — root router component
- [Link](/tapi/router/reference/link/) — navigation component
