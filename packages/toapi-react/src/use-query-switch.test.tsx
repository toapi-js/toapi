import { createFetchClient } from "@toapi/client";
import {
  createRequestHandler,
  defineApi,
  defineHandler,
  TResponse,
} from "@toapi/server";
import { act, render, screen } from "@testing-library/react";
import { Suspense } from "react";
import { describe, expect, test, vi } from "vitest";
import { z } from "zod/v4";
import { useQuery } from "./use-query.js";

/**
 * When the query changes, the hook must not keep showing the previous
 * query's data. `useQuery` holds the last value in a `useState`; that value
 * belongs to the *previous* observable and has to be discarded as soon as a
 * different one comes in — otherwise the component renders data for a
 * request it is no longer making.
 */
describe("useQuery on query change", () => {
  const api = defineApi().route("/thing", {
    GET: defineHandler(
      { authorize: () => true, query: { q: z.string() } },
      async (req) => {
        const { q } = req.query();
        return TResponse.json({ message: `value:${q}` });
      },
    ),
  });
  const handler = createRequestHandler(api);

  function Sut({ q }: { q: string }) {
    const data = useQuery(client.thing.get({ q }));
    return <div data-testid="sut">{data.message}</div>;
  }

  // The second request is held open so we can look at what the component
  // renders *while* the new query is still in flight.
  let hold: Promise<void> = Promise.resolve();
  const logger = {
    info: vi.fn(),
  };
  const client = createFetchClient<typeof api.routes>("http://localhost", {
    fetch: async (url, init) => {
      if (url.includes("q=second")) await hold;
      return handler(new Request(url, init));
    },
    logger,
  });

  test("does not render the previous query's data", async () => {
    const view = await act(() =>
      render(
        <Suspense fallback={<div data-testid="fallback">loading</div>}>
          <Sut q="first" />
        </Suspense>,
      ),
    );
    expect(screen.getByTestId("sut")).toHaveTextContent("value:first");

    let release!: () => void;
    hold = new Promise<void>((resolve) => {
      release = resolve;
    });

    await act(async () => {
      view.rerender(
        <Suspense fallback={<div data-testid="fallback">loading</div>}>
          <Sut q="second" />
        </Suspense>,
      );
    });

    // The new request has not resolved yet, so the only correct outcomes are
    // "suspended" or "already showing the new value" — never the old one.
    // React keeps the old subtree mounted but hides it while the boundary
    // falls back, so "not rendered" means "not visible" here.
    expect(screen.getByTestId("fallback")).toBeInTheDocument();
    expect(screen.getByTestId("sut")).not.toBeVisible();

    await act(async () => {
      release();
      await hold;
    });
    expect(screen.getByTestId("sut")).toHaveTextContent("value:second");
  });
});
