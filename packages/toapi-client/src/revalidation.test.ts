// @vitest-environment happy-dom
import { TAGS_HEADER } from "@toapi/common";
import { describe, expect, test, vi } from "vitest";
import { createFetchClient } from "./create-fetch-client.js";

function taggedResponse(value: number) {
  return new Response(JSON.stringify({ value }), {
    status: 200,
    headers: {
      "Content-Type": "application/json",
      [TAGS_HEADER]: "foo",
    },
  });
}

function invalidationStreamResponse() {
  // a stream that never closes, like a real long-lived connection
  return new Response(new ReadableStream({ start() {} }), { status: 200 });
}

describe("revalidation", () => {
  test("reconnects after a network failure and invalidates everything once reconnected", async () => {
    const logger = { error: vi.fn(), warn: vi.fn(), info: vi.fn() };
    let taggedCalls = 0;
    let invalidationAttempts = 0;

    const fetch = vi.fn(async (url: string) => {
      if (url.includes("/__tapi/invalidations")) {
        invalidationAttempts++;
        if (invalidationAttempts === 1) {
          throw new TypeError("Failed to fetch");
        }
        return invalidationStreamResponse();
      }

      taggedCalls++;
      return taggedResponse(taggedCalls);
    });

    const client = createFetchClient("http://localhost", {
      fetch,
      logger,
    });

    await client.tagged.get();
    expect(taggedCalls).toBe(1);

    // the first attempt to open the invalidation stream fails, so it
    // should warn and schedule a retry
    await vi.waitFor(() =>
      expect(logger.warn).toHaveBeenCalledWith(
        expect.stringContaining(
          "Reconnecting to invalidation stream after error",
        ),
      ),
    );
    expect(invalidationAttempts).toBe(1);

    // it retries and eventually connects
    await vi.waitFor(() =>
      expect(logger.info).toHaveBeenCalledWith(
        "Invalidation stream connection established",
      ),
    );
    expect(logger.error).not.toHaveBeenCalled();

    // reconnecting invalidates the whole cache, so a request for
    // previously-cached data hits the network again instead of being
    // served from cache
    await client.tagged.get();
    expect(taggedCalls).toBe(2);
  });
});
