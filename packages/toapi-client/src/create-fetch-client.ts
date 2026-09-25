import type {
  Path as BasePath,
  BaseRoute,
  Logger,
  MaybePromise,
} from "@toapi/common";
import { INVALIDATIONS_ROUTE, TAGS_HEADER } from "@toapi/common";
import { buildObservable } from "./build-observable.js";
import { Cache } from "./cache.js";
import type { Client, Revalidating } from "./client-types.js";
import { handleResponse } from "./handle-response.js";
import { listenForInvalidations } from "./invalidation-stream.js";
import { PubSub } from "./pub-sub.js";

const globalFetch = fetch;

interface Options {
  fetch?: (url: string, init: RequestInit) => Promise<Response>;
  minTTL?: number;
  maxOverdueTTL?: number;
  logger?: Logger;
  invalidationsUrl?: string | false;
}

const DEFAULT_MIN_TTL = 100;

export function createFetchClient<
  Routes extends Record<BasePath, MaybePromise<BaseRoute>>,
>(apiUrl: string, options: Options = {}) {
  const fetch = options.fetch ?? globalFetch;

  const minTTL = options?.minTTL ?? DEFAULT_MIN_TTL;

  const pubSub = new PubSub({
    minTTL,
  });

  const cache = new Cache({
    maxOverdueTTL: options.maxOverdueTTL,
    logger: options.logger,
    pubSub,
    minTTL,
  });

  const invalidationsUrl =
    options.invalidationsUrl === false
      ? null
      : (options.invalidationsUrl ?? apiUrl + INVALIDATIONS_ROUTE);

  listenForInvalidations({
    fetch,
    onInvalidate: (tags) => cache.invalidateTags(tags, false),
    onConnect: () => cache.invalidateAll(),
    logger: options.logger,
    invalidationsUrl,
  });

  function load(url: string, init: RequestInit = {}) {
    return buildObservable({
      fetch,
      url,
      init: {
        method: "GET",
        ...init,
      },
      pubSub,
      cache,
    });
  }

  async function revalidate(url: string) {
    await cache.invalidateUrl(url, true);
  }

  function mutate(
    method: string,
    url: string,
    data?: FormData | unknown,
    init: RequestInit = {},
  ) {
    const headers = new Headers(init.headers);

    if (!(data instanceof FormData) && !headers.has("Content-Type")) {
      headers.set("Content-Type", "application/json");
    }

    const response = fetch(url, {
      method,
      body:
        typeof data === "undefined"
          ? undefined
          : data instanceof FormData
            ? data
            : JSON.stringify(data),
      ...init,
      headers,
    });

    const body = response.then(handleResponse).catch(async (error) => {
      await options?.logger?.error?.(error);
      throw error;
    });

    const revalidated = response.then((res) =>
      cache.invalidateTags(
        res.headers.get(TAGS_HEADER)?.split(" ") ?? [],
        true,
      ),
    );

    return Object.defineProperty(body, "revalidated", {
      get: async () => {
        const [data] = await Promise.all([body, revalidated]);

        return data;
      },
    }) as Promise<any> & Revalidating<any>;
  }

  return new Proxy(() => {}, {
    get(_target, prop: string) {
      return createProxy(
        {
          revalidate,
          load,
          mutate,
        },
        apiUrl,
        prop,
      );
    },
  }) as unknown as Client<Routes>;
}

interface ProxyMethods {
  load(url: string, init?: RequestInit): Promise<unknown>;
  revalidate(url: string): void;
  mutate(
    method: string,
    url: string,
    data: FormData | unknown,
    init?: RequestInit,
  ): Promise<unknown> & Revalidating<unknown>;
}

function buildUrl(baseUrl: string, query: unknown) {
  if (!query || typeof query !== "object") return baseUrl;
  const searchParams = new URLSearchParams();
  for (const [key, value] of Object.entries(query)) {
    if (typeof value === "undefined") continue;
    searchParams.append(key, value as string);
  }
  return searchParams.size > 0 ? baseUrl + "?" + searchParams : baseUrl;
}

function createProxy(methods: ProxyMethods, baseUrl: string, lastProp: string) {
  return new Proxy(() => {}, {
    get(_target, prop: string | symbol) {
      if (typeof prop === "symbol") {
        if (prop === Symbol.toPrimitive) {
          return function toPrimitive() {
            return `[TApi Route ${baseUrl}/${lastProp}]`;
          };
        }
        return undefined;
      }
      return createProxy(methods, baseUrl + "/" + lastProp, prop);
    },
    apply(_target, _thisArg, args) {
      switch (lastProp) {
        case "revalidate": {
          return methods.revalidate(buildUrl(baseUrl, args[0]));
        }
        case "get": {
          return methods.load(buildUrl(baseUrl, args[0]), args[1]);
        }
        case "delete": {
          return methods.mutate(
            "DELETE",
            buildUrl(baseUrl, args[0]),
            undefined,
            args[1],
          );
        }
        case "post":
        case "put":
        case "patch": {
          return methods.mutate(
            lastProp.toUpperCase(),
            buildUrl(baseUrl, args[1]?.query),
            args[0],
            args[1],
          );
        }

        default:
          throw new Error(`Tapi: Unsupported method: ${lastProp}`);
      }
    },
  });
}
