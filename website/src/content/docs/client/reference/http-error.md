---
title: "HttpError"
description: "The error thrown by the client for non-2xx responses, carrying the HTTP status and any structured error payload."
---

`HttpError` is the error type the client throws (rejects with) whenever a request fails with a non-`2xx` status. It is re-exported from `@toapi/client` for convenience, but the class itself is defined in [`@toapi/common`](/tapi/common/) and is the same class the server throws — so you can `instanceof`-check it consistently on both sides.

```ts
import { HttpError } from "@toapi/client";
```

## Type

```ts
class HttpError<Data = any> extends Error {
  status: number;
  data?: Data;

  constructor(status: number, message: string, data?: Data);
}
```

- **`status`** — the HTTP status code (e.g. `404`, `409`, `500`).
- **`message`** — the error message (from the server's JSON error payload, or the response's `statusText`).
- **`data`** — an optional structured payload the server attached to the error.

## How the client produces it

When a response is not `200`:

- If the response carries `Content-Type: application/json+httperror`, the client reads the JSON `{ message, data }` and throws `new HttpError(status, message, data)`.
- Otherwise it throws `new HttpError(status, statusText)`.
- As a special case, a `400` with `Content-Type: application/json+zodissues` is thrown as a `ZodError` (from `zod`) instead, so you can inspect validation issues directly.

## Handling errors

```ts
import { HttpError } from "@toapi/client";

try {
  await client.users["999"].get();
} catch (error) {
  if (error instanceof HttpError) {
    console.error(`Request failed (${error.status}): ${error.message}`);
    if (error.status === 404) {
      // handle not found
    }
    // error.data holds any structured payload the server sent
  } else {
    throw error;
  }
}
```

:::tip
Errors from failed background revalidations are not thrown to your `await` — the client keeps serving the last good value and reports the error through the configured [`Logger`](/tapi/client/reference/create-fetch-client/#logger) instead. Only the request you directly await (or the promise passed to a subscriber) rejects with `HttpError`.
:::

## Related

- [createFetchClient](/tapi/client/reference/create-fetch-client/)
- [Revalidation & subscriptions](/tapi/client/reference/revalidation/)
