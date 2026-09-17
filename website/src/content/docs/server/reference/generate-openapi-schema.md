---
title: "generateOpenAPISchema"
description: "Generate an OpenAPI 3.1 document from your Toapi definition for documentation tools and cross-language SDKs."
---

The `generateOpenAPISchema` function generates an OpenAPI 3.1 specification from your [`defineApi`](/tapi/server/reference/define-api/) definition. This is useful for generating documentation (like Swagger UI) or client SDKs for other languages.

:::tip
You usually do not need to call this by hand. Pass `oas: { title, version }` to [`defineApi`](/tapi/server/reference/define-api/) and [`createRequestHandler`](/tapi/server/reference/create-request-handler/) serves the generated document at `<basePath>/__tapi/openapi.json` automatically.
:::

## Usage

```ts
import { generateOpenAPISchema } from "@toapi/server";
import { api } from "./api"; // Your ApiDefinition

async function main() {
  const openApiDoc = await generateOpenAPISchema(api, {
    info: {
      title: "My API",
      version: "1.0.0",
    },
  });

  console.log(JSON.stringify(openApiDoc, null, 2));
}

main();
```

## Signature

```ts
function generateOpenAPISchema(
  apiDefinition: ApiDefinition<Routes>,
  options: { info: { title: string; version: string } },
): Promise<OpenAPIDocument>;
```

## Parameters

### `apiDefinition`

**Type**: [`ApiDefinition`](/tapi/server/reference/api-definition/)

The API definition object returned by [`defineApi`](/tapi/server/reference/define-api/).

### `options`

| Property | Type | Description |
| --- | --- | --- |
| `info` | `{ title: string; version: string }` | **Required.** Metadata for the API specification. |

## Schema mapping

The function iterates through all routes in your API and maps the Zod schemas from your [`defineHandler`](/tapi/server/reference/define-handler/) calls to OpenAPI schema objects.

| Toapi schema | OpenAPI location |
| --- | --- |
| `params` | `parameters` (in path) |
| `query` | `parameters` (in query) |
| `body` | `requestBody` (content: `application/json`) |
| `response` | `responses.200` (content: `application/json`) |

### Path parameter transformation

Toapi uses colon syntax (e.g. `/users/:id`) for path parameters. `generateOpenAPISchema` automatically converts this to OpenAPI curly-brace syntax (e.g. `/users/{id}`).

### Response schemas

To get the most out of the generated specification, provide a `response` schema in your `defineHandler` options. It describes the success-response shape in the OpenAPI document (and is validated at runtime). When omitted, the response is described as `z.any()`.

```ts
export const GET = defineHandler(
  {
    authorize: () => true,
    // Define response schema for OpenAPI
    response: z.object({
      id: z.string(),
      name: z.string(),
    }),
  },
  async () => {
    // ...
  },
);
```

## Related

- [`defineApi`](/tapi/server/reference/define-api/) — pass `oas` to serve the document automatically.
- [`createRequestHandler`](/tapi/server/reference/create-request-handler/) — mounts the OpenAPI route.
