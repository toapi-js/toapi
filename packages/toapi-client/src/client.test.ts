import { HttpError, TResponse } from "@toapi/common";
import { createRequestHandler, defineApi, defineHandler } from "@toapi/server";
import { beforeEach, describe, expect, test, vi } from "vitest";
import { mockLogger, type api } from "./api.mock.js";
import { createFetchClient } from "./create-fetch-client.js";
import { requestHandler } from "./request-handler.mock.js";

describe("createFetchClient", () => {
  const fetch = vi.fn((url: string, init: RequestInit) => {
    return requestHandler(new Request(url, init));
  });
  const logger = {
    error: vi.fn(),
  };
  let client = createFetchClient<typeof api.routes>("https://example.com/api", {
    fetch,
    logger,
  });

  beforeEach(() => {
    fetch.mockClear();
    client = createFetchClient<typeof api.routes>("https://example.com/api", {
      fetch,
      logger,
    });
  });

  test("get books", async () => {
    const response = await client.books.get();
    expect(fetch).toHaveBeenCalledWith("https://example.com/api/books", {
      method: "GET",
    });
    expect(response).toEqual([
      { id: "1", title: "Book 1" },
      { id: "2", title: "Book 2" },
    ]);
  });

  test("get book", async () => {
    const response = await client.books[1]!.get({ test: "asdf" });
    expect(fetch).toHaveBeenCalledWith(
      "https://example.com/api/books/1?test=asdf",
      {
        method: "GET",
      },
    );
    expect(response).toEqual({ id: "1", title: "Book 1" });
  });

  test("post book", async () => {
    const response = await client.books.post({
      id: "3",
      title: "Book 3",
    });
    expect(fetch).toHaveBeenCalledWith("https://example.com/api/books", {
      method: "POST",
      headers: new Headers({
        "Content-Type": "application/json",
      }),
      body: JSON.stringify({
        id: "3",
        title: "Book 3",
      }),
    });
    expect(response).toEqual({ id: "3", title: "Book 3" });
  });

  test("subscribe to query", async () => {
    const cb = vi.fn();
    const promise = client.books.get();
    const unsubscribe = promise.subscribe(cb);
    await promise;
    expect(cb).toHaveBeenCalledTimes(1);
    await client.books.revalidate();
    expect(cb).toHaveBeenCalledTimes(2);
    // Regression check: an unsubscribed callback must stop receiving updates.
    unsubscribe();
    await client.books.revalidate();
    expect(cb).toHaveBeenCalledTimes(2);
  });

  test("tag-based revalidation", async () => {
    const cb = vi.fn();
    const promise = client.movies[1]!.get({ test: "asdf" });
    promise.subscribe(cb);
    const data = await promise;
    expect(data.id).toEqual("1");
    expect(cb).toHaveBeenCalledTimes(1);
    await client.movies.post({ id: "3", title: "Movie 3" }).revalidated;
    expect(cb).toHaveBeenCalledTimes(2);
  });

  test("wildcard route", async () => {
    const response = await client.files["documents/report.pdf"]!.get();
    expect(fetch).toHaveBeenCalledWith(
      "https://example.com/api/files/documents/report.pdf",
      {
        method: "GET",
      },
    );
    expect(response).toEqual({
      path: "documents/report.pdf",
      message: "Accessing file: documents/report.pdf",
    });
  });

  test("as form action", async () => {
    const formData = new FormData();
    formData.set("id", "3");
    formData.set("title", "Movie 3");
    const response = await client.formData.post(formData);
    expect(response).toEqual({ id: "3", title: "Movie 3" });
  });

  test("not found", async () => {
    vi.useFakeTimers();
    const promise = client.error["not-found"].get();
    await expect(promise).rejects.toThrow();
    const anotherPromise = client.error["not-found"].get();
    expect(anotherPromise).toBe(promise);
    await expect(anotherPromise).rejects.toThrow();
    expect(logger.error).toHaveBeenCalled();

    await vi.advanceTimersByTimeAsync(150);

    const thirdPromise = client.error["not-found"].get();
    expect(thirdPromise).not.toBe(promise);
    await expect(thirdPromise).rejects.toThrow();
  });

  test("TTL-based revalidation fires after TTL, not immediately", async () => {
    vi.useFakeTimers();
    // A fresh client with no jitter so timing is deterministic
    const ttlClient = createFetchClient<typeof api.routes>(
      "https://example.com/api",
      { fetch, maxOverdueTTL: 0 },
    );
    try {
      const ttlSeconds = 60;

      // Initial fetch — entry.current is not yet set when we subscribe
      const observable = ttlClient.cached.get();
      const unsubscribe = observable.subscribe(vi.fn());
      await observable;
      await Promise.resolve(); // let waitForRevalidation finish setting entry.current

      // Unsubscribe so size drops to 0, then re-subscribe:
      // the subscribe handler sees entry.current.expiresAt and schedules the TTL timeout
      unsubscribe();
      observable.subscribe(vi.fn());

      expect(fetch).toHaveBeenCalledTimes(1);

      // Before TTL expires: no revalidation
      await vi.advanceTimersByTimeAsync((ttlSeconds - 1) * 1000);
      expect(fetch).toHaveBeenCalledTimes(1);

      // After TTL expires: revalidation should fire
      await vi.advanceTimersByTimeAsync(2 * 1000);
      expect(fetch).toHaveBeenCalledTimes(2);
    } finally {
      vi.useRealTimers();
    }
  });

  test("Symbol.toPrimitive", async () => {
    expect((client.method as any)[Symbol.toPrimitive]()).toBe(
      "[TApi Route https://example.com/api/method]",
    );
  });

  test("Stream", async () => {
    const response = await client.stream.get();
    // @ts-ignore
    expect(await Array.fromAsync(response)).toEqual([
      { value: 0 },
      { value: 1 },
      { value: 2 },
      { value: 3 },
      { value: 4 },
    ]);
  });

  test("optional query parameters", async () => {
    expect(await client.optionalQuery.get()).toEqual({});
    expect(await client.optionalQuery.get({ optional: "foo" })).toEqual({
      optional: "foo",
    });
  });

  test("undefined query parameters are omitted", async () => {
    expect(await client.optionalQuery.get({ optional: undefined })).toEqual({});
    expect(fetch).toHaveBeenCalledWith(
      "https://example.com/api/optionalQuery",
      { method: "GET" },
    );
  });

  test("errors are propagated", async () => {
    const observable = client.error["not-found"].get();

    await expect(observable).rejects.toThrow(new HttpError(404, "Not Found"));

    // The server-side handler logs the error it threw.
    expect(mockLogger.error).toHaveBeenCalled();
  });

  test("client logger is notified of background GET failures", async () => {
    // Regression check: Cache used to log every non-4xx fetch error via the
    // `logger` option (see the old setResolveHook `this.errorLog(error)`).
    // The new Cache stores a `logger`-derived errorLog field but never calls
    // it, so this currently fails — background GET failures go unlogged.
    const logClientError = vi.fn();
    const flakyClient = createFetchClient<typeof api.routes>(
      "https://example.com/api",
      {
        fetch: vi.fn(async () => {
          throw new Error("network down");
        }),
        logger: { error: logClientError },
      },
    );

    await expect(flakyClient.books.get()).rejects.toThrow("network down");

    expect(logClientError).toHaveBeenCalledWith(expect.any(Error));
  });

  test("cache eviction on 404", async () => {
    let exists = true;
    const getThing = vi.fn(async () => {
      if (!exists) throw new HttpError(404, "Not Found");
      return TResponse.json({ name: "thing" }, { cache: { tags: ["thing"] } });
    });
    const deleteThing = vi.fn(async () => {
      exists = false;
      return TResponse.json({ deleted: true }, { cache: { tags: ["thing"] } });
    });
    const api = defineApi({
      logger: mockLogger,
    }).route("/thing", {
      GET: defineHandler({ authorize: () => true }, getThing),
      DELETE: defineHandler({ authorize: () => true }, deleteThing),
    });

    const handler = createRequestHandler(api, {
      basePath: "/api",
    });

    const logClientError = vi.fn();

    const client = createFetchClient<typeof api.routes>("http://test/api", {
      fetch: async (url, init) => {
        return handler(new Request(url, init));
      },
      logger: {
        error: logClientError,
      },
    });

    const sub = vi.fn();
    const observable = client.thing.get();
    const thing = await observable;
    const unsubscribe = observable.subscribe(sub);

    expect(thing).toEqual({ name: "thing" });

    await client.thing.delete();
    unsubscribe();

    // Delete invalidates the "thing" tag, so the subscriber sees one update.
    expect(sub).toHaveBeenCalledTimes(2);
    expect(mockLogger.error).toHaveBeenCalledWith(
      new HttpError(404, "Not Found"),
    );
    expect(logClientError).not.toHaveBeenCalled();

    await expect(client.thing.get()).rejects.toThrow(HttpError);
  });

  test("stale tag mapping causes a spurious cache eviction when a URL's tags change", async () => {
    // Regression check (review finding 2, cache.ts:46): tagIndex only ever
    // adds tag -> url mappings and never removes the old ones when a URL's
    // tags change on refetch, so invalidating a tag the URL no longer
    // carries still evicts it from the cache.
    //
    // This is checked via refetch counts rather than subscriber
    // notifications, since notifications go through PubSub's own buggy
    // debounce (review finding 3) which would confound the result here.
    // `invalidateTags` deletes the storage entry synchronously before
    // touching PubSub, so the eviction itself is directly observable.
    let published = false;
    const getPost = vi.fn(async () =>
      TResponse.json(
        { title: "post" },
        { cache: { tags: [published ? "published" : "draft"] } },
      ),
    );
    const touchDraft = vi.fn(async () =>
      TResponse.json({ ok: true }, { cache: { tags: ["draft"] } }),
    );
    const api = defineApi({ logger: mockLogger })
      .route("/post", {
        GET: defineHandler({ authorize: () => true }, getPost),
      })
      .route("/touchDraft", {
        POST: defineHandler({ authorize: () => true }, touchDraft),
      });

    const handler = createRequestHandler(api, { basePath: "/api" });
    const client = createFetchClient<typeof api.routes>("http://test/api", {
      fetch: (url, init) => handler(new Request(url, init)),
    });

    await client.post.get(); // cached under tag "draft"

    // Republish the post: the cache entry is dropped and, on the next
    // .get(), refetched, now carrying only the "published" tag.
    published = true;
    await client.post.revalidate();
    await client.post.get();
    expect(getPost).toHaveBeenCalledTimes(2);

    // A mutation invalidating "draft" — a tag the post no longer carries —
    // must not evict the now-published post from the cache.
    await client.touchDraft.post().revalidated;
    await client.post.get();
    expect(getPost).toHaveBeenCalledTimes(2);
  });

  test("debounce", async () => {
    vi.useFakeTimers();
    const minTTL = 1000;
    const debounceClient = createFetchClient<typeof api.routes>(
      "https://example.com/api",
      { fetch, minTTL },
    );
    try {
      const cb = vi.fn();
      const observable = debounceClient.books.get();
      observable.subscribe(cb);
      await observable;

      await debounceClient.books.revalidate();
      expect(cb).toHaveBeenCalledTimes(2);

      await debounceClient.books.revalidate();
      expect(cb).toHaveBeenCalledTimes(2);

      await vi.advanceTimersByTimeAsync(minTTL);
      expect(cb).toHaveBeenCalledTimes(3);
    } finally {
      vi.useRealTimers();
    }
  });

  test("a single invalidation does not self-repeat after minTTL", async () => {
    // Regression check: PubSub.publish() unconditionally re-publishes the
    // same urls via setTimeout after `minTTL`, even when no second
    // invalidation happened. A single revalidate() should only notify
    // subscribers once — it must not fire again on its own once minTTL
    // elapses.
    vi.useFakeTimers();
    const minTTL = 1000;
    const debounceClient = createFetchClient<typeof api.routes>(
      "https://example.com/api",
      { fetch, minTTL },
    );
    try {
      const cb = vi.fn();
      const observable = debounceClient.books.get();
      observable.subscribe(cb);
      await observable;

      await debounceClient.books.revalidate();
      expect(cb).toHaveBeenCalledTimes(2);

      // No further invalidation — waiting out minTTL must not trigger
      // another notification on its own.
      await vi.advanceTimersByTimeAsync(minTTL);
      expect(cb).toHaveBeenCalledTimes(2);
    } finally {
      vi.useRealTimers();
    }
  });
});
