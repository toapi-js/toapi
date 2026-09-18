import { createFetchClient } from "@toapi/client";
import { defineApi, defineHandler, TResponse } from "@toapi/server";
import { act, fireEvent, render, screen } from "@testing-library/react";
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

  test("custom observables without queryKey still use promise identity", async () => {
    let current: Promise<string> = Promise.resolve("first");
    const subscribe = (callback: (next: Promise<string>) => void) => {
      callback(current);
      return () => {};
    };
    const first = Object.assign(current, { subscribe });
    const next = deferred<string>();
    const second = Object.assign(next.promise, { subscribe });
    function View({ query }: { query: typeof first }) {
      return <p>{useQuery(query)}</p>;
    }
    const tree = (query: typeof first) => (
      <Suspense fallback={<p>Loading</p>}>
        <View query={query} />
      </Suspense>
    );
    const view = await act(() => render(tree(first)));
    current = second;
    await act(() => view.rerender(tree(second)));
    expect(screen.getByText("Loading")).toBeVisible();
    expect(screen.getByText("first")).not.toBeVisible();
    await act(async () => {
      next.resolve("second");
      await next.promise;
    });
    expect(screen.getByText("second")).toBeVisible();
  });
});
