import { describe, expect, test, vi } from "vitest";
import { PubSub } from "./cache.js";
import { streamRevalidatedTags } from "./revalidation-stream.js";
import { SESSION_COOKIE_NAME } from "@toapi/common";

describe("revalidation stream", () => {
  const req = new Request("http://localhost:3000/invalidations");

  test("should set session cookie", async () => {
    const cache = new PubSub();
    const response = streamRevalidatedTags({ cache, req });

    expect(
      response.headers.get("Set-Cookie")?.startsWith(`${SESSION_COOKIE_NAME}=`)
    ).toBeTruthy();
  });

  test("should flush an initial keepalive so headers are sent immediately", async () => {
    const cache = new PubSub();
    const response = streamRevalidatedTags({ cache, req });

    const result = await response.body?.getReader().read();
    expect(new TextDecoder().decode(result?.value)).toBe("\n");
  });

  test("should send revalidated tags", async () => {
    const cache = new PubSub();
    const response = streamRevalidatedTags({ cache, req });
    const reader = response.body!.getReader();

    // consume the initial keepalive
    await reader.read();

    await cache.delete(["tag1"]);

    const result = await reader.read();
    expect(new TextDecoder().decode(result?.value)).toBe("tag1\n");
  });

  test("should filter revalidated tags using the stream request", async () => {
    const cache = new PubSub();
    const request = new Request("http://localhost:3000/invalidations", {
      headers: { "X-Allowed-Tag": "visible" },
    });
    const filter = vi.fn((req: Request) => (tag: string) =>
      tag === req.headers.get("X-Allowed-Tag"),
    );
    const response = streamRevalidatedTags({
      cache,
      req: request,
      config: { filter },
    });
    const reader = response.body!.getReader();

    await reader.read(); // initial keepalive
    await cache.delete(["hidden", "visible"]);

    expect(new TextDecoder().decode((await reader.read()).value)).toBe(
      "visible\n",
    );
    expect(filter).toHaveBeenCalledWith(request);
    await reader.cancel();
  });
});
