import { type Logger } from "@toapi/common";
import type { Metadata } from "./extract-metadata.js";
import type { PubSub } from "./pub-sub.js";

export interface CacheEntry {
  response: Promise<Response>;
  meta: Promise<Metadata>;
  data: Promise<unknown>;
}

interface Options {
  maxOverdueTTL?: number;
  logger?: Logger;
  pubSub: PubSub;
}

const DEFAULT_MAX_OVERDUE_TTL = 1000;

export class Cache {
  private storage = new Map<string, CacheEntry>();
  private tagIndex = new Map<string, Set<string>>();
  private maxOverdueTTL: number;
  private errorLog: (error: unknown) => void | Promise<void>;
  private pubSub: PubSub;

  constructor(options: Options) {
    this.maxOverdueTTL = options.maxOverdueTTL ?? DEFAULT_MAX_OVERDUE_TTL;
    this.errorLog = options.logger?.error ?? console.error;
    this.pubSub = options.pubSub;
  }

  public set(url: string, entry: CacheEntry) {
    this.storage.set(url, entry);

    entry.data.catch((error) => {
      this.storage.delete(url);
      return this.errorLog(error);
    });

    entry.meta
      .then(({ tags, expiresAt }) => {
        for (const tag of tags) {
          let urls = this.tagIndex.get(tag);
          if (!urls) {
            urls = new Set();
            this.tagIndex.set(tag, urls);
          }
          urls.add(url);
        }

        if (!expiresAt) return;
        const timeUntilRevalidation =
          expiresAt -
          Date.now() +
          Math.round(Math.random() * this.maxOverdueTTL);
        setTimeout(
          () => {
            const cached = this.storage.get(url);
            if (cached !== entry) return;
            this.storage.delete(url);
            this.pubSub.publish(new Set([url]));
          },
          Math.max(0, timeUntilRevalidation),
        );
      })
      .catch(() => {});
  }

  public get(url: string) {
    return this.storage.get(url);
  }

  public invalidateUrl(url: string) {
    this.storage.delete(url);
    return this.pubSub.publish(new Set([url]));
  }

  public invalidateTags(tags: string[]) {
    const urls = new Set<string>();
    for (const tag of tags) {
      const taggedUrls = this.tagIndex.get(tag);
      if (!taggedUrls) continue;
      for (const url of taggedUrls) {
        if (this.storage.has(url)) {
          this.storage.delete(url);
        } else {
          taggedUrls.delete(url);
        }
        urls.add(url);
      }
    }
    return this.pubSub.publish(urls);
  }

  public invalidateAll() {
    const urls = new Set(this.storage.keys());
    this.storage.clear();
    this.tagIndex.clear();
    return this.pubSub.publish(urls);
  }
}
