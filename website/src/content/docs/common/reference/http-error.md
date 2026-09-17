---
title: "HttpError"
description: "A specialized error type for returning specific HTTP status codes and optional data from Toapi handlers and authorize functions."
---

The `HttpError` class is a specialized error type used to represent HTTP error responses. Throwing this error within your handlers allows you to return specific HTTP status codes and optional additional data to the client.

It is defined in `@toapi/common` and re-exported from [`@toapi/server`](/tapi/server/) and [`@toapi/client`](/tapi/client/), so you can import it from whichever package you are already using.

## Usage

You can throw an `HttpError` from anywhere within your `authorize` function or your route handler logic.

```ts
import { defineHandler, HttpError, TResponse } from "@toapi/server";

export const GET = defineHandler({ authorize: () => true }, async (req) => {
  const resource = await findResource(req.params().id);

  if (!resource) {
    throw new HttpError(404, "Resource not found");
  }

  if (resource.isPrivate) {
    throw new HttpError(403, "Access denied", { reason: "private_resource" });
  }

  return TResponse.json(resource);
});
```

## Signature

```ts
class HttpError<Data = any> extends Error {
  status: number;
  data?: Data;

  constructor(status: number, message: string, data?: Data);
}
```

## Constructor parameters

### `status`

**Type**: `number`

The HTTP status code to return (e.g., `400`, `401`, `404`, `500`).

### `message`

**Type**: `string`

A human-readable error message. This is passed to the parent `Error` constructor and is typically exposed to the client.

### `data`

**Type**: `Data` (optional)

Arbitrary additional data to include with the error response. This can be useful for passing validation errors or specific error codes that the client can handle programmatically.

## Properties

### `status`

The HTTP status code provided in the constructor.

### `data`

The optional data object provided in the constructor.

## Error handling

When using `createRequestHandler`, uncaught `HttpError`s are automatically caught and converted into a proper JSON response with the specified status code.

If you are using the `createFetchClient` on the client side, these errors will result in the promise rejecting. You can catch them and inspect the status and data.

```ts
try {
  await client.items.get();
} catch (error) {
  if (error instanceof HttpError && error.status === 404) {
    console.log("Not found!");
  }
}
```
