import {
  SESSION_COOKIE_NAME,
  TAGS_CONTENT_TYPE,
  type MaybePromise,
} from "@toapi/common";
import type { Cache } from "./cache.js";
import { hashEncoder, textEncoder } from "./hash-tags.js";

const DEFAULT_KEEPALIVE_INTERVAL = 10 * 1000;
const DEFAULT_THROTTLE_TIMEOUT = 500;

export interface RevalidationStreamConfig {
  throttleTimeout?: number;
  keepaliveInterval?: number;
  filter?: (req: Request) => MaybePromise<(tag: string) => boolean>;
  hashSecret?: string;
}

interface Options {
  cache: Cache;
  config?: RevalidationStreamConfig;
  req: Request;
}

export function streamRevalidatedTags({ cache, config = {}, req }: Options) {
  const {
    throttleTimeout = DEFAULT_THROTTLE_TIMEOUT,
    keepaliveInterval = DEFAULT_KEEPALIVE_INTERVAL,
    filter = () => () => true,
  } = config;
  const id = crypto.randomUUID();
  let interval: ReturnType<typeof setInterval> | null = null;
  let timeout: ReturnType<typeof setTimeout> | null = null;
  let unsubscribe = () => {};
  const stream = new ReadableStream({
    async start(controller) {
      const filterTag = await filter(req);
      let queue = new Set<string>();
      const encode = config.hashSecret
        ? hashEncoder(config.hashSecret)
        : textEncoder();

      // subscribe to tag invalidations
      unsubscribe = cache.subscribe(async (tags, meta) => {
        // ignore our own invalidations
        if (
          meta &&
          typeof meta === "object" &&
          "clientId" in meta &&
          meta.clientId === id
        )
          return;

        for (const tag of tags) {
          if (filterTag(tag)) {
            queue.add(tag);
          }
        }

        // send tags to client
        if (queue.size > 0 && !timeout) {
          controller.enqueue(await encode(Array.from(queue)));
          queue = new Set();

          timeout = setTimeout(async () => {
            controller.enqueue(await encode(Array.from(queue)));
            queue = new Set();
            timeout = null;
          }, throttleTimeout);
        }
      });

      // keepalive. The first one is sent right away so the response headers
      // are flushed immediately instead of only once the interval first
      // fires — consumers skip empty lines, so this is a no-op for them.
      const keepalive = () => controller.enqueue(new Uint8Array([10]));
      keepalive();
      interval = setInterval(keepalive, keepaliveInterval);
    },
    cancel() {
      if (interval) clearInterval(interval);
      if (timeout) clearTimeout(timeout);
      unsubscribe();
    },
  });

  const headers = new Headers({
    "Set-Cookie": `${SESSION_COOKIE_NAME}=${id}; Path=/; HttpOnly; SameSite=Strict`,
    "Content-Type": TAGS_CONTENT_TYPE,
  });

  return new Response(stream, {
    headers,
  });
}
