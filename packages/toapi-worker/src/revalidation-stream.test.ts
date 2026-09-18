import "fake-indexeddb/auto";
import {
  CONNECT_POST_EVENT,
  EXPIRES_AT_HEADER,
  INVALIDATION_POST_EVENT,
  TAGS_CONTENT_TYPE,
  TAGS_HEADER,
} from "@toapi/common";
import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";
import { getCachedEntry, getMetadata, storeCacheEntry } from "./cache.js";
import { listenForInvalidations } from "./revalidation-stream.js";

class FakeCache {
  private store = new Map<string, Response>();

  async match(req: RequestInfo) {
    return this.store.get(keyFor(req));
  }

  async put(req: RequestInfo, res: Response) {
    this.store.set(keyFor(req), res);
  }

  async delete(req: RequestInfo) {
    return this.store.delete(keyFor(req));
  }
}

class FakeCacheStorage {
  private caches = new Map<string, FakeCache>();

  async open(name: string) {
    let cache = this.caches.get(name);
    if (!cache) {
      cache = new FakeCache();
      this.caches.set(name, cache);
    }
    return cache as unknown as Cache;
  }

  async delete(name: string) {
    return this.caches.delete(name);
  }
}

function keyFor(req: RequestInfo) {
  return typeof req === "string" ? req : req.url;
}

function taggedResponse(value: number, tags: string[]) {
  return new Response(JSON.stringify({ value }), {
    status: 200,
    headers: {
      "Content-Type": "application/json",
      [TAGS_HEADER]: tags.join(" "),
      [EXPIRES_AT_HEADER]: String(Date.now() + 60_000),
    },
  });
}

interface StreamHandle {
  enqueue(line: string): void;
}

function invalidationStreamResponse(): {
  response: Response;
  stream: StreamHandle;
} {
  let controller!: ReadableStreamDefaultController<Uint8Array>;
  const encoder = new TextEncoder();
  const body = new ReadableStream<Uint8Array>({
    start(c) {
      controller = c;
    },
  });

  return {
    response: new Response(body, {
      status: 200,
      headers: { "Content-Type": TAGS_CONTENT_TYPE },
    }),
    stream: {
      enqueue: (line) => controller.enqueue(encoder.encode(`${line}\n`)),
    },
  };
}

describe("worker revalidation stream", () => {
  const fakeClient = { postMessage: vi.fn() };

  beforeEach(() => {
    (globalThis as any).self = globalThis;
    (globalThis as any).caches = new FakeCacheStorage();
    (globalThis as any).clients = {
      matchAll: vi.fn(async () => [fakeClient]),
    };
  });

  afterEach(() => {
    // note: cache.ts memoizes its IndexedDB/CacheStorage handles at module
    // scope and never closes them, so calling deleteCache() here would hang
    // waiting for a "blocked" deleteDatabase to resolve. Tests below use
    // disjoint request URLs per assertion instead of relying on a reset.
    vi.clearAllMocks();
  });

  test("revalidates matching cache entries and notifies clients", async () => {
    const logger = { error: vi.fn(), warn: vi.fn(), info: vi.fn() };
    const { response, stream } = invalidationStreamResponse();
    (globalThis as any).fetch = vi.fn(async () => response);

    // don't await: the stream stays open for the lifetime of the worker
    listenForInvalidations({ url: "/api/__tapi/invalidations", logger });

    await vi.waitFor(() =>
      expect(logger.info).toHaveBeenCalledWith(
        "Invalidation Stream Connection Established",
      ),
    );
    await vi.waitFor(() =>
      expect(fakeClient.postMessage).toHaveBeenCalledWith({
        type: CONNECT_POST_EVENT,
      }),
    );
    expect(logger.error).not.toHaveBeenCalled();

    // populate the cache only once the initial connect-time expiry pass
    // has already run, so it isn't wiped out by that instead of by the
    // tag invalidation this test is actually exercising
    const request = new Request("http://localhost/api/foo");
    await storeCacheEntry(request, taggedResponse(1, ["foo"]));
    expect(await getCachedEntry(request)).toBeTruthy();

    fakeClient.postMessage.mockClear();
    stream.enqueue("foo");

    await vi.waitFor(() =>
      expect(fakeClient.postMessage).toHaveBeenCalledWith({
        type: INVALIDATION_POST_EVENT,
        tags: ["foo"],
      }),
    );

    expect(await getCachedEntry(request)).toBeUndefined();
    expect(await getMetadata(request.url)).toBeNull();
  });

  test("leaves unrelated cache entries alone", async () => {
    const logger = { error: vi.fn(), warn: vi.fn(), info: vi.fn() };
    const { response, stream } = invalidationStreamResponse();
    (globalThis as any).fetch = vi.fn(async () => response);

    listenForInvalidations({ url: "/api/__tapi/invalidations", logger });

    await vi.waitFor(() =>
      expect(logger.info).toHaveBeenCalledWith(
        "Invalidation Stream Connection Established",
      ),
    );

    const fooRequest = new Request("http://localhost/api/foo");
    const barRequest = new Request("http://localhost/api/bar");
    await storeCacheEntry(fooRequest, taggedResponse(1, ["foo"]));
    await storeCacheEntry(barRequest, taggedResponse(2, ["bar"]));

    stream.enqueue("foo");

    await vi.waitFor(() =>
      expect(fakeClient.postMessage).toHaveBeenCalledWith({
        type: INVALIDATION_POST_EVENT,
        tags: ["foo"],
      }),
    );

    expect(await getCachedEntry(fooRequest)).toBeUndefined();
    expect(await getCachedEntry(barRequest)).toBeTruthy();
  });
});
