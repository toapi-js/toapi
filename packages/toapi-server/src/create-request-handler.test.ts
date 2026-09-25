import { InMemoryCache } from "@toapi/cache/in-memory-cache";
import { describe, expect, test, vi } from "vitest";
import {
  compilePathRegex,
  createRequestHandler,
} from "./create-request-handler.js";
import { defineApi } from "./define-api.js";
import { defineHandler } from "./define-handler.js";
import z from "zod";
import { INVALIDATIONS_ROUTE, TResponse } from "@toapi/common";
import { type Cache, PubSub } from "./cache.js";

describe("compilePathRegex", () => {
  test("match a simple route", () => {
    const pattern = compilePathRegex("/routes");
    expect(pattern.test("/routes")).toBe(true);
    expect(pattern.test("/routes1234")).toBe(false);
    expect(pattern.test("/routes/:id")).toBe(false);
    expect(pattern.test("/asdf")).toBe(false);
  });

  test("match route with dynamic segment", () => {
    const pattern = compilePathRegex("/routes/:id");
    expect(pattern.test("/routes/123")).toBe(true);
    expect(pattern.test("/routes/abc")).toBe(true);
    expect(pattern.test("/routes/")).toBe(false);
    expect(pattern.test("/routes")).toBe(false);
    const match = "/routes/123".match(pattern);
    expect(match?.[0]).toEqual("/routes/123");
    expect(match?.[1]).toEqual("123");
    expect(match?.groups).toEqual({ id: "123" });
  });

  test("match route with wildcard", () => {
    const pattern = compilePathRegex("/files/*");
    expect(pattern.test("/files/a")).toBe(true);
    expect(pattern.test("/files/a/b")).toBe(true);
    expect(pattern.test("/files/a/b/c")).toBe(true);
    expect(pattern.test("/files/")).toBe(false);
    expect(pattern.test("/files")).toBe(false);
    expect(pattern.test("/other")).toBe(false);
  });

  test("match route with named wildcard", () => {
    const pattern = compilePathRegex("/files/*path");
    expect(pattern.test("/files/a")).toBe(true);
    expect(pattern.test("/files/a/b")).toBe(true);
    expect(pattern.test("/files/a/b/c")).toBe(true);
    const match = "/files/a/b/c".match(pattern);
    expect(match?.groups).toEqual({ path: "a/b/c" });
  });

  test("match route with param and wildcard", () => {
    const pattern = compilePathRegex("/api/:version/*rest");
    expect(pattern.test("/api/v1/users/123")).toBe(true);
    expect(pattern.test("/api/v2/posts/456/comments")).toBe(true);
    const match = "/api/v1/users/123".match(pattern);
    expect(match?.groups).toEqual({ version: "v1", rest: "users/123" });
  });

  test("match route with uuid as path parameter", () => {
    const pattern = compilePathRegex("/api/:id");
    expect(pattern.test("/api/51696958-4970-4994-814f-ca36de54b096")).toBe(
      true,
    );
  });
});

describe("createRequestHandler", () => {
  test("applies the configured tag filter to the invalidation stream", async () => {
    const cache = new PubSub();
    const request = new Request(`http://localhost:3000${INVALIDATIONS_ROUTE}`, {
      headers: { "X-Allowed-Tag": "visible" },
    });
    const filter = vi.fn((req: Request) => (tag: string) =>
      tag === req.headers.get("X-Allowed-Tag"),
    );
    const handler = createRequestHandler(
      defineApi({ cache, revalidationStream: { filter } }),
    );
    const response = await handler(request);
    const reader = response.body!.getReader();

    await reader.read(); // initial keepalive
    await cache.delete(["hidden", "visible"]);

    expect(new TextDecoder().decode((await reader.read()).value)).toBe(
      "visible\n",
    );
    expect(filter).toHaveBeenCalledWith(request);
    await reader.cancel();
  });

  test("returns 500 for arbitrary errors in handler", async () => {
    const errorHook = vi.fn();
    const sut = createRequestHandler(
      defineApi({ logger: { error: errorHook } }).route("/", {
        GET: defineHandler(
          {
            authorize: () => true,
          },
          () => {
            throw new Error("Unexpected error");
          },
        ),
      }),
    );
    const response = await sut(new Request("http://localhost:3000"));
    expect(response.status).toBe(500);
    expect(errorHook).toHaveBeenCalled();
  });

  test("returns 400 and zod issues for validation errors", async () => {
    const errorHook = vi.fn();
    const sut = createRequestHandler(
      defineApi({ logger: { error: errorHook } }).route("/", {
        POST: defineHandler(
          {
            authorize: () => true,
            body: z.object({
              foo: z.string(),
            }),
          },
          async (req) => {
            await req.data();
            return new TResponse();
          },
        ),
      }),
    );
    const response = await sut(
      new Request("http://localhost:3000", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify({}),
      }),
    );
    expect(response.status).toBe(400);
    expect(response.headers.get("Content-Type")).toBe(
      "application/json+zodissues",
    );
    expect(await response.json()).toEqual([
      {
        code: "invalid_type",
        expected: "string",
        message: "Invalid input: expected string, received undefined",
        path: ["foo"],
      },
    ]);
    expect(errorHook).toHaveBeenCalled();
  });

  test("req.invalidate calls cache.delete with the given tags", async () => {
    const deleteMock = vi.fn().mockResolvedValue(undefined);
    const mockCache: Cache = {
      get: vi.fn().mockResolvedValue(null),
      set: vi.fn().mockResolvedValue(undefined),
      delete: deleteMock,
      subscribe: vi.fn().mockReturnValue(() => {}),
    };
    const sut = createRequestHandler(
      defineApi({ cache: mockCache }).route("/", {
        GET: defineHandler(
          { authorize: () => true },
          async (req) => {
            await req.invalidate(["posts", "users"]);
            return new TResponse();
          },
        ),
      }),
    );
    const response = await sut(new Request("http://localhost:3000"));
    expect(response.status).toBe(200);
    expect(deleteMock).toHaveBeenCalledWith(["posts", "users"], undefined);
  });

  test("does not serve cached responses to unauthorized requests", async () => {
    const authorize = vi.fn(
      (req: Request) => req.headers.get("Authorization") === "Bearer valid",
    );
    const handler = vi.fn(async () =>
      TResponse.json({ secret: "data" }, { cache: { tags: ["secret"] } }),
    );
    const sut = createRequestHandler(
      defineApi({ cache: new InMemoryCache(), logger: { error: vi.fn() } }).route(
        "/secret",
        { GET: defineHandler({ authorize }, handler) },
      ),
    );

    // while the cache is empty, an unauthorized request is rejected
    const unauthorized = await sut(
      new Request("http://localhost:3000/secret"),
    );
    expect(unauthorized.status).toBe(401);
    expect(handler).not.toHaveBeenCalled();

    // an authorized request runs the handler and populates the cache
    const authorized = await sut(
      new Request("http://localhost:3000/secret", {
        headers: { Authorization: "Bearer valid" },
      }),
    );
    expect(authorized.status).toBe(200);
    expect(await authorized.json()).toEqual({ secret: "data" });
    expect(handler).toHaveBeenCalledTimes(1);

    // the same unauthorized request must still be rejected,
    // even though a cached response exists for this URL
    const cached = await sut(new Request("http://localhost:3000/secret"));
    expect(cached.status).toBe(401);
    expect(handler).toHaveBeenCalledTimes(1);
  });

  test("auth data is available on the request object", async () => {
    const errorHook = vi.fn();
    const sut = createRequestHandler(
      defineApi({ logger: { error: errorHook } }).route("/", {
        GET: defineHandler(
          {
            authorize: () => "foo",
          },
          async (req) => {
            return TResponse.json({ auth: req.auth() });
          },
        ),
      }),
    );
    const response = await sut(new Request("http://localhost:3000"));
    expect(await response.json()).toEqual({ auth: "foo" });
  });
});

describe("openapi.json route", () => {
  test("serves the generated spec when oas config is provided", async () => {
    const sut = createRequestHandler(
      defineApi({ oas: { title: "My API", version: "1.2.3" } }).route(
        "/things",
        {
          GET: defineHandler({ authorize: () => true }, async () =>
            TResponse.json([]),
          ),
        },
      ),
      { basePath: "/api" },
    );
    const response = await sut(
      new Request("http://localhost:3000/api/__tapi/openapi.json"),
    );
    expect(response.status).toBe(200);
    expect(response.headers.get("Content-Type")).toBe("application/json");
    const body = await response.json();
    expect(body.info.title).toBe("My API");
    expect(body.info.version).toBe("1.2.3");
    expect(body.paths["/things"].get).toBeDefined();
  });

  test("falls through to 404 when oas config is not provided", async () => {
    const sut = createRequestHandler(
      defineApi().route("/things", {
        GET: defineHandler({ authorize: () => true }, async () =>
          TResponse.json([]),
        ),
      }),
    );
    const response = await sut(
      new Request("http://localhost:3000/__tapi/openapi.json"),
    );
    expect(response.status).toBe(404);
  });

  test("respects basePath", async () => {
    const sut = createRequestHandler(
      defineApi({ oas: { title: "T", version: "0.0.1" } }).route("/x", {
        GET: defineHandler({ authorize: () => true }, async () =>
          TResponse.json([]),
        ),
      }),
      { basePath: "/api" },
    );
    const response = await sut(
      new Request("http://localhost:3000/__tapi/openapi.json"),
    );
    expect(response.status).toBe(404);
  });

  test("memoizes the spec across requests", async () => {
    const sut = createRequestHandler(
      defineApi({ oas: { title: "T", version: "0.0.1" } }).route("/x", {
        GET: defineHandler({ authorize: () => true }, async () =>
          TResponse.json([]),
        ),
      }),
    );
    const first = await sut(
      new Request("http://localhost:3000/__tapi/openapi.json"),
    );
    const second = await sut(
      new Request("http://localhost:3000/__tapi/openapi.json"),
    );
    expect(await first.text()).toBe(await second.text());
  });
});
