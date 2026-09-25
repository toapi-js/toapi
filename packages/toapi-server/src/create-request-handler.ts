import { ZodError, z } from "zod/v4";
import {
  INVALIDATIONS_ROUTE,
  OPENAPI_ROUTE,
  SESSION_COOKIE_NAME,
} from "@toapi/common";
import { HttpError } from "@toapi/common";
import type { MaybePromise } from "@toapi/common";
import type { Path as BasePath } from "@toapi/common";
import type { BaseRoute } from "@toapi/common";
import { CookieStore } from "@toapi/common";
import type { ApiDefinition } from "./define-api.js";
import type { Handler } from "@toapi/common";
import type { TRequest } from "@toapi/common";
import type { Cache } from "./cache.js";
import { generateOpenAPISchema } from "./openapi.js";
import { streamRevalidatedTags } from "./revalidation-stream.js";

interface Options {
  /** the root path for all API routes */
  basePath?: string;
  /** the default maximum time-to-live (TTL) for cached responses */
  defaultTTL?: number;
}

const DEFAULT_TTL = 60 * 60 * 24 * 14;
const authUsed = Symbol("TApi.authUsed");

const headersSchema = z
  .tuple([z.string(), z.string()])
  .array()
  .optional()
  .nullable();

export function createRequestHandler(
  api: ApiDefinition<Record<BasePath, MaybePromise<BaseRoute>>>,
  options: Options = {},
) {
  const errorLog =
    api.logger?.error ??
    ((error) => {
      console.error(error);
    });

  const basePath = options.basePath ?? "";

  const routes: { pattern: RegExp; route: MaybePromise<BaseRoute> }[] = [];

  for (const [path, route] of Object.entries(api.routes)) {
    const pattern = compilePathRegex(basePath + path);
    routes.push({ pattern, route });
  }

  let openapiJson: string | undefined;

  return async (req: Request) => {
    const url = new URL(req.url);

    if (url.pathname === `${basePath}${INVALIDATIONS_ROUTE}`) {
      return streamRevalidatedTags({
        cache: api.cache,
        config: api.revalidationStreamConfig,
        req,
      });
    }

    if (api.oas && url.pathname === `${basePath}${OPENAPI_ROUTE}`) {
      if (!openapiJson) {
        const spec = await generateOpenAPISchema(api, { info: api.oas });
        openapiJson = JSON.stringify(spec);
      }
      return new Response(openapiJson, {
        headers: { "Content-Type": "application/json" },
      });
    }

    for (const { pattern, route: routePromise } of routes) {
      const match = url.pathname.match(pattern);
      const route = await routePromise;
      if (match) {
        const params = match.groups || {};
        switch (req.method) {
          case "HEAD":
          case "GET": {
            try {
              const handler = route[req.method];
              if (!handler) return new Response("Not Found", { status: 404 });

              // calls the authorizer, throws if unauthorized
              const treq = await prepareRequestWithoutBody(
                handler,
                url,
                params,
                req,
                api.cache,
              );

              try {
                // get matching cache entry
                const cached = await api.cache?.get(req.url);

                if (cached) {
                  // serve from cache
                  const body =
                    req.method === "HEAD"
                      ? null
                      : new ReadableStream({
                          start(controller) {
                            controller.enqueue(cached.attachment);
                            controller.close();
                          },
                        });
                  const headers = await headersSchema.parseAsync(cached.data);

                  return new Response(body, {
                    headers: headers ?? undefined,
                  });
                }
              } catch (error) {
                // catches errors while retrieving from cache
                // errors are logged but response is still served
                errorLog(error);
              }

              const res = await executeHandler(handler, treq);

              if (res.cache) {
                if ((treq as any)[authUsed] !== true) {
                  // cache fresh response according to cache options
                  try {
                    const cloned = res.clone();
                    api.cache
                      ?.set({
                        key: req.url,
                        data: Array.from(res.headers.entries()),
                        attachment: new Uint8Array(await cloned.arrayBuffer()),
                        ttl: res.cache.ttl ?? options.defaultTTL ?? DEFAULT_TTL,
                        tags: res.cache.tags ?? [],
                      })
                      // catches errors while caching if cache.set is async (redis cache)
                      .catch(errorLog);
                  } catch (error) {
                    // catches errors while caching if cache.set is sync (in-memory cache)
                    errorLog(error);
                  }
                }
              }
              return res;
            } catch (error) {
              // catches errors while actually handling the request
              await errorLog(error);
              return handleError(error);
            }
          }

          case "DELETE": {
            const handler = route[req.method];
            if (!handler) return new Response("Not Found", { status: 404 });
            try {
              const treq = await prepareRequestWithoutBody(
                handler,
                url,
                params,
                req,
                api.cache,
              );
              const res = await executeHandler(handler, treq);
              if (res.cache?.tags) {
                try {
                  const clientId = await treq
                    .cookies()
                    .get(SESSION_COOKIE_NAME);
                  api.cache
                    ?.delete(
                      res.cache.tags,
                      clientId ? { clientId: clientId.value } : undefined,
                    )
                    .catch(errorLog);
                } catch (error) {
                  errorLog(error);
                }
              }
              return res;
            } catch (error) {
              await errorLog(error);
              return handleError(error);
            }
          }
          case "POST":
          case "PUT":
          case "PATCH": {
            const handler = route[req.method];
            if (!handler)
              return new Response("Not Found", {
                status: 404,
                statusText: "Not Found",
              });
            try {
              const treq = await prepareRequestWithBody(
                handler,
                url,
                params,
                req,
                api.cache,
              );
              const res = await executeHandler(handler, treq);
              if (res.cache?.tags) {
                try {
                  const clientId = await treq
                    .cookies()
                    .get(SESSION_COOKIE_NAME);
                  api.cache
                    ?.delete(
                      res.cache.tags,
                      clientId ? { clientId: clientId.value } : undefined,
                    )
                    .catch(errorLog);
                } catch (error) {
                  errorLog(error);
                }
              }
              return res;
            } catch (error) {
              await errorLog(error);
              return handleError(error);
            }
          }
          default:
            return new Response("Not Found", {
              status: 404,
              statusText: "Not Found",
            });
        }
      }
    }

    return new Response("Not Found", { status: 404, statusText: "Not Found" });
  };
}

export function compilePathRegex(path: string): RegExp {
  // Handle wildcards: *name captures as named group, * catches all without capturing
  const pattern = path
    .replaceAll(/\*(\w+)/g, "(?<$1>.+)") // *name -> named capture group
    .replaceAll(/\*/g, ".+") // * -> match everything including /
    .replaceAll(/:(\w+)/g, "(?<$1>[^\\/]+)"); // :param -> named capture group
  return new RegExp(`^${pattern}$`);
}

async function prepareRequestWithoutBody<TBody = never>(
  handler: Handler<any, any, any, TBody>,
  url: URL,
  params: Record<string, string>,
  req: Request,
  cache: Cache,
) {
  const treq = req as TRequest<any, any, any, TBody>;
  treq.params = () => {
    const decodedParams = Object.fromEntries(
      Object.entries(params).map(([key, value]) => [
        key,
        decodeURIComponent(value),
      ]),
    );
    if (handler.schema.params) {
      const schema = z.object(handler.schema.params);
      return schema.parse(decodedParams);
    }
    treq.params = () => decodedParams;
    return decodedParams;
  };
  treq.query = () => {
    const params = collectData(url.searchParams.entries());
    if (handler.schema.query) {
      const schema = z.object(handler.schema.query);
      return schema.parse(params);
    }
    treq.query = () => params;
    return params;
  };
  treq.cookies = () => {
    const cookieStore = new CookieStore(req);
    treq.cookies = () => cookieStore;
    return cookieStore;
  };
  treq.invalidate = async (tags: string[]) => {
    const clientId = await treq.cookies().get(SESSION_COOKIE_NAME);
    cache.delete(tags, clientId ? { clientId: clientId.value } : undefined);
  };
  const auth = await handler.schema.authorize(
    treq as TRequest<never, any, any, never>,
  );

  if (!auth) {
    throw new HttpError(401, "Unauthorized");
  }

  treq.auth = () => {
    (treq as any)[authUsed] = true;
    return auth;
  };

  return treq;
}

async function prepareRequestWithBody(
  handler: Handler<any, any, any, unknown>,
  url: URL,
  params: Record<string, string>,
  req: Request,
  cache: Cache,
) {
  const treq = await prepareRequestWithoutBody(
    handler,
    url,
    params,
    req,
    cache,
  );

  if (handler.schema.body) {
    treq.data = async () => handler.schema.body?.parseAsync(await req.json());
  } else {
    treq.data = () => {
      console.error(
        "Unexpected call to TRequest.data() method: no body parser specified",
      );
      throw new HttpError(500, "Internal Server Error");
    };
  }

  return treq;
}

function collectData(input: Iterable<[string, any]>) {
  const params: Record<string, string | string[]> = {};
  for (const [key, value] of input) {
    if (params[key]) {
      if (Array.isArray(params[key])) {
        params[key].push(value);
      } else {
        params[key] = [params[key], value];
      }
    } else {
      params[key] = value;
    }
  }
  return params;
}

export async function executeHandler<Body>(
  handler: Handler<any, any, any, Body>,
  req: TRequest<any, any, any, Body>,
) {
  const res = await handler.handler(req);
  if (handler.schema.response) {
    await handler.schema.response.parseAsync(res.data);
  }
  return res;
}

function handleError(error: unknown) {
  if (error instanceof ZodError) {
    return Response.json(error.issues, {
      status: 400,
      headers: {
        "Content-Type": "application/json+zodissues",
      },
    });
  }
  if (error instanceof HttpError) {
    return Response.json(
      {
        message: error.message,
        data: error.data,
      },
      {
        status: error.status,
        headers: {
          "Content-Type": "application/json+httperror",
        },
      },
    );
  }
  return new Response("Internal Server Error", { status: 500 });
}
