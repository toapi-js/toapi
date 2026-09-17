---
title: "Testing"
description: "Learn how to test components and routes with @toapi/router."
---

Testing components that use `@toapi/router` is straightforward because the `Router` component is designed to be decoupled from the browser environment. You can explicitly provide `location` and `history` objects via props, allowing you to simulate any route state and capture navigation events without a real browser window.

## Rendering a Specific Route

To render your application at a specific route during a test, pass a custom `location` object to the `Router` component. This object should mimic the `window.location` interface, primarily needing `pathname`, `search`, and `hash`.

If you don't provide a `location`, the router defaults to using `window.location`, which might not be what you want in a test environment (usually defaults to `/`).

```tsx
import { test, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { Router, Route } from "@toapi/router";

test("renders user profile", () => {
  render(
    <Router
      location={{
        pathname: "/users/123",
        search: "",
        hash: "",
      }}
    >
      <Route path="/users/:id">
        <UserProfile />
      </Route>
    </Router>
  );

  expect(screen.getByText("User 123")).toBeInTheDocument();
});
```

## Mocking Navigation

To test interactions that cause navigation (like clicking a link or submitting a form), provide a mock `history` object. The router uses `history.pushState` and `history.replaceState` to change routes.

You can create a helper to generate a mock history object. This helper should maintain the current URL state so that when the router asks for it, it gets the updated value.

Here is an example of how to implement a mock history helper using `vitest` (or `jest`):

```ts
// test-utils.ts
import { vi } from "vitest"; // or jest

export function createMockHistory(initialPath = "/") {
  // We use a real URL object to handle relative paths and query params logic automatically
  const location = new URL(initialPath, "http://localhost:3000");

  const history = {
    pushState: vi.fn((_state, _title, url) => {
      // Update our location object when navigation happens
      location.href = new URL(url, location.href).href;
    }),
    replaceState: vi.fn((_state, _title, url) => {
      location.href = new URL(url, location.href).href;
    }),
  };

  return { location, history };
}
```

:::tip
`@toapi/router`'s own test suite ships an equivalent `mockHistory` helper (in `src/mock-history.ts`) that additionally tracks a history stack and exposes a `back()` method that dispatches a `popstate` event. It's a good reference if you need to simulate back-navigation in your own tests.
:::

### Usage in Tests

Now you can use this helper to inject both the `location` and `history` into your Router. This allows the Router to read the initial state and write updates back to your mock.

```tsx
import { test, expect } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { Router, Link } from "@toapi/router";
import { createMockHistory } from "./test-utils";

test("navigates to about page on click", () => {
  // 1. Create the mock history starting at home
  const { location, history } = createMockHistory("/");

  render(
    <Router location={location} history={history}>
      <nav>
        <Link href="/about">About Us</Link>
      </nav>
    </Router>
  );

  // 2. Perform the navigation action
  fireEvent.click(screen.getByText("About Us"));

  // 3. Verify that pushState was called
  expect(history.pushState).toHaveBeenCalled();

  // 4. Verify the new location state
  expect(location.pathname).toBe("/about");
});
```

## Summary

1. **Static Rendering**: pass a `location` object to `<Router>` to set the initial state.
2. **Interactive Tests**: pass both `location` and `history` objects (created via a helper) to `<Router>` to track and assert on navigation.
