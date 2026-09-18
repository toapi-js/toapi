import {
  CONNECT_POST_EVENT,
  INVALIDATION_POST_EVENT,
  TAGS_CONTENT_TYPE,
  type Logger,
} from "@toapi/common";
import { deleteCache, expireAll, invalidateTags } from "./cache.js";
import { consoleFallback } from "./console-fallback.js";

declare const self: ServiceWorkerGlobalScope;

interface Options {
  url: string;
  timeout?: number;
  logger?: Logger;
}

export async function listenForInvalidations({
  url,
  timeout = 5000,
  logger: customLogger,
}: Options) {
  const logger = consoleFallback(customLogger);
  logger.info("Listening for invalidations...");

  let res: Response | null = null;
  const MAX_ATTEMPTS = 1000;
  for (let retry = 0; retry < MAX_ATTEMPTS; retry++) {
    try {
      res = await fetch(url);
      break;
    } catch (error) {
      logger.warn(
        `Failed attempt #${retry + 1} to open invalidation stream\n${String(error)}`,
      );
    }
    await new Promise((resolve) =>
      setTimeout(resolve, 500 * Math.pow(2, retry)),
    );
  }

  if (!res) {
    logger.error(
      new Error(
        `Failed to open invalidation stream after ${MAX_ATTEMPTS} attempts, giving up.`,
      ),
    );
    return;
  }

  const contentType = res.headers.get("Content-Type");
  if (!res.ok || contentType !== TAGS_CONTENT_TYPE || !res.body) {
    logger.error(
      new Error(
        `Failed to open invalidation stream: Received ${res.status} ${
          res.statusText
        } with content type ${contentType}. Cleaning up and unregistering service worker.`,
      ),
    );
    await deleteCache();
    await self.registration.unregister();
    return;
  }

  logger.info("Invalidation Stream Connection Established");

  try {
    await expireAll();
    const clients = await self.clients.matchAll();
    for (const client of clients) {
      client.postMessage({ type: CONNECT_POST_EVENT });
    }
    logger.info("Marked all cached entries as expired");
  } catch (error) {
    const formattedError =
      error instanceof Error ? `${error.name} ${error.message}` : String(error);
    logger.warn(`Failed to expire existing cache entries: ${formattedError}`);
  }

  try {
    let buffer = "";
    const decoder = new TextDecoder();
    for await (const chunk of res.body) {
      buffer += decoder.decode(chunk);
      const lines = buffer.split("\n");
      buffer = lines.pop() || "";

      const clients = await self.clients.matchAll();

      for (const line of lines) {
        const rawTags = line.trim();
        if (!rawTags) continue;
        const tags = rawTags.split(" ");
        logger.info(`Remote-Invalidating tags: ${tags}`);
        await invalidateTags(tags);
        for (const client of clients) {
          client.postMessage({ type: INVALIDATION_POST_EVENT, tags });
        }
      }
    }
  } catch (error) {
    logger.warn(
      `Invalidation stream failure, retrying in ${Math.round(timeout / 1000)}s`,
    );
    setTimeout(() => {
      listenForInvalidations({ url, timeout: Math.round(timeout * 1.5) });
    }, timeout);
  }
}
