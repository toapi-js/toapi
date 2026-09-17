---
title: "usePathname"
description: "The usePathname hook returns the current normalized pathname string for active states, guards, and breadcrumbs."
---

The `usePathname` hook provides access to the current pathname of the URL. It returns a string representing the current path without query parameters or hash fragments.

## Usage

```tsx
import { usePathname } from "@toapi/router";

function Navigation() {
  const pathname = usePathname();

  return (
    <nav>
      <a href="/" className={pathname === "/" ? "active" : ""}>
        Home
      </a>
      <a href="/about" className={pathname === "/about" ? "active" : ""}>
        About
      </a>
    </nav>
  );
}
```

## Return Value

- **Type**: `string`
- **Description**: the current pathname without query parameters or hash fragment
- **Examples**:
  - `/` for the root path
  - `/users` for a users page
  - `/users/123` for a specific user
  - `/products/category/electronics` for nested paths

## Pathname Normalization

The pathname is normalized by the [`Router`](/tapi/router/reference/router/) component:

- Trailing slashes are removed: `/users/` becomes `/users`
- Root path remains `/`

## Examples

### Active Navigation Links

```tsx
import { Link, usePathname } from "@toapi/router";

function NavItem({ href, children }) {
  const pathname = usePathname();
  const isActive = pathname === href;

  return (
    <Link href={href} className={`nav-item ${isActive ? "active" : ""}`}>
      {children}
    </Link>
  );
}

function Navigation() {
  return (
    <nav>
      <NavItem href="/">Home</NavItem>
      <NavItem href="/products">Products</NavItem>
      <NavItem href="/about">About</NavItem>
      <NavItem href="/contact">Contact</NavItem>
    </nav>
  );
}
```

### Conditional Rendering Based on Path

```tsx
function Layout({ children }) {
  const pathname = usePathname();
  const isAdminPage = pathname.startsWith("/admin");
  const isPublicPage = ["/", "/about", "/contact"].includes(pathname);

  return (
    <div className="layout">
      {!isAdminPage && <Header />}

      <main className={isAdminPage ? "admin-layout" : "public-layout"}>
        {children}
      </main>

      {isPublicPage && <Footer />}
    </div>
  );
}
```

### Breadcrumb Generation

```tsx
import { Link, usePathname } from "@toapi/router";

function Breadcrumbs() {
  const pathname = usePathname();

  const pathSegments = pathname.split("/").filter(Boolean);

  const breadcrumbs = pathSegments.map((segment, index) => {
    const href = "/" + pathSegments.slice(0, index + 1).join("/");
    const label = segment.charAt(0).toUpperCase() + segment.slice(1);
    return { href, label };
  });

  return (
    <nav aria-label="Breadcrumb">
      <Link href="/">Home</Link>
      {breadcrumbs.map((crumb, index) => (
        <span key={crumb.href}>
          <span> / </span>
          {index === breadcrumbs.length - 1 ? (
            <span aria-current="page">{crumb.label}</span>
          ) : (
            <Link href={crumb.href}>{crumb.label}</Link>
          )}
        </span>
      ))}
    </nav>
  );
}
```

### Analytics and Tracking

```tsx
function AnalyticsProvider({ children }) {
  const pathname = usePathname();

  useEffect(() => {
    analytics.track("Page View", {
      path: pathname,
      timestamp: new Date().toISOString(),
    });
  }, [pathname]);

  return <>{children}</>;
}
```

## Best Practices

1. **Use for UI state**: perfect for conditional rendering and styling based on the current path.
2. **Memoize expensive computations**: use `useMemo` for complex path-based calculations.
3. **Combine with other hooks**: often used together with [`useParams`](/tapi/router/reference/use-params/) and [`useSearchParams`](/tapi/router/reference/use-search-params/).
4. **Avoid side effects**: don't use pathname changes to trigger navigation — use [`useRouter`](/tapi/router/reference/use-router/) instead.

## Common Anti-Patterns

### Don't use it for navigation

```tsx
// Wrong - don't navigate based on pathname changes
useEffect(() => {
  if (pathname === "/old-path") {
    router.push("/new-path");
  }
}, [pathname]);
```

### Don't ignore normalization

```tsx
// Wrong - might not match due to trailing slash
const isActive = pathname === "/users/";

// Correct - router normalizes trailing slashes
const isActive = pathname === "/users";
```

## Related

- [useRouter](/tapi/router/reference/use-router/) — programmatic navigation
- [useParams](/tapi/router/reference/use-params/) — access route parameters
- [useSearchParams](/tapi/router/reference/use-search-params/) — access search parameters
- [useHash](/tapi/router/reference/use-hash/) — access the URL hash fragment
