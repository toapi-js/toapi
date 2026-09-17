---
title: "defineHandler"
description: "Define a single Toapi endpoint — connect request validation and authorization to your implementation with end-to-end type safety."
---

The `defineHandler` function is the core building block for creating API endpoints in Toapi. It ensures end-to-end type safety by connecting your request schema validation with your implementation logic. A route module typically exports one handler per HTTP method (`GET`, `POST`, `PUT`, `PATCH`, `DELETE`).

## Usage

```ts
import { defineHandler, TResponse } from "@toapi/server";
import { z } from "zod";

export const GET = defineHandler(
  {
    authorize: () => true,
    query: {
      limit: z.coerce.number().min(1).default(10),
    },
  },
  async (req) => {
    const { limit } = req.query();
    return TResponse.json({ limit });
  },
);
```

## Signature

```ts
function defineHandler<Response, AuthData, Params, Query, Body>(
  schema: Schema<Response, AuthData, Params, Query, Body>,
  handler: (
    req: TRequest<AuthData, Params, Query, Body>,
  ) => Promise<TResponse<Response>>,
): Handler;
```

## Parameters

### `schema`

An object defining the input validation, authorization logic, and response shape.

| Property | Type | Description |
| --- | --- | --- |
| `authorize` | `(req: TRequest) => AuthData \| Promise<AuthData>` | **Required.** Determines whether the request is allowed. Return a falsy value (or throw) to deny access — the request is rejected with `401 Unauthorized`. The return value is accessible via `req.auth()`. |
| `params` | `Record<string, ZodType>` | Optional. A map of Zod schemas validating path parameters (e.g. `/users/:id`). |
| `query` | `Record<string, ZodType>` | Optional. A map of Zod schemas validating query-string parameters. |
| `body` | `ZodType` | Optional. Zod schema validating the JSON request body, parsed and validated lazily by `req.data()`. |
| `response` | `ZodType` | Optional. Zod schema describing the success response. Validated at runtime after the handler returns, and used to describe the `200` response in the generated [OpenAPI document](/tapi/server/reference/generate-openapi-schema/). |

:::note
`params` and `query` are supplied as **maps of Zod schemas** (e.g. `{ id: z.string() }`), not a single `z.object({...})`. Toapi composes them into an object schema internally.
:::

### `handler`

The implementation function that receives the validated request and returns a response.

- **Argument**: `req` — a [`TRequest`](/tapi/server/reference/t-request/) object with typed, validated accessors.
- **Returns**: a promise resolving to a [`TResponse`](/tapi/server/reference/t-response/).

## Examples

### Basic GET route

```ts
export const GET = defineHandler(
  { authorize: () => true },
  async () => {
    return TResponse.json({ message: "Hello World" });
  },
);
```

### Route with path parameters

For a route defined as `/users/:id`:

```ts
export const GET = defineHandler(
  {
    authorize: () => true,
    params: {
      id: z.string().uuid(),
    },
  },
  async (req) => {
    const { id } = req.params();
    return TResponse.json({ userId: id });
  },
);
```

### POST route with body validation

```ts
export const POST = defineHandler(
  {
    authorize: (req) => {
      if (!req.headers.get("Authorization")) throw new HttpError(401, "Unauthorized");
      return { userId: "current-user" };
    },
    body: z.object({
      title: z.string().min(3),
      content: z.string(),
    }),
  },
  async (req) => {
    const user = req.auth();
    const { title, content } = await req.data();

    // Perform database operation...

    return TResponse.json({ success: true, author: user.userId });
  },
);
```

### POST route with form-data validation

```ts
export const POST = defineHandler(
  {
    authorize: (req) => {
      if (!req.headers.get("Authorization")) throw new HttpError(401, "Unauthorized");
      return { userId: "current-user" };
    },
  },
  async (req) => {
    const user = req.auth();

    // For multipart/form-data, read the body with `req.formData()` inside the
    // handler. The `body` schema is for JSON payloads (parsed from `req.json()`).
    const formData = await req.formData();
    const parsed = z
      .object({
        title: z.string().min(3),
        content: z.string(),
      })
      .parse({
        title: formData.get("title"),
        content: formData.get("content"),
      });

    return TResponse.json({ success: true, title: parsed.title, author: user.userId });
  },
);
```

:::tip
Reading `req.auth()` inside a `GET` handler marks the response as user-specific, so it will not be stored in the shared server cache. See [`createRequestHandler`](/tapi/server/reference/create-request-handler/#request-lifecycle) and [Caching Strategies](/tapi/server/reference/caching/).
:::

## Related

- [`TRequest`](/tapi/server/reference/t-request/) — the request object passed to your handler.
- [`TResponse`](/tapi/server/reference/t-response/) — the response helper you return.
- [`HttpError`](/tapi/server/reference/http-error/) — throw typed error responses from `authorize` or the handler.
- [`defineApi`](/tapi/server/reference/define-api/) — register the handler under a route path.
