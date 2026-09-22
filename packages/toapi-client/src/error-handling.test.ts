import {
  createRequestHandler,
  defineApi,
  defineHandler,
  PubSub,
} from "@toapi/server";
import { describe, expect, test, vi } from "vitest";
import { createFetchClient } from "./create-fetch-client.js";
import { HttpError } from "@toapi/common";

describe("Error Handling", () => {
  const cache = new PubSub();
  const handler = vi.fn(async () => {
    throw new Error("NEIN");
  });
  const serverLogger = {
    error: vi.fn(),
  };
  const api = defineApi({ cache, logger: serverLogger }).route("/throws", {
    GET: defineHandler({ authorize: () => true }, handler),
    POST: defineHandler({ authorize: () => true }, handler),
  });
  const requestHandler = createRequestHandler(api);
  const clientLogger = {
    error: vi.fn(),
  };
  const client = createFetchClient<typeof api.routes>("http://localhost", {
    fetch: (url, init) => requestHandler(new Request(url, init)),
    logger: clientLogger,
  });

  test("Errors are cached for minTTL", async () => {
    const observable = client.throws.get();

    await expect(observable).rejects.toThrow();

    expect(client.throws.get()).toBe(observable);
  });

  test("Revalidated is no dangling promise", async () => {
    try {
      await client.throws.post();
    } catch (error) {
      expect(error).toBeInstanceOf(HttpError);
    }
  });
});
