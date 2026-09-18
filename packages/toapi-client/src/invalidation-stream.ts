import {
  INVALIDATION_POST_EVENT,
  CONNECT_POST_EVENT,
  type Logger,
} from "@toapi/common";

interface Options {
  fetch: (url: string, init: RequestInit) => Promise<Response>;
  invalidationsUrl: string | null;
  onInvalidate(tags: string[]): Promise<void>;
  onConnect(): Promise<void>;
  logger?: Logger;
}

export function listenForInvalidations(options: Options) {
  const { onInvalidate, onConnect, logger } = options;

  const warn = logger?.warn ?? console.warn;

  if ("serviceWorker" in navigator) {
    navigator.serviceWorker.addEventListener("message", async (event) => {
      if (
        typeof event.data === "object" &&
        event.data !== null &&
        "type" in event.data
      ) {
        switch (event.data.type) {
          case INVALIDATION_POST_EVENT: {
            try {
              await onInvalidate(event.data.tags);
            } catch (error) {
              warn(
                `TApi: Failed to revalidate tags: received invalid post message ${JSON.stringify(event.data)}`,
              );
            }
            return;
          }
          case CONNECT_POST_EVENT: {
            await onConnect();
            return;
          }
        }
      }
    });
    if (!navigator.serviceWorker.controller) {
      fetchInvalidationStream(options);
    }
  } else if (typeof window !== "undefined") {
    fetchInvalidationStream(options);
  }
}

async function fetchInvalidationStream({
  fetch,
  onInvalidate,
  invalidationsUrl,
  onConnect,
  logger,
}: Options) {
  if (!invalidationsUrl) return;

  const warn = logger?.warn ?? console.warn;

  const MAX_ATTEMPTS = 500;
  for (let retry = 0; retry < MAX_ATTEMPTS; retry++) {
    try {
      const res = await fetch(invalidationsUrl, {});
      if (!res.ok) throw `${res.status} ${res.statusText}`;
      if (!res.body) {
        throw `Invalidation stream response has no body`;
      }

      // reset retry counter
      retry = 0;

      // invalidate everything in the cache, it might have gone stale while we were not listening
      await onConnect();

      let buffer = "";
      const decoder = new TextDecoder();
      for await (const chunk of res.body) {
        buffer += decoder.decode(chunk);
        const lines = buffer.split("\n");
        buffer = lines.pop() ?? "";
        for (const line of lines) {
          const rawTags = line.trim();
          if (!rawTags) continue;
          await onInvalidate(rawTags.split(" "));
        }
      }
    } catch (error) {
      const formattedError =
        error instanceof Error
          ? `${error.name} ${error.message}`
          : String(error);
      warn(
        `Reconnecting to invalidation stream after error: ${formattedError}`,
      );
    } finally {
      // error or stream ended, retry with exponential backoff
      await new Promise((resolve) =>
        // so the retry interval is roughly
        // 0.55s, 0.61s, 0.66s, 0.73s
        // growing exponentially up to 1.9hours and then stays constant
        // for about 63 days before it throws an error
        setTimeout(resolve, 500 * Math.pow(1.1, Math.min(retry, 100))),
      );
    }
  }

  throw new Error("Failed to reconnect to invalidation stream");
}
