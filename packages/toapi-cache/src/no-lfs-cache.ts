import {
  GetObjectCommand,
  PutObjectCommand,
  type S3Client,
} from "@aws-sdk/client-s3";
import type { Cache, CacheEntry, Json, Subscription } from "./index.js";

interface Options {
  cutoffBytes?: number;
}

export class NoLfsCache implements Cache {
  cutoffBytes: number;

  constructor(
    private base: Cache,
    options?: Options,
  ) {
    this.cutoffBytes = options?.cutoffBytes ?? 1_000_000;
  }

  get(key: string) {
    return this.base.get(key);
  }

  set(input: CacheEntry & { key: string; ttl: number; tags: string[] }) {
    if (input.attachment && input.attachment.byteLength > this.cutoffBytes) {
      return Promise.resolve();
    }

    return this.base.set(input);
  }

  delete(tags: string[], meta?: { clientId?: string }) {
    return this.invalidate(tags, meta);
  }
  invalidate(tags: string[], meta?: { clientId?: string }) {
    return this.base.invalidate(tags);
  }
  subscribe(callback: Subscription) {
    return this.base.subscribe(callback);
  }
}
