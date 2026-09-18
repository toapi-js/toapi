import { extractMetadata } from "./extract-metadata.js";
import { handleResponse } from "./handle-response.js";
import type { Cache, CacheEntry } from "./cache.js";

interface Options {
  fetch(url: string, init: RequestInit): Promise<Response>;
  url: string;
  cache: Cache;
  init: RequestInit;
}

export function request(options: Options): CacheEntry {
  const { fetch, url, cache, init } = options;

  const cached = cache.get(url);
  if (cached) return cached;

  const response = fetch(url, init);
  const data = response.then(handleResponse);
  const meta = response.then(extractMetadata);

  cache.set(url, {
    response,
    data,
    meta,
  });

  return {
    response,
    data,
    meta,
  };
}
