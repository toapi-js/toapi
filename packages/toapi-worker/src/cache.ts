import { EXPIRES_AT_HEADER, TAGS_HEADER } from "@toapi/common";
import { deleteDB, openDB } from "./idb.js";

const DB_NAME = "tapi-cache-meta";
const CACHE_NAME = "tapi-cache";
const DB_VERSION = 1;
export const META_STORE_NAME = "meta";
export const TAGS_STORE_NAME = "tags";

export interface CacheMeta {
  tags: string[];
  expiresAt: number | null;
}

let metaDb: Promise<IDBDatabase> | null = null;
let cache: Promise<Cache> | null = null;

export function openCacheMetaDB() {
  if (metaDb) return metaDb;

  metaDb = openDB(DB_NAME, DB_VERSION, (db) => {
    db.createObjectStore(META_STORE_NAME);
    db.createObjectStore(TAGS_STORE_NAME);
  });

  return metaDb;
}

export function openCache() {
  if (cache) return cache;

  cache = self.caches.open(CACHE_NAME);

  return cache;
}

export async function deleteCache() {
  metaDb = null;
  cache = null;
  await Promise.all([deleteDB(DB_NAME), self.caches.delete(CACHE_NAME)]);
}

export async function storeCacheEntry(req: Request, res: Response) {
  const [metaDb, cache] = await Promise.all([openCacheMetaDB(), openCache()]);

  const url = req.url;
  const expiresAt = res.headers.get(EXPIRES_AT_HEADER);
  const meta = {
    tags: res.headers.get(TAGS_HEADER)?.split(" ").filter(Boolean) ?? [],
    expiresAt: expiresAt ? parseInt(expiresAt, 10) : null,
  };

  await new Promise<void>(async (resolve, reject) => {
    const tx = metaDb.transaction(
      [META_STORE_NAME, TAGS_STORE_NAME],
      "readwrite",
    );
    tx.oncomplete = () => resolve();
    tx.onerror = () =>
      reject(new Error(`Failed to store metadata for "${url}"`));

    const metaStore = tx.objectStore(META_STORE_NAME);
    metaStore.put(meta, url);
    const tagsStore = tx.objectStore(TAGS_STORE_NAME);
    for (const tag of meta.tags) {
      const req = tagsStore.get(tag);
      req.onsuccess = () => {
        const urls: string[] = req.result ?? [];
        urls.push(url);
        tagsStore.put(urls, tag);
      };
    }
  });

  await cache.put(req, res.clone());
}

export async function deleteCacheEntry(req: Request) {
  const [metaDb, cache] = await Promise.all([openCacheMetaDB(), openCache()]);
  const url = req.url;

  await new Promise<void>((resolve, reject) => {
    const tx = metaDb.transaction(
      [META_STORE_NAME, TAGS_STORE_NAME],
      "readwrite",
    );
    tx.oncomplete = () => resolve();
    tx.onerror = () =>
      reject(new Error(`Failed to clear metadata for "${url}"`));

    const metaStore = tx.objectStore(META_STORE_NAME);
    const tagsStore = tx.objectStore(TAGS_STORE_NAME);

    const meta = metaStore.get(url);
    meta.onsuccess = () => {
      const tags: string[] = meta.result?.tags ?? [];

      for (const tag of tags) {
        const req = tagsStore.get(tag);
        req.onsuccess = () => {
          const urls: string[] = req.result ?? [];
          tagsStore.put(
            urls.filter((u) => u !== url),
            tag,
          );
        };
      }
    };
    metaStore.delete(url);
  });

  await cache.delete(req);
}

export async function getMetadata(url: string) {
  const db = await openCacheMetaDB();
  return new Promise<CacheMeta | null>((resolve, reject) => {
    const tx = db.transaction(META_STORE_NAME, "readonly");
    const metaStore = tx.objectStore(META_STORE_NAME);
    const req = metaStore.get(url);
    req.onsuccess = () => resolve(req.result ?? null);
    req.onerror = () =>
      reject(new Error(`Failed to retrieve metadata for "${url}"`));
  });
}

export async function getCachedEntry(req: Request) {
  const cache = await openCache();
  return cache.match(req);
}

export async function invalidateTags(tags: string[]) {
  const [metaDb, cache] = await Promise.all([openCacheMetaDB(), openCache()]);
  return new Promise<void>((resolve, reject) => {
    const deletes: Promise<boolean>[] = [];
    const tx = metaDb.transaction(
      [TAGS_STORE_NAME, META_STORE_NAME],
      "readwrite",
    );
    tx.oncomplete = async () => {
      await Promise.all(deletes);
      resolve();
    };
    tx.onerror = () =>
      reject(new Error(`Failed to invalidate tags ${tags.join(", ")}`));
    const tagsStore = tx.objectStore(TAGS_STORE_NAME);
    const metaStore = tx.objectStore(META_STORE_NAME);
    for (const tag of tags) {
      const tagRequest = tagsStore.get(tag);
      tagRequest.onsuccess = () => {
        const urls: string[] = tagRequest.result ?? [];
        for (const url of urls) {
          metaStore.delete(url);
          deletes.push(cache.delete(url));
        }
        tagsStore.delete(tag);
      };
    }
  });
}

export async function expireAll() {
  const metaDb = await openCacheMetaDB();

  return new Promise<void>((resolve, reject) => {
    const tx = metaDb.transaction([META_STORE_NAME], "readwrite");
    tx.oncomplete = () => resolve();
    tx.onerror = () => reject(new Error(`Failed to expire all cache entries`));

    const metaStore = tx.objectStore(META_STORE_NAME);
    metaStore.clear();
  });
}
