import { randomUUID } from "node:crypto";
import { Pool } from "pg";
import {
  afterAll,
  afterEach,
  beforeAll,
  beforeEach,
  describe,
  expect,
  test,
} from "vitest";
import { PostgresCache } from "./postgres-cache.js";

const POSTGRES_URL =
  process.env.POSTGRES_URL ??
  "postgres://postgres:postgres@localhost:5432/postgres";

// Skip the suite when no Postgres is reachable (e.g. local runs without
// `docker compose up`). CI provides a Postgres service so it always runs there.
async function isPostgresAvailable(connectionString: string): Promise<boolean> {
  const pool = new Pool({ connectionString, connectionTimeoutMillis: 1000 });
  try {
    await pool.query("SELECT 1");
    return true;
  } catch {
    return false;
  } finally {
    await pool.end().catch(() => {});
  }
}

const postgresAvailable = await isPostgresAvailable(POSTGRES_URL);

describe.skipIf(!postgresAvailable)("PostgresCache", () => {
  const pool = new Pool({ connectionString: POSTGRES_URL });

  const caches: PostgresCache[] = [];

  function createCache() {
    const cache = new PostgresCache(pool);
    caches.push(cache);
    return cache;
  }

  beforeAll(async () => {
    // Ensure the schema exists before the per-test truncation runs.
    await new PostgresCache(pool)["ready"];
  });

  beforeEach(async () => {
    await pool.query("TRUNCATE cache_entries, cache_tags");
  });

  afterEach(async () => {
    await Promise.all(caches.splice(0).map((cache) => cache.close()));
  });

  afterAll(async () => {
    await pool.end();
  });

  test("basic store and retrieve", async () => {
    const sut = createCache();

    await sut.set({
      key: "test",
      data: {
        foo: 1,
        bar: "baz",
      },
      ttl: 1000,
      tags: [],
    });

    expect(await sut.get("test")).toEqual({
      data: {
        foo: 1,
        bar: "baz",
      },
      attachment: null,
    });
  });

  test("returns null for missing key", async () => {
    const sut = createCache();

    expect(await sut.get("nope")).toEqual(null);
  });

  test("expire by ttl", async () => {
    const sut = createCache();

    await sut.set({
      key: "test",
      data: {
        foo: 1,
        bar: "baz",
      },
      ttl: 1,
      tags: [],
    });

    // Wait for the TTL to expire.
    await new Promise((resolve) => setTimeout(resolve, 1100));

    expect(await sut.get("test")).toEqual(null);
  });

  test("expire by tags", async () => {
    const sut = createCache();

    await sut.set({
      key: "test",
      data: {
        foo: 1,
        bar: "baz",
      },
      ttl: 1000,
      tags: ["tag1", "tag2"],
    });

    await sut.set({
      key: "test2",
      data: {
        foo: 2,
        bar: "qux",
      },
      ttl: 1000,
      tags: ["tag2", "tag3"],
    });

    // Delete all entries with tag1
    await sut.delete(["tag1"]);

    // test should be deleted
    expect(await sut.get("test")).toEqual(null);

    // test2 should still exist
    expect(await sut.get("test2")).toEqual({
      data: {
        foo: 2,
        bar: "qux",
      },
      attachment: null,
    });

    // Delete all entries with tag2
    await sut.delete(["tag2"]);

    // Now test2 should also be deleted
    expect(await sut.get("test2")).toEqual(null);

    // The tag rows are cleaned up via ON DELETE CASCADE.
    const { rows } = await pool.query("SELECT * FROM cache_tags");
    expect(rows).toEqual([]);
  });

  test("store binary blob", async () => {
    const sut = createCache();

    const numbers = [1, 2, 3, 4, 5];

    await sut.set({
      key: "test",
      ttl: 1000,
      tags: [],
      attachment: new Uint8Array(numbers),
    });

    expect(await sut.get("test")).toEqual({
      data: null,
      attachment: new Uint8Array(numbers),
    });
  });

  test("store both data and attachment", async () => {
    const sut = createCache();

    const numbers = [1, 2, 3, 4, 5];

    await sut.set({
      key: "test",
      data: { message: "hello" },
      attachment: new Uint8Array(numbers),
      ttl: 1000,
      tags: ["tag1"],
    });

    expect(await sut.get("test")).toEqual({
      data: { message: "hello" },
      attachment: new Uint8Array(numbers),
    });
  });

  test("delete with multiple tags", async () => {
    const sut = createCache();

    await sut.set({
      key: "item1",
      data: { id: 1 },
      ttl: 1000,
      tags: ["user:1", "post:1"],
    });

    await sut.set({
      key: "item2",
      data: { id: 2 },
      ttl: 1000,
      tags: ["user:1", "post:2"],
    });

    await sut.set({
      key: "item3",
      data: { id: 3 },
      ttl: 1000,
      tags: ["user:2", "post:3"],
    });

    // Delete all items for user:1
    await sut.delete(["user:1"]);

    expect(await sut.get("item1")).toEqual(null);
    expect(await sut.get("item2")).toEqual(null);
    expect(await sut.get("item3")).toEqual({
      data: { id: 3 },
      attachment: null,
    });
  });

  test("concurrent overlapping invalidations do not expose deadlocks", async () => {
    const schema = `deadlock_${randomUUID().replaceAll("-", "").slice(0, 16)}`;
    // Both plans are valid for the production query. Force them here so one
    // DELETE visits insertion order (z, a), while the other visits key order
    // (a, z), instead of depending on the planner to choose them by chance.
    const firstPool = new Pool({
      connectionString: POSTGRES_URL,
      max: 1,
      options:
        "-c enable_mergejoin=off -c enable_nestloop=off -c statement_timeout=10000",
    });
    const secondPool = new Pool({
      connectionString: POSTGRES_URL,
      max: 1,
      options:
        "-c enable_hashjoin=off -c enable_nestloop=off -c statement_timeout=10000",
    });
    const controller = await pool.connect();
    const first = new PostgresCache(firstPool, { schema });
    let second: PostgresCache | undefined;
    const pending: Promise<void>[] = [];
    let outcomes: Promise<PromiseSettledResult<void>[]> | undefined;
    let gateLocked = false;

    try {
      await first.set({ key: "z", ttl: 1000, tags: ["left", "right"] });
      await first.set({ key: "a", ttl: 1000, tags: ["left", "right"] });
      second = new PostgresCache(secondPool, { schema, createSchema: false });
      await second.get("warmup");
      await controller.query(`ANALYZE "${schema}".cache_entries`);
      await controller.query(`ANALYZE "${schema}".cache_tags`);

      // Pause each DELETE after it locks its first entry. Shared advisory
      // locks allow both to proceed together once the controller opens the
      // gate; they do not themselves create a deadlock between the workers.
      await controller.query(`
        CREATE FUNCTION "${schema}".pause_delete() RETURNS trigger
        LANGUAGE plpgsql AS $$
        BEGIN
          PERFORM pg_advisory_xact_lock_shared(hashtext(TG_TABLE_SCHEMA));
          RETURN OLD;
        END;
        $$;
        CREATE TRIGGER pause_delete BEFORE DELETE ON "${schema}".cache_entries
        FOR EACH ROW EXECUTE FUNCTION "${schema}".pause_delete();
      `);
      await controller.query("SELECT pg_advisory_lock(hashtext($1))", [schema]);
      gateLocked = true;
      const firstPid = (await firstPool.query("SELECT pg_backend_pid() AS pid"))
        .rows[0].pid as number;
      const secondPid = (await secondPool.query("SELECT pg_backend_pid() AS pid"))
        .rows[0].pid as number;

      pending.push(first.delete(["left"]));
      // Attach rejection handlers immediately, including on early test exit.
      outcomes = Promise.allSettled(pending);
      const waitUntilBlocked = async (pid: number) => {
        await expect
          .poll(
            async () => {
              const { rows } = await controller.query(
                "SELECT cardinality(pg_blocking_pids($1)) > 0 AS blocked",
                [pid]
              );
              return rows[0].blocked;
            },
            { timeout: 5000, interval: 10 }
          )
          .toBe(true);
      };
      await waitUntilBlocked(firstPid);
      pending.push(second.invalidate(["right"]));
      outcomes = Promise.allSettled(pending);
      // With ordered entry locking, the second worker may instead wait on
      // the first worker before reaching the trigger. That is also valid.
      await waitUntilBlocked(secondPid);
      await controller.query("SELECT pg_advisory_unlock(hashtext($1))", [schema]);
      gateLocked = false;

      expect(await outcomes).toEqual([
        { status: "fulfilled", value: undefined },
        { status: "fulfilled", value: undefined },
      ]);
      expect((await controller.query(`SELECT * FROM "${schema}".cache_entries`)).rows)
        .toEqual([]);
      expect((await controller.query(`SELECT * FROM "${schema}".cache_tags`)).rows)
        .toEqual([]);
    } finally {
      if (gateLocked) {
        await controller.query("SELECT pg_advisory_unlock(hashtext($1))", [schema]);
      }
      await outcomes;
      await first.close();
      await second?.close();
      await Promise.all([firstPool.end(), secondPool.end()]);
      await controller.query(`DROP SCHEMA IF EXISTS "${schema}" CASCADE`);
      controller.release();
    }
  }, 20000);

  test("set replaces stale tags", async () => {
    const sut = createCache();

    await sut.set({ key: "k", data: { v: 1 }, ttl: 1000, tags: ["t1", "t2"] });
    // Re-set with a smaller tag set: t2 should no longer point at "k".
    await sut.set({ key: "k", data: { v: 2 }, ttl: 1000, tags: ["t1"] });

    await sut.delete(["t2"]);
    expect(await sut.get("k")).toEqual({ data: { v: 2 }, attachment: null });

    await sut.delete(["t1"]);
    expect(await sut.get("k")).toEqual(null);
  });

  test("delete propagates to subscribers across instances", async () => {
    const writer = createCache();
    const reader = createCache();

    const events: Array<{ tags: string[]; meta: unknown }> = [];
    reader.subscribe((tags, meta) => {
      events.push({ tags, meta });
    });

    // Give the dedicated LISTEN connection time to be established.
    await new Promise((resolve) => setTimeout(resolve, 200));

    await writer.delete(["tag1"], { clientId: "abc" });

    // Wait for the NOTIFY to be delivered.
    await new Promise((resolve) => setTimeout(resolve, 200));

    expect(events).toEqual([{ tags: ["tag1"], meta: { clientId: "abc" } }]);
  });

  describe("with a dedicated schema", () => {
    const schema = "cache_test_schema";

    // Start each test from a clean slate — this also recovers from a prior
    // run that was killed before it could clean up.
    beforeEach(async () => {
      await pool.query(`DROP SCHEMA IF EXISTS "${schema}" CASCADE`);
    });

    test("creates the schema and stores entries there", async () => {
      const sut = new PostgresCache(pool, { schema });
      caches.push(sut);

      await sut.set({ key: "k", data: { v: 1 }, ttl: 1000, tags: ["t1"] });

      expect(await sut.get("k")).toEqual({ data: { v: 1 }, attachment: null });

      // The tables really live in the dedicated schema.
      const { rows } = await pool.query(
        `SELECT to_regclass('${schema}.cache_entries') IS NOT NULL AS entries,
                to_regclass('${schema}.cache_tags') IS NOT NULL AS tags`
      );
      expect(rows[0]).toEqual({ entries: true, tags: true });

      // Tag invalidation works against the schema-qualified tables.
      await sut.delete(["t1"]);
      expect(await sut.get("k")).toEqual(null);
    });

    test("is isolated from the default schema", async () => {
      const scoped = new PostgresCache(pool, { schema });
      const shared = createCache();
      caches.push(scoped);

      await shared.set({ key: "k", data: { v: "public" }, ttl: 1000, tags: [] });
      await scoped.set({ key: "k", data: { v: "scoped" }, ttl: 1000, tags: [] });

      expect(await shared.get("k")).toEqual({
        data: { v: "public" },
        attachment: null,
      });
      expect(await scoped.get("k")).toEqual({
        data: { v: "scoped" },
        attachment: null,
      });
    });

    test("invalidations do not cross schema boundaries", async () => {
      const scoped = new PostgresCache(pool, { schema });
      const shared = createCache();
      caches.push(scoped);
      // Materialize the schema tables before the writer notifies.
      await scoped.get("warmup");

      const events: string[][] = [];
      shared.subscribe((tags) => events.push(tags));

      await new Promise((resolve) => setTimeout(resolve, 200));

      // A delete on the scoped cache uses a schema-specific channel, so the
      // default-schema subscriber must not receive it.
      await scoped.delete(["tag1"]);
      await new Promise((resolve) => setTimeout(resolve, 200));

      expect(events).toEqual([]);
    });
  });
});
