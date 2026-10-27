//! Server-side response caches for Toapi: implementations of
//! [`toapi_core::Cache`] to pass to `Api::cache`.
//!
//! ```ignore
//! Api::new("Users", "1.0.0").cache(Arc::new(InMemoryCache::new()))
//! ```

use std::{
    collections::HashSet,
    future::ready,
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};

use dashmap::DashMap;
use toapi_core::{BoxFuture, Cache, CacheEntry, Invalidation, Lookup};
use tokio::sync::broadcast;

struct Stored {
    entry: CacheEntry,
    expires: Instant,
}

/// A [`Cache`] that keeps responses in memory, for a single server process.
pub struct InMemoryCache {
    entries: DashMap<String, Stored>,
    /// The generation at which each tag was last invalidated.
    invalidated_at: DashMap<String, u64>,
    /// Incremented by every invalidation; lookups return it as their token.
    generation: AtomicU64,
    /// Counts stores, to drop expired entries every now and then.
    stores: AtomicU64,
    sender: broadcast::Sender<Invalidation>,
}

impl InMemoryCache {
    pub fn new() -> Self {
        Self::with_capacity(1024)
    }

    /// `capacity` is how many invalidations a slow stream may fall behind
    /// before it is closed (the client then reconnects and drops its cache).
    pub fn with_capacity(capacity: usize) -> Self {
        InMemoryCache {
            entries: DashMap::new(),
            invalidated_at: DashMap::new(),
            generation: AtomicU64::new(0),
            stores: AtomicU64::new(0),
            sender: broadcast::channel(capacity).0,
        }
    }
}

impl Default for InMemoryCache {
    fn default() -> Self {
        Self::new()
    }
}

impl Cache for InMemoryCache {
    fn get<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Lookup> {
        // read the generation first: an invalidation after this point must
        // keep the response of this lookup from being stored
        let token = self.generation.load(Ordering::SeqCst);
        let now = Instant::now();
        // the read reference must be dropped before removing from the same
        // shard, or `remove_if` deadlocks on it
        let hit = self
            .entries
            .get(key)
            .map(|stored| (stored.expires > now).then(|| stored.entry.clone()));
        let lookup = match hit {
            Some(Some(entry)) => Lookup::Hit(entry),
            Some(None) => {
                self.entries
                    .remove_if(key, |_, stored| stored.expires <= now);
                Lookup::Miss(token)
            }
            None => Lookup::Miss(token),
        };
        Box::pin(ready(lookup))
    }

    fn set(&self, key: String, entry: CacheEntry, ttl: Duration, token: u64) -> BoxFuture<'_, ()> {
        let stale = entry.tags.iter().any(|tag| {
            self.invalidated_at
                .get(tag)
                .is_some_and(|generation| *generation > token)
        });
        if !stale {
            self.entries.insert(
                key,
                Stored {
                    entry,
                    expires: Instant::now() + ttl,
                },
            );
        }
        if self.stores.fetch_add(1, Ordering::Relaxed) % 256 == 255 {
            let now = Instant::now();
            self.entries.retain(|_, stored| stored.expires > now);
        }
        Box::pin(ready(()))
    }

    fn invalidate(&self, tags: Vec<String>, origin: Option<String>) -> BoxFuture<'_, ()> {
        let generation = self.generation.fetch_add(1, Ordering::SeqCst) + 1;
        for tag in &tags {
            self.invalidated_at.insert(tag.clone(), generation);
        }
        let invalidated: HashSet<&String> = tags.iter().collect();
        self.entries.retain(|_, stored| {
            !stored
                .entry
                .tags
                .iter()
                .any(|tag| invalidated.contains(tag))
        });
        // no receivers is fine: nobody is listening
        let _ = self.sender.send(Invalidation { tags, origin });
        Box::pin(ready(()))
    }

    fn subscribe(&self) -> broadcast::Receiver<Invalidation> {
        self.sender.subscribe()
    }
}

#[cfg(test)]
mod tests {
    use bytes::Bytes;

    use super::*;

    fn entry(tags: &[&str]) -> CacheEntry {
        CacheEntry {
            body: Bytes::from_static(b"[]"),
            tags: tags.iter().map(|t| t.to_string()).collect(),
            expires_at: None,
        }
    }

    fn token(lookup: Lookup) -> u64 {
        match lookup {
            Lookup::Miss(token) => token,
            Lookup::Hit(_) => panic!("expected a miss"),
        }
    }

    #[tokio::test]
    async fn stores_and_invalidates() {
        let cache = InMemoryCache::new();
        let mut subscription = cache.subscribe();
        let t = token(cache.get("GET /users").await);
        cache
            .set(
                "GET /users".into(),
                entry(&["users"]),
                Duration::from_secs(60),
                t,
            )
            .await;
        assert!(matches!(cache.get("GET /users").await, Lookup::Hit(_)));

        cache
            .invalidate(vec!["users".into()], Some("session".into()))
            .await;
        assert!(matches!(cache.get("GET /users").await, Lookup::Miss(_)));
        let invalidation = subscription.recv().await.unwrap();
        assert_eq!(invalidation.tags, vec!["users"]);
        assert_eq!(invalidation.origin.as_deref(), Some("session"));
    }

    #[tokio::test]
    async fn skips_responses_older_than_an_invalidation() {
        let cache = InMemoryCache::new();
        let t = token(cache.get("GET /users").await);
        // the handler runs, then a mutation invalidates the tag
        cache.invalidate(vec!["users".into()], None).await;
        cache
            .set(
                "GET /users".into(),
                entry(&["users"]),
                Duration::from_secs(60),
                t,
            )
            .await;
        assert!(matches!(cache.get("GET /users").await, Lookup::Miss(_)));

        // responses for other tags are stored
        cache
            .set(
                "GET /posts".into(),
                entry(&["posts"]),
                Duration::from_secs(60),
                t,
            )
            .await;
        assert!(matches!(cache.get("GET /posts").await, Lookup::Hit(_)));
    }

    #[tokio::test]
    async fn expires_entries() {
        let cache = InMemoryCache::new();
        let t = token(cache.get("GET /users").await);
        cache
            .set(
                "GET /users".into(),
                entry(&[]),
                Duration::from_millis(20),
                t,
            )
            .await;
        assert!(matches!(cache.get("GET /users").await, Lookup::Hit(_)));
        tokio::time::sleep(Duration::from_millis(40)).await;
        assert!(matches!(cache.get("GET /users").await, Lookup::Miss(_)));
    }
}
