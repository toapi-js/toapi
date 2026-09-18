import { act, render } from "@testing-library/react";
import { createFetchClient, HttpError } from "@toapi/client";
import {
  createRequestHandler,
  defineApi,
  defineHandler,
  PubSub,
  TResponse,
} from "@toapi/server";
import { Suspense } from "react";
import { ErrorBoundary } from "react-error-boundary";
import { beforeEach, describe, expect, test, vi } from "vitest";
import { useQuery } from "./use-query.js";

describe("useQuery error handling", () => {
  const get = vi.fn<() => Promise<TResponse<{ ok: boolean }>>>(async () => {
    throw new Error("NEIN");
  });
  const post = vi.fn<() => Promise<TResponse>>();
  const serverLogger = {
    error: vi.fn(),
  };
  const api = defineApi({ logger: serverLogger }).route("/throws", {
    GET: defineHandler({ authorize: () => true }, get),
    POST: defineHandler({ authorize: () => true }, post),
  });
  const requestHandler = createRequestHandler(api);
  const clientLogger = {
    error: vi.fn(),
    info: vi.fn(),
  };
  let client = createFetchClient<typeof api.routes>("http://localhost", {
    fetch: (url, init) => requestHandler(new Request(url, init)),
    logger: clientLogger,
  });

  beforeEach(() => {
    client = createFetchClient<typeof api.routes>("http://localhost", {
      fetch: (url, init) => requestHandler(new Request(url, init)),
      logger: clientLogger,
    });
  });

  test("ErrorBoundary in Suspense", async () => {
    function Sut() {
      useQuery(client.throws.get());
      return "sut";
    }

    const view = await act(() =>
      render(
        <Suspense fallback="suspended">
          <ErrorBoundary fallback="fallback">
            <Sut />
          </ErrorBoundary>
        </Suspense>,
      ),
    );

    expect(view.getByText("fallback")).toBeVisible();
  });

  test("Suspense in ErrorBoundary", async () => {
    function Sut() {
      useQuery(client.throws.get());
      return "sut";
    }

    const view = await act(() =>
      render(
        <ErrorBoundary fallback="fallback">
          <Suspense fallback="suspended">
            <Sut />
          </Suspense>
        </ErrorBoundary>,
      ),
    );

    expect(view.getByText("fallback")).toBeVisible();
  });

  test("Error on revalidation", async () => {
    get.mockImplementation(async () => {
      return TResponse.json({ ok: true }, { cache: { tags: ["foo"] } });
    });
    function Sut() {
      const data = useQuery(client.throws.get());

      return data.ok ? "ok" : "nope";
    }

    const view = await act(() =>
      render(
        <Suspense fallback="suspended">
          <Sut />
        </Suspense>,
      ),
    );

    expect(view.getByText("ok")).toBeVisible();

    get.mockReset();

    post.mockImplementation(async () =>
      TResponse.void({ cache: { tags: ["foo"] } }),
    );

    await act(async () => {
      await client.throws.post().revalidated;
    });

    expect(clientLogger.error).toHaveBeenCalledWith(new HttpError(500, ""));
    expect(view.getByText("ok")).toBeVisible();

    await expect(client.throws.get()).rejects.toThrow();
  });
});
