import { InMemoryCache } from "@toapi/cache/in-memory-cache";
import { describe, expect, test, vi } from "vitest";
import { defineApi } from "./define-api.js";
import { createRequestHandler } from "./create-request-handler.js";
import { defineHandler } from "./define-handler.js";
import { INVALIDATIONS_ROUTE, TAGS_HEADER, TResponse } from "@toapi/common";
import { hashTag } from "./hash-tags.js";

describe("Hashed Tags", async () => {
  const secret = "superSecretTestSecret";
  const tags = ["foo", "bar"];
  const te = new TextEncoder();

  const expectedTags = await Promise.all(
    tags.map((tag) => hashTag(te, secret, tag)),
  );

  const cache = new InMemoryCache();

  const respond = vi.fn(async () => {
    return TResponse.void({ cache: { tags } });
  });

  const api = defineApi({
    revalidationStream: {
      hashSecret: secret,
    },
    cache,
  }).route("/foo", {
    GET: defineHandler(
      {
        authorize: () => true,
      },
      respond,
    ),
    POST: defineHandler(
      {
        authorize: () => true,
      },
      respond,
    ),
    DELETE: defineHandler(
      {
        authorize: () => true,
      },
      respond,
    ),
  }).route("/cached", {
    GET: defineHandler(
      {
        authorize: () => true,
      },
      respond,
    ),
  });

  const handler = createRequestHandler(api);

  test.for([["GET"], ["POST"], ["DELETE"]])(
    "hashes tags in %s response header",
    async ([method]) => {
      const res = await handler(
        new Request("http://localhost/foo", {
          method,
        }),
      );
      expect(res.headers.get(TAGS_HEADER)).toBe(expectedTags.join(" "));
    },
  );

  // https://github.com/toapi-js/toapi/issues/534
  test("hashes tags in GET response served from server cache", async () => {
    const first = await handler(new Request("http://localhost/cached"));
    expect(first.headers.get(TAGS_HEADER)).toBe(expectedTags.join(" "));

    const callsBefore = respond.mock.calls.length;
    const second = await handler(new Request("http://localhost/cached"));
    // second request must be a cache hit
    expect(respond.mock.calls.length).toBe(callsBefore);
    expect(second.headers.get(TAGS_HEADER)).toBe(expectedTags.join(" "));

    // tags are kept in plain text in cache storage
    const stored = new Headers(
      (await cache.get("http://localhost/cached"))?.data as [string, string][],
    );
    expect(stored.get(TAGS_HEADER)).toBe(tags.join(" "));
  });

  test("hashes tags in invalidation stream", async () => {
    const stream = await handler(
      new Request(`http://localhost${INVALIDATIONS_ROUTE}`),
    );
    cache.delete(tags);
    const reader = stream.body?.getReader();
    const chunk0 = await reader?.read();
    expect(chunk0?.value).toEqual(te.encode("\n"));
    const chunk1 = await reader?.read();
    expect(chunk1?.value).toEqual(te.encode(expectedTags.join(" ") + "\n"));
  });
});
