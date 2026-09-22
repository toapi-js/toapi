import { createFetchClient } from "@toapi/client";
import type { Observable } from "@toapi/common";
import {
  createRequestHandler,
  defineApi,
  defineHandler,
  PubSub,
  TResponse,
} from "@toapi/server";
import {
  act,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { StrictMode, Suspense, useLayoutEffect, useState } from "react";
import { describe, expect, test, vi } from "vitest";
import { useQuery } from "./use-query.js";

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((done) => {
    resolve = done;
  });
  return { promise, resolve };
}

function setup() {
  const fetch = vi.fn(async () => Response.json("first"));
  const route = {
    GET: defineHandler({ authorize: () => true }, async () =>
      TResponse.json("first"),
    ),
  };
  const api = defineApi()
    .route("/item", route)
    .route("/parent", route)
    .route("/child", route);
  const client = createFetchClient<typeof api.routes>("http://localhost", {
    fetch,
    invalidationsUrl: false,
  });
  return { client, fetch };
}

describe("query identity during revalidation", () => {
  test.each(["factory", "promise"] as const)(
    "%s queries keep content and local state on urgent renders during a refresh",
    async (kind) => {
      const { client, fetch } = setup();
      const mounted = vi.fn();
      const cleanedUp = vi.fn();
      function View() {
        const value = useQuery(
          kind === "factory" ? () => client.item.get() : client.item.get(),
        );
        const [draft, setDraft] = useState("");
        useLayoutEffect(() => {
          mounted();
          return cleanedUp;
        }, []);
        return (
          <>
            <p>{String(value)}</p>
            <input
              aria-label="Draft"
              value={draft}
              onChange={(e) => setDraft(e.target.value)}
            />
          </>
        );
      }
      await act(() =>
        render(
          <StrictMode>
            <Suspense fallback={<p>Loading</p>}>
              <View />
            </Suspense>
          </StrictMode>,
        ),
      );
      const input = screen.getByRole("textbox");
      input.focus();
      const mounts = mounted.mock.calls.length;
      const cleanups = cleanedUp.mock.calls.length;
      const pending = deferred<Response>();
      fetch.mockImplementationOnce(() => pending.promise);
      let refresh!: Promise<void>;
      act(() => {
        refresh = client.item.revalidate();
      });
      act(() => {
        fireEvent.change(input, { target: { value: "unsent" } });
      });

      expect(screen.queryByText("Loading")).not.toBeInTheDocument();
      expect(screen.getByText("first")).toBeVisible();
      expect(input).toHaveValue("unsent");
      expect(input).toHaveFocus();
      expect(mounted).toHaveBeenCalledTimes(mounts);
      expect(cleanedUp).toHaveBeenCalledTimes(cleanups);

      await act(async () => {
        pending.resolve(Response.json("updated"));
        await refresh;
      });
      expect(screen.getByText("updated")).toBeVisible();
      expect(screen.getByRole("textbox")).toBe(input);
      expect(input).toHaveValue("unsent");
    },
  );

  test("a parent's completed refresh does not hide a still-refreshing child", async () => {
    const { client, fetch } = setup();
    function Child() {
      return <p>child:{String(useQuery(() => client.child.get()))}</p>;
    }
    function Parent() {
      const value = useQuery(() => client.parent.get());
      return (
        <>
          <p>parent:{String(value)}</p>
          <Child />
        </>
      );
    }
    await act(() =>
      render(
        <Suspense fallback={<p>Loading</p>}>
          <Parent />
        </Suspense>,
      ),
    );
    const pending = deferred<Response>();
    fetch.mockImplementationOnce(() => pending.promise);
    let childRefresh!: Promise<void>;
    await act(() => {
      childRefresh = client.child.revalidate();
    });
    fetch.mockResolvedValueOnce(Response.json("updated"));
    await act(async () => {
      await client.parent.revalidate();
    });
    expect(screen.getByText("parent:updated")).toBeVisible();
    expect(screen.getByText("child:first")).toBeVisible();
    expect(screen.queryByText("Loading")).not.toBeInTheDocument();
    await act(async () => {
      pending.resolve(Response.json("updated"));
      await childRefresh;
    });
    expect(screen.getByText("child:updated")).toBeVisible();
  });

  test("a late refresh from the previous query cannot overwrite the selected query", async () => {
    const { client, fetch } = setup();
    function View({ selected }: { selected: "item" | "child" }) {
      return <p>{useQuery(() => client[selected].get())}</p>;
    }
    const tree = (selected: "item" | "child") => (
      <Suspense fallback={<p>Loading</p>}>
        <View selected={selected} />
      </Suspense>
    );
    const view = await act(() => render(tree("item")));
    const pending = deferred<Response>();
    fetch.mockImplementationOnce(() => pending.promise);
    let refresh!: Promise<void>;
    await act(() => {
      refresh = client.item.revalidate();
    });
    fetch.mockResolvedValueOnce(Response.json("selected"));
    await act(() => {
      view.rerender(tree("child"));
    });
    expect(screen.getByText("selected")).toBeVisible();
    await act(async () => {
      pending.resolve(Response.json("obsolete"));
      await refresh;
    });
    expect(screen.getByText("selected")).toBeVisible();
    expect(screen.queryByText("obsolete")).not.toBeInTheDocument();
  });

  test("re-suspends only when the queryKey actually changes", async () => {
    const cache = new PubSub();
    const itemHandler = vi.fn(async () =>
      TResponse.json("item" as string, { cache: { tags: ["item"] } }),
    );
    const otherHandler = vi.fn(async () =>
      TResponse.json("other" as string, { cache: { tags: ["other"] } }),
    );
    const api = defineApi({ cache })
      .route("/item", {
        GET: defineHandler({ authorize: () => true }, itemHandler),
      })
      .route("/other", {
        GET: defineHandler({ authorize: () => true }, otherHandler),
      });
    const requestHandler = createRequestHandler(api);
    const logger = {
      info: vi.fn(),
    };
    const client = createFetchClient<typeof api.routes>("http://localhost", {
      fetch: (url, init) => requestHandler(new Request(url, init)),
      logger,
    });
    await waitFor(() =>
      expect(logger.info).toHaveBeenCalledWith(
        "Invalidations stream connection established",
      ),
    );

    type Query = Promise<string> & Observable<string>;
    function View({ query }: { query: Query }) {
      return <p>{useQuery(query)}</p>;
    }
    const tree = (query: Query) => (
      <Suspense fallback={<p>Loading</p>}>
        <View query={query} />
      </Suspense>
    );

    // suspend initially
    const pendingFirst = deferred<string>();
    itemHandler.mockImplementationOnce(async () => {
      return TResponse.json(await pendingFirst.promise, {
        cache: { tags: ["item"] },
      });
    });
    const view = await act(() => render(tree(client.item.get())));
    expect(screen.getByText("Loading")).toBeVisible();
    await act(async () => {
      pendingFirst.resolve("first");
      await client.item.get();
    });
    expect(itemHandler).toHaveBeenCalledTimes(1);
    expect(screen.getByText("first")).toBeVisible();

    // wait for the invalidation stream to connect before invalidating tags
    await vi.waitFor(() => expect(cache.subscribers.size).toBeGreaterThan(0));

    // the route is revalidated: invalidating the tag via the server's PubSub
    // propagates to the client and the component updates without suspending.
    const pendingSecond = deferred<string>();
    itemHandler.mockImplementationOnce(async () => {
      return TResponse.json(await pendingSecond.promise, {
        cache: { tags: ["item"] },
      });
    });
    await act(async () => {
      await cache.delete(["item"]);
    });
    expect(screen.queryByText("Loading")).not.toBeInTheDocument();

    await act(async () => pendingSecond.resolve("updated"));
    expect(await screen.findByText("updated")).toBeVisible();

    // never re-suspend otherwise: a re-render passes a brand new observable
    // for the same query.
    await act(() => view.rerender(tree(client.item.get())));
    expect(screen.queryByText("Loading")).not.toBeInTheDocument();
    expect(screen.getByText("updated")).toBeVisible();

    // re-suspend when the queryKey changes: a genuinely different query must
    // still show the fallback while its own promise is pending.
    const pendingOther = deferred<string>();
    otherHandler.mockImplementationOnce(async () => {
      return TResponse.json(await pendingOther.promise, {
        cache: { tags: ["other"] },
      });
    });
    const other = client.other.get();
    await act(() => view.rerender(tree(other)));
    expect(screen.getByText("Loading")).toBeVisible();
    await act(async () => {
      pendingOther.resolve("other");
    });
    expect(screen.getByText("other")).toBeVisible();
  });
});
