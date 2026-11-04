import type { Pool, PoolClient, Notification } from "pg";
import type { Cache, CacheEntry, Json, Subscription } from "./index.js";

const CHANNEL = "tag_based_cache_invalidate";

const MIN_GC_TIMEOUT = 5 * 1000;
const MAX_GC_TIMEOUT = 5 * 60 * 1000;
const GC_TIMEOUT_STEP = 0.1;

interface InvalidationMessage {
  tags: string[];
  meta?: Json;
}

/** Quote a Postgres identifier so it can be safely interpolated into SQL. */
function quoteIdentifier(name: string): string {
  return `"${name.replace(/"/g, '""')}"`;
}

export interface PostgresCacheOptions {
  /**
   * When `true` (the default) the required tables and indexes are created
   * lazily on the first operation via `CREATE TABLE IF NOT EXISTS`. Set to
   * `false` if you manage the schema yourself (e.g. via migrations).
   */
  createSchema?: boolean;

  /**
   * The Postgres schema to hold the cache tables. Defaults to whatever the
   * connection's `search_path` resolves to (usually `public`). When set, the
   * tables are fully qualified as `<schema>.cache_entries` /
   * `<schema>.cache_tags` and — with `createSchema` enabled — the schema is
   * created if it does not already exist. Each schema also gets its own
   * invalidation channel so caches on different schemas do not cross-notify.
   */
  schema?: string;
}

export class PostgresCache implements Cache {
  subscriptions = new Set<Subscription>();
  listener?: PoolClient;
  gcTimeout = MIN_GC_TIMEOUT;

  private ready: Promise<void>;
  private gcTimer?: ReturnType<typeof setTimeout>;
  private stopped = false;

  /** Fully-qualified (schema-prefixed when applicable) table identifiers. */
  private readonly entriesTable: string;
  private readonly tagsTable: string;
  /** Invalidation channel, namespaced by schema so schemas stay isolated. */
  private readonly channel: string;

  constructor(
    private pool: Pool,
    private options: PostgresCacheOptions = {}
  ) {
    const { schema } = options;
    const prefix = schema ? `${quoteIdentifier(schema)}.` : "";
    this.entriesTable = `${prefix}cache_entries`;
    this.tagsTable = `${prefix}cache_tags`;
    this.channel = schema ? `${CHANNEL}:${schema}` : CHANNEL;

    this.ready = this.setup();
  }

  private async setup(): Promise<void> {
    if (this.options.createSchema !== false) {
      if (this.options.schema) {
        await this.pool.query(
          `CREATE SCHEMA IF NOT EXISTS ${quoteIdentifier(this.options.schema)}`
        );
      }

      await this.pool.query(`
        CREATE TABLE IF NOT EXISTS ${this.entriesTable} (
          key TEXT PRIMARY KEY,
          data JSONB,
          attachment BYTEA,
          added_at BIGINT NOT NULL,
          expires_at BIGINT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS ${this.tagsTable} (
          key TEXT NOT NULL REFERENCES ${this.entriesTable}(key) ON DELETE CASCADE,
          tag TEXT NOT NULL,
          PRIMARY KEY (key, tag)
        );
        CREATE INDEX IF NOT EXISTS cache_tags_tag_idx ON ${this.tagsTable} (tag);
        CREATE INDEX IF NOT EXISTS cache_entries_expires_at_idx
          ON ${this.entriesTable} (expires_at);
      `);
    }

    this.gcTimer = setTimeout(() => this.gc(), this.gcTimeout);
  }

  async get(key: string): Promise<CacheEntry | null> {
    await this.ready;

    const { rows } = await this.pool.query(
      `
        SELECT data, attachment
        FROM ${this.entriesTable}
        WHERE key = $1 AND expires_at >= $2
      `,
      [key, Date.now()]
    );

    const row = rows[0];
    if (!row) return null;

    return {
      data: row.data ?? null,
      attachment: row.attachment ? new Uint8Array(row.attachment) : null,
    };
  }

  async set({
    key,
    data,
    attachment,
    ttl,
    tags,
  }: CacheEntry & {
    key: string;
    ttl: number;
    tags: string[];
  }): Promise<void> {
    await this.ready;

    const client = await this.pool.connect();
    try {
      await client.query("BEGIN");

      await client.query(
        `
          INSERT INTO ${this.entriesTable} (key, data, attachment, added_at, expires_at)
          VALUES ($1, $2, $3, $4, $5)
          ON CONFLICT (key) DO UPDATE SET
            data = EXCLUDED.data,
            attachment = EXCLUDED.attachment,
            added_at = EXCLUDED.added_at,
            expires_at = EXCLUDED.expires_at
        `,
        [
          key,
          data ? JSON.stringify(data) : null,
          attachment ? Buffer.from(attachment) : null,
          Date.now(),
          Date.now() + ttl * 1000,
        ]
      );

      // Replace the full tag set so tags removed since the last `set` are
      // dropped, matching the `INSERT OR REPLACE` semantics of the SQLite
      // implementations.
      await client.query(`DELETE FROM ${this.tagsTable} WHERE key = $1`, [key]);

      for (const tag of tags) {
        await client.query(
          `INSERT INTO ${this.tagsTable} (key, tag) VALUES ($1, $2)
             ON CONFLICT DO NOTHING`,
          [key, tag]
        );
      }

      await client.query("COMMIT");
    } catch (e) {
      try {
        await client.query("ROLLBACK");
      } catch {}
      throw e;
    } finally {
      client.release();
    }
  }

  delete(tags: string[], meta?: Json): Promise<void> {
    return this.invalidate(tags, meta);
  }

  private async withDeletionLock<T>(
    operation: (client: PoolClient) => Promise<T>
  ): Promise<T> {
    const client = await this.pool.connect();
    try {
      await client.query("BEGIN");
      // Lock before touching entries, across every instance using this table.
      // Its OID also unifies explicit schemas and search_path-based access.
      await client.query(
        `SELECT pg_advisory_xact_lock(
          hashtext('toapi:cache_entries:delete'), $1::regclass::oid::integer
        )`,
        [this.entriesTable]
      );
      const result = await operation(client);
      await client.query("COMMIT");
      return result;
    } catch (error) {
      try {
        await client.query("ROLLBACK");
      } catch {}
      throw error;
    } finally {
      client.release();
    }
  }

  async invalidate(tags: string[], meta?: Json): Promise<void> {
    await this.ready;

    await this.withDeletionLock(async (client) => {
      // `cache_tags` rows are removed automatically via `ON DELETE CASCADE`.
      await client.query(
        `
          DELETE FROM ${this.entriesTable}
          WHERE key IN (SELECT key FROM ${this.tagsTable} WHERE tag = ANY($1))
        `,
        [tags]
      );

      // PostgreSQL delivers the notification only when the deletion commits.
      await client.query(`SELECT pg_notify($1, $2)`, [
        this.channel,
        JSON.stringify({ tags, meta } satisfies InvalidationMessage),
      ]);
    });
  }

  private async setupListener(): Promise<void> {
    const listener = await this.pool.connect();
    this.listener = listener;

    listener.on("notification", (msg: Notification) => {
      if (msg.channel !== this.channel || !msg.payload) return;
      const { tags, meta } = JSON.parse(msg.payload) as InvalidationMessage;
      for (const callback of this.subscriptions) {
        callback(tags, meta);
      }
    });

    // LISTEN does not accept a bound parameter; the channel name is quoted so
    // an arbitrary (schema-derived) identifier is interpolated safely.
    await listener.query(`LISTEN ${quoteIdentifier(this.channel)}`);
  }

  subscribe(callback: Subscription): () => void {
    if (this.subscriptions.size === 0) {
      this.setupListener();
    }
    this.subscriptions.add(callback);

    return () => {
      this.subscriptions.delete(callback);
      if (this.subscriptions.size === 0) {
        this.listener?.removeAllListeners("notification");
        this.listener?.release();
        this.listener = undefined;
      }
    };
  }

  private async gc(): Promise<void> {
    try {
      const result = await this.withDeletionLock((client) =>
        client.query(
          `DELETE FROM ${this.entriesTable} WHERE expires_at < $1`,
          [Date.now()]
        )
      );
      const changes = result.rowCount ?? 0;

      if (changes > 100) {
        this.gcTimeout = Math.max(
          Math.floor(this.gcTimeout * (1 - GC_TIMEOUT_STEP)),
          MIN_GC_TIMEOUT
        );
      } else if (changes < 10) {
        this.gcTimeout = Math.min(
          Math.floor(this.gcTimeout * (1 + GC_TIMEOUT_STEP)),
          MAX_GC_TIMEOUT
        );
      }
    } catch {
      // Ignore transient errors (e.g. the pool being drained on shutdown);
      // the next tick will retry.
    } finally {
      if (!this.stopped) {
        this.gcTimer = setTimeout(() => this.gc(), this.gcTimeout);
      }
    }
  }

  /**
   * Stops the background garbage collector and releases the dedicated
   * invalidation listener connection back to the pool. Call this before
   * shutting down so no timers or connections are left dangling. The
   * underlying pool is left untouched — the caller owns its lifecycle.
   */
  async close(): Promise<void> {
    this.stopped = true;
    if (this.gcTimer) {
      clearTimeout(this.gcTimer);
      this.gcTimer = undefined;
    }
    this.listener?.removeAllListeners("notification");
    this.listener?.release();
    this.listener = undefined;
    this.subscriptions.clear();
  }
}
