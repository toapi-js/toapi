import { createFetchClient, type GetRoute } from "@toapi/client";
import {
  createRequestHandler,
  defineApi,
  defineHandler,
  TResponse,
} from "@toapi/server";
import { act, render, screen } from "@testing-library/react";
import { Suspense } from "react";
import { beforeEach, describe, expect, test, vi } from "vitest";
import { z } from "zod/v4";
import { useQuery } from "./use-query.js";

describe("useQuery", () => {
  const api = defineApi()
    .route("/noQuery", {
      GET: defineHandler(
        {
          authorize: () => true,
        },
        async (req) => {
          return TResponse.json({ message: "No Query" });
        },
      ),
    })
    .route("/withQuery", {
      GET: defineHandler(
        {
          authorize: () => true,
          query: {
            q: z.string(),
          },
        },
        async (req) => {
          const { q } = req.query();
          return TResponse.json({ message: `Query: ${q}` });
        },
      ),
    });

  const handler = createRequestHandler(api);
  const logger = {
    info: vi.fn(),
  };
  let client: ReturnType<typeof createFetchClient<typeof api.routes>>;

  beforeEach(() => {
    client = createFetchClient<typeof api.routes>("http://localhost", {
      fetch: (url, init) => handler(new Request(url, init)),
      logger,
    });
  });

  test("Without Query", async () => {
    function Sut() {
      const data = useQuery(client.noQuery.get());
      return <div>{data.message}</div>;
    }

    await act(() =>
      render(
        <Suspense fallback={<div>Loading...</div>}>
          <Sut />
        </Suspense>,
      ),
    );

    expect(screen.getByText("No Query")).toBeInTheDocument();
  });

  test("With Query", async () => {
    function Sut() {
      const data = useQuery(client.withQuery.get({ q: "test" }));
      return <div>{data.message}</div>;
    }

    await act(() =>
      render(
        <Suspense fallback={<div>Loading...</div>}>
          <Sut />
        </Suspense>,
      ),
    );

    expect(screen.getByText("Query: test")).toBeInTheDocument();
  });

  describe("Route as Prop", () => {
    test("Without Query", async () => {
      interface Props {
        route: {
          get: GetRoute<{ message: string }>;
        };
      }
      function Sut({ route }: Props) {
        const data = useQuery(route.get());
        return <div>{data.message}</div>;
      }

      await act(() =>
        render(
          <Suspense fallback={<div>Loading...</div>}>
            <Sut route={client.noQuery} />
          </Suspense>,
        ),
      );

      expect(screen.getByText("No Query")).toBeInTheDocument();
    });

    test("With Query", async () => {
      interface Props {
        route: {
          get: GetRoute<{ message: string }, { q: string }>;
        };
      }
      function Sut({ route }: Props) {
        const data = useQuery(route.get({ q: "test" }));
        return <div>{data.message}</div>;
      }

      await act(() =>
        render(
          <Suspense fallback={<div>Loading...</div>}>
            <Sut route={client.withQuery} />
          </Suspense>,
        ),
      );

      expect(screen.getByText("Query: test")).toBeInTheDocument();
    });
  });

  describe("reactivity", () => {
    test("updates on revalidation", async () => {
      vi.useFakeTimers();
      const things: string[] = [];

      const get = vi.fn(async () => {
        return TResponse.json(things, { cache: { tags: ["things"] } });
      });
      const post = vi.fn(async (req) => {
        const { thing } = await req.data();
        things.push(thing);
        return TResponse.json(null, { cache: { tags: ["things"] } });
      });

      const api = defineApi().route("/things", {
        GET: defineHandler(
          {
            authorize: () => true,
          },
          get,
        ),
        POST: defineHandler(
          {
            authorize: () => true,
            body: z.object({
              thing: z.string(),
            }),
          },
          post,
        ),
      });

      const handler = createRequestHandler(api);
      const client = createFetchClient<typeof api.routes>(
        "http://localhost:3000",
        {
          fetch: (url, init) => handler(new Request(url, init)),
          logger,
          invalidationsUrl: false,
        },
      );

      function Sut() {
        const data = useQuery(client.things.get());

        return <div data-testid="sut">{JSON.stringify(data)}</div>;
      }

      expect(get).not.toHaveBeenCalled();
      const screen = await act(() => render(<Sut />));
      expect(get).toHaveBeenCalledTimes(1);

      expect(screen.getByTestId("sut")).toHaveTextContent("[]");

      await act(async () => {
        await client.things.post({ thing: "test" }).revalidated;
      });
      expect(post).toHaveBeenCalled();
      expect(get).toHaveBeenCalledTimes(2);

      expect(screen.getByTestId("sut")).toHaveTextContent('["test"]');

      await act(async () => {
        await client.things.post({ thing: "foo" }).revalidated;
        await vi.advanceTimersByTimeAsync(150);
      });

      expect(screen.getByTestId("sut")).toHaveTextContent('["test","foo"]');
    });
  });
});
