import { TResponse } from "@toapi/common";
import { createRequestHandler, defineApi, defineHandler } from "@toapi/server";
import { beforeEach, describe, expect, test, vi } from "vitest";
import { createFetchClient } from "./create-fetch-client.js";

describe("Client cache", () => {
  const get = vi.fn(async () => {
    return TResponse.json({ ok: true }, { cache: { tags: ["things"] } });
  });

  const api = defineApi().route("/things", {
    GET: defineHandler(
      {
        authorize: () => true,
      },
      get,
    ),
  });

  const logger = {
    info: vi.fn(),
  };

  const handler = createRequestHandler(api);
  let client: ReturnType<typeof createFetchClient<typeof api.routes>>;

  beforeEach(() => {
    client = createFetchClient<typeof api.routes>("http://localhost:3000", {
      fetch: (url, init) => handler(new Request(url, init)),
      logger,
    });
  });

  test("serves from cache on multiple requests", async () => {
    const a = client.things.get();
    const b = client.things.get();

    expect(a).toBe(b);
    await a;
    expect(get).toHaveBeenCalledTimes(1);
  });
});
