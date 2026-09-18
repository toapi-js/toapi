import type { Observable } from "@toapi/common";
import type { PubSub } from "./pub-sub.js";
import type { Cache } from "./cache.js";
import { request } from "./request.js";

interface Options {
  fetch(url: string, init: RequestInit): Promise<Response>;
  url: string;
  init: RequestInit;
  pubSub: PubSub;
  cache: Cache;
}

interface Subscription {
  (data: Promise<unknown>): void;
}

export function buildObservable(
  options: Options,
): Observable<unknown> & Promise<unknown> {
  const { url, pubSub } = options;
  const { data } = request(options);

  function subscribe(callback: Subscription) {
    return pubSub.subscribe(async (invalidUrls) => {
      if (invalidUrls.has(url)) {
        const { data } = request(options);
        callback(data);
      }
    });
  }

  return Object.assign(data, {
    subscribe,
    queryKey: url,
  });
}
