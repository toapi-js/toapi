import { randomUUID } from "node:crypto";
import {
  CreateBucketCommand,
  GetObjectCommand,
  HeadObjectCommand,
  S3Client,
} from "@aws-sdk/client-s3";
import { beforeAll, beforeEach, describe, expect, test } from "vitest";
import { InMemoryCache } from "./in-memory-cache.js";
import { S3LfsCache } from "./s3-lfs-cache.js";

const S3_URL = process.env.S3_URL ?? "http://localhost:9000";
const S3_ACCESS_KEY_ID = process.env.S3_ACCESS_KEY_ID ?? "toapi";
const S3_SECRET_ACCESS_KEY = process.env.S3_SECRET_ACCESS_KEY ?? "toapi-secret";
const BUCKET = "toapi-cache-test";
const CUTOFF = 16;

// Skip the suite when no S3 server is reachable (e.g. local runs without
// `docker compose up`). CI provides an S3 service so it always runs there.
async function isS3Available(url: string): Promise<boolean> {
  try {
    await fetch(url, { signal: AbortSignal.timeout(1000) });
    return true;
  } catch {
    return false;
  }
}

const s3Available = await isS3Available(S3_URL);

function bytes(length: number, seed = 0) {
  return Uint8Array.from({ length }, (_, i) => (i + seed) % 256);
}

describe.skipIf(!s3Available)("S3LfsCache", () => {
  const client = new S3Client({
    endpoint: S3_URL,
    region: "us-east-1",
    forcePathStyle: true,
    credentials: {
      accessKeyId: S3_ACCESS_KEY_ID,
      secretAccessKey: S3_SECRET_ACCESS_KEY,
    },
  });

  let base: InMemoryCache;
  let sut: S3LfsCache;
  // The bucket outlives a test run, so every test gets its own key namespace.
  let prefix: string;

  beforeAll(async () => {
    try {
      await client.send(new CreateBucketCommand({ Bucket: BUCKET }));
    } catch (error) {
      if (
        !(error instanceof Error) ||
        (error.name !== "BucketAlreadyOwnedByYou" &&
          error.name !== "BucketAlreadyExists")
      ) {
        throw error;
      }
    }
  });

  beforeEach(() => {
    base = new InMemoryCache();
    sut = new S3LfsCache(base, client, { bucket: BUCKET, cutoffBytes: CUTOFF });
    prefix = `${randomUUID()}/`;
  });

  async function getObject(key: string) {
    const response = await client.send(
      new GetObjectCommand({ Bucket: BUCKET, Key: key }),
    );
    return response.Body?.transformToByteArray();
  }

  async function objectExists(key: string) {
    try {
      await client.send(new HeadObjectCommand({ Bucket: BUCKET, Key: key }));
      return true;
    } catch (error) {
      if (error instanceof Error && error.name === "NotFound") return false;
      throw error;
    }
  }

  test("returns null for unknown keys", async () => {
    expect(await sut.get(`${prefix}missing`)).toEqual(null);
  });

  test("basic store and retrieve", async () => {
    const key = `${prefix}test`;

    await sut.set({
      key,
      data: { foo: 1, bar: "baz" },
      ttl: 1000,
      tags: [],
    });

    expect(await sut.get(key)).toEqual({
      data: { foo: 1, bar: "baz" },
      attachment: null,
    });
    expect(await objectExists(key)).toBe(false);
  });

  test("keeps small attachments in the base cache", async () => {
    const key = `${prefix}small`;
    const attachment = bytes(CUTOFF - 1);

    await sut.set({ key, attachment, ttl: 1000, tags: [] });

    expect(await sut.get(key)).toEqual({ data: null, attachment });
    expect((await base.get(key))?.attachment).toEqual(attachment);
    expect(await objectExists(key)).toBe(false);
  });

  test("keeps attachments of exactly cutoffBytes in the base cache", async () => {
    const key = `${prefix}boundary`;
    const attachment = bytes(CUTOFF);

    await sut.set({ key, attachment, ttl: 1000, tags: [] });

    expect(await sut.get(key)).toEqual({ data: null, attachment });
    expect(await objectExists(key)).toBe(false);
  });

  test("offloads large attachments to s3", async () => {
    const key = `${prefix}large`;
    const attachment = bytes(CUTOFF * 64);

    await sut.set({ key, attachment, ttl: 1000, tags: [] });

    expect(await sut.get(key)).toEqual({ data: null, attachment });
    expect(await getObject(key)).toEqual(attachment);
    expect((await base.get(key))?.attachment).toBeNull();
  });

  test("store both data and large attachment", async () => {
    const key = `${prefix}both`;
    const attachment = bytes(CUTOFF * 64);

    await sut.set({
      key,
      data: { message: "hello" },
      attachment,
      ttl: 1000,
      tags: ["tag1"],
    });

    expect(await sut.get(key)).toEqual({
      data: { message: "hello" },
      attachment,
    });
  });

  test("does not mutate the input entry", async () => {
    const attachment = bytes(CUTOFF * 64);
    const input = {
      key: `${prefix}input`,
      data: { message: "hello" },
      attachment,
      ttl: 1000,
      tags: [],
    };

    await sut.set(input);

    expect(input.data).toEqual({ message: "hello" });
    expect(input.attachment).toBe(attachment);
  });

  test("overwriting a key returns the latest attachment", async () => {
    const key = `${prefix}overwrite`;

    await sut.set({ key, attachment: bytes(CUTOFF * 64), ttl: 1000, tags: [] });
    await sut.set({
      key,
      attachment: bytes(CUTOFF * 32, 7),
      ttl: 1000,
      tags: [],
    });

    expect((await sut.get(key))?.attachment).toEqual(bytes(CUTOFF * 32, 7));
  });

  test("overwriting a large attachment with a small one", async () => {
    const key = `${prefix}shrink`;

    await sut.set({ key, attachment: bytes(CUTOFF * 64), ttl: 1000, tags: [] });
    await sut.set({ key, attachment: bytes(4), ttl: 1000, tags: [] });

    expect(await sut.get(key)).toEqual({ data: null, attachment: bytes(4) });
  });

  test("expire by ttl", async () => {
    const key = `${prefix}ttl`;

    await sut.set({
      key,
      data: { foo: 1 },
      attachment: bytes(CUTOFF * 64),
      ttl: 1,
      tags: [],
    });

    // margin over the 1s TTL to avoid a boundary race on loaded CI runners
    await new Promise((resolve) => setTimeout(resolve, 1500));

    expect(await sut.get(key)).toEqual(null);
  });

  test("expire by tags", async () => {
    const first = `${prefix}first`;
    const second = `${prefix}second`;

    await sut.set({
      key: first,
      data: { foo: 1 },
      attachment: bytes(CUTOFF * 64),
      ttl: 1000,
      tags: ["tag1", "tag2"],
    });
    await sut.set({
      key: second,
      data: { foo: 2 },
      attachment: bytes(CUTOFF * 64, 1),
      ttl: 1000,
      tags: ["tag2", "tag3"],
    });

    await sut.invalidate(["tag1"]);

    expect(await sut.get(first)).toEqual(null);
    expect(await sut.get(second)).toEqual({
      data: { foo: 2 },
      attachment: bytes(CUTOFF * 64, 1),
    });

    await sut.delete(["tag2"]);

    expect(await sut.get(second)).toEqual(null);
  });

  test("forwards invalidations to subscribers", async () => {
    const calls: string[][] = [];
    const unsubscribe = sut.subscribe((tags) => {
      calls.push(tags);
    });

    await sut.invalidate(["tag1"]);
    unsubscribe();
    await sut.invalidate(["tag2"]);

    expect(calls).toEqual([["tag1"]]);
  });

  test("ignores base entries not written by S3LfsCache", async () => {
    const key = `${prefix}foreign`;

    await base.set({ key, data: { foo: 1 }, ttl: 1000, tags: [] });

    expect(await sut.get(key)).toEqual(null);
  });
});
