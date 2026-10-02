import {
  GetObjectCommand,
  PutObjectCommand,
  type S3Client,
} from "@aws-sdk/client-s3";
import type { Cache, CacheEntry, Json, Subscription } from "./index.js";

interface Options {
  cutoffBytes?: number;
  bucket?: string;
}

interface ObjectReference {
  k: string;
  v: string | null;
}

interface Data {
  p: Json | null;
  a: ObjectReference | null;
}

export class S3LfsCache implements Cache {
  bucket?: string;
  cutoffBytes: number;

  constructor(
    private base: Cache,
    private client: S3Client,
    options?: Options,
  ) {
    this.bucket = options?.bucket;
    this.cutoffBytes = options?.cutoffBytes ?? 1_000_000;
  }

  async get(key: string) {
    const entry = await this.base.get(key);

    const data = entry?.data;
    if (!data || typeof data !== "object" || !("p" in data) || !("a" in data))
      return null;

    entry.data = data.p;
    const attachment = data.a;

    if (
      attachment &&
      typeof attachment === "object" &&
      attachment !== null &&
      "k" in attachment &&
      "v" in attachment
    ) {
      const cmd = new GetObjectCommand({
        Bucket: this.bucket,
        Key: attachment.k?.toString(),
        VersionId: attachment.v?.toString() ?? undefined,
      });
      const response = await this.client.send(cmd);

      if (!response.Body) return null;

      entry.attachment = await response.Body.transformToByteArray();
    }

    return entry;
  }

  async set(input: CacheEntry & { key: string; ttl: number; tags: string[] }) {
    const data: Data = {
      p: input.data ?? null,
      a: null,
    };

    input = {
      ...input,
      data: data as any,
    };

    if (input.attachment && input.attachment.byteLength > this.cutoffBytes) {
      const cmd = new PutObjectCommand({
        Bucket: this.bucket,
        Key: input.key,
        Body: input.attachment,
      });
      const response = await this.client.send(cmd);
      input.attachment = null;
      data.a = {
        k: input.key,
        v: response.VersionId ?? null,
      };
    }

    await this.base.set(input);
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
