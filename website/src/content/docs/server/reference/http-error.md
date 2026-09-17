---
title: "HttpError"
description: "Throw a typed HTTP error from a Toapi handler or authorizer to control the status code and error payload of the response."
---

`HttpError` is an `Error` subclass you throw from an `authorize` function or a route handler to control the HTTP status and payload of the error response. It is re-exported from `@toapi/server` (defined in `@toapi/common`). When the [request handler](/tapi/server/reference/create-request-handler/#error-handling) catches an `HttpError`, it responds with the given `status`, a JSON body of `{ message, data }`, and `Content-Type: application/json+httperror` — a shape the [client](/tapi/client/) understands and rethrows.

## Signature

```ts
class HttpError<Data = any> extends Error {
  status: number;
  data?: Data;

  constructor(status: number, message: string, data?: Data);
}
```

| Argument | Type | Description |
| --- | --- | --- |
| `status` | `number` | The HTTP status code to send (e.g. `401`, `404`, `422`). |
| `message` | `string` | A human-readable error message, sent as `message` in the JSON body. |
| `data` | `Data` (optional) | Arbitrary extra payload, sent as `data` in the JSON body — for example field-level validation details. |

## Usage

### Deny a request in `authorize`

```ts
import { defineHandler, HttpError, TResponse } from "@toapi/server";

export const GET = defineHandler(
  {
    authorize: (req) => {
      const token = req.headers.get("Authorization");
      if (!token) throw new HttpError(401, "Unauthorized");
      return verify(token);
    },
  },
  async () => TResponse.json({ ok: true }),
);
```

### Signal a not-found or validation error in a handler

```ts
export const GET = defineHandler(
  { authorize: () => true, params: { id: z.string() } },
  async (req) => {
    const book = await db.books.find(req.params().id);
    if (!book) throw new HttpError(404, "Book not found");
    return TResponse.json(book);
  },
);
```

### Attach structured data

```ts
throw new HttpError(422, "Validation failed", {
  fields: { email: "already in use" },
});
```

The client receives the `status`, `message`, and `data`, letting you branch on typed error details.

:::note
Uncaught non-`HttpError` errors are reported to the [logger](/tapi/server/reference/define-api/#logger-interface) and returned as a generic `500 Internal Server Error`, so internal failure details never leak to clients. Throw `HttpError` whenever you want a specific status and message to reach the caller. Zod validation failures are handled separately as `400` responses.
:::

## Related

- [`defineHandler`](/tapi/server/reference/define-handler/) — where you typically throw `HttpError`.
- [`createRequestHandler`](/tapi/server/reference/create-request-handler/#error-handling) — how thrown errors become responses.
