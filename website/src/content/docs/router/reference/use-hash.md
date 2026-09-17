---
title: "useHash"
description: "The useHash hook returns the current URL hash fragment (including the # symbol) for tab and anchor navigation."
---

The `useHash` hook provides access to the current URL hash fragment (the part after `#`). It returns a string containing the hash value, including the `#` symbol when present.

## Usage

```tsx
import { useHash } from "@toapi/router";

function TableOfContents() {
  const hash = useHash();

  return (
    <nav>
      <a
        href="#introduction"
        className={hash === "#introduction" ? "active" : ""}
      >
        Introduction
      </a>
      <a href="#features" className={hash === "#features" ? "active" : ""}>
        Features
      </a>
    </nav>
  );
}
```

## Return Value

- **Type**: `string`
- **Description**: the current hash fragment, including the leading `#`. Empty string (`""`) when there is no hash.

## Example: Tab Navigation

```tsx
import { useHash } from "@toapi/router";

function Tabs() {
  const hash = useHash();
  const activeTab = hash.slice(1) || "overview";

  return (
    <div>
      <nav>
        <a href="#overview">Overview</a>
        <a href="#activity">Activity</a>
        <a href="#settings">Settings</a>
      </nav>

      {activeTab === "overview" && <Overview />}
      {activeTab === "activity" && <Activity />}
      {activeTab === "settings" && <Settings />}
    </div>
  );
}
```

## Related

- [usePathname](/tapi/router/reference/use-pathname/) — access the current pathname
- [useSearchParams](/tapi/router/reference/use-search-params/) — access search parameters
- [useRouter](/tapi/router/reference/use-router/) — programmatic navigation
