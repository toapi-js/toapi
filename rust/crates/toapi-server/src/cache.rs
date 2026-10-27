use std::{
    collections::HashSet,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

use axum::{
    extract::FromRequestParts,
    http::{StatusCode, header, request::Parts},
    response::{IntoResponse, Response},
};
use bytes::Bytes;
use dashmap::DashMap;
use futures::{FutureExt, future::BoxFuture};
use tokio::sync::broadcast;

use crate::response::{TagInfo, TagKind};

/// A set of invalidated tags, as the [`Cache`] hands it to its subscribers.
#[derive(Debug, Clone)]
pub struct Invalidation {
    pub tags: Vec<String>,
    /// The session of the client that caused the invalidation. Its own
    /// invalidation stream skips it, because the client already invalidated
    /// its cache when the mutation returned.
    pub origin: Option<String>,
}

/// A cached GET response.
#[derive(Debug, Clone)]
pub struct CacheEntry {
    /// The JSON body.
    pub body: Bytes,
    /// The tags the response depends on, before hashing.
    pub tags: Vec<String>,
    /// When clients should consider the response stale (milliseconds since
    /// the Unix epoch), if the handler set a TTL.
    pub expires_at: Option<u64>,
}

impl IntoResponse for CacheEntry {
    fn into_response(self) -> Response {
        let mut res = ([(header::CONTENT_TYPE, "application/json")], self.body).into_response();
        res.extensions_mut().insert(TagInfo {
            kind: TagKind::Cache,
            tags: self.tags,
            expires_at: self.expires_at,
        });
        res
    }
}

/// The result of [`Cache::get`].
#[derive(Debug, Clone)]
pub enum Lookup {
    Hit(CacheEntry),
    /// Not cached. The token goes back into [`Cache::set`], which must not
    /// store the response if any of its tags were invalidated since the
    /// lookup: the handler may have read the data before the invalidation.
    Miss(u64),
}

/// The server's cache, like the TypeScript server's `Cache`.
///
/// GET responses that set tags or a TTL are stored under their URL and
/// served from the cache to every request that passes the route's
/// `authorize`. Invalidating tags deletes the entries carrying them and
/// hands the tags to the subscribers, the invalidation streams.
///
/// [`PubSub`], the default, caches nothing and only passes invalidations
/// on; [`InMemoryCache`] caches within one process. For several processes,
/// implement it on top of Redis or Postgres.
pub trait Cache: Send + Sync + 'static {
    fn get<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Lookup>;

    /// Stores `entry` for `ttl`, unless one of its tags was invalidated
    /// since the lookup that returned `token`.
    fn set(&self, key: String, entry: CacheEntry, ttl: Duration, token: u64) -> BoxFuture<'_, ()>;

    /// Deletes the entries carrying any of `tags`, then hands the tags to
    /// the subscribers. `origin` is the session of the client that caused
    /// it, if any (the TypeScript interface's `meta.clientId`).
    fn invalidate(&self, tags: Vec<String>, origin: Option<String>) -> BoxFuture<'_, ()>;

    fn subscribe(&self) -> broadcast::Receiver<Invalidation>;
}

/// The default [`Cache`], like the TypeScript server's `PubSub`: caches
/// nothing and passes invalidations on to the subscribers in this process.
pub struct PubSub {
    sender: broadcast::Sender<Invalidation>,
}

impl PubSub {
    pub fn new() -> Self {
        Self::with_capacity(1024)
    }

    /// `capacity` is how many invalidations a slow stream may fall behind
    /// before it is closed (the client then reconnects and drops its cache).
    pub fn with_capacity(capacity: usize) -> Self {
        PubSub {
            sender: broadcast::channel(capacity).0,
        }
    }

    fn publish(&self, tags: Vec<String>, origin: Option<String>) {
        // no receivers is fine: nobody is listening
        let _ = self.sender.send(Invalidation { tags, origin });
    }
}

impl Default for PubSub {
    fn default() -> Self {
        Self::new()
    }
}

impl Cache for PubSub {
    fn get<'a>(&'a self, _: &'a str) -> BoxFuture<'a, Lookup> {
        futures::future::ready(Lookup::Miss(0)).boxed()
    }

    fn set(&self, _: String, _: CacheEntry, _: Duration, _: u64) -> BoxFuture<'_, ()> {
        futures::future::ready(()).boxed()
    }

    fn invalidate(&self, tags: Vec<String>, origin: Option<String>) -> BoxFuture<'_, ()> {
        self.publish(tags, origin);
        futures::future::ready(()).boxed()
    }

    fn subscribe(&self) -> broadcast::Receiver<Invalidation> {
        self.sender.subscribe()
    }
}

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
    pub_sub: PubSub,
}

impl InMemoryCache {
    pub fn new() -> Self {
        InMemoryCache {
            entries: DashMap::new(),
            invalidated_at: DashMap::new(),
            generation: AtomicU64::new(0),
            stores: AtomicU64::new(0),
            pub_sub: PubSub::new(),
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
        futures::future::ready(lookup).boxed()
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
        futures::future::ready(()).boxed()
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
        self.pub_sub.publish(tags, origin);
        futures::future::ready(()).boxed()
    }

    fn subscribe(&self) -> broadcast::Receiver<Invalidation> {
        self.pub_sub.subscribe()
    }
}

/// The [`Cache`] as seen from one request, as an extractor: invalidations
/// made through it are attributed to the calling client (like the
/// TypeScript server's `req.invalidate`), so that client isn't sent its own
/// invalidation over the invalidation stream.
#[derive(Clone)]
pub struct RequestCache {
    cache: Arc<dyn Cache>,
    origin: Option<String>,
}

impl RequestCache {
    pub(crate) fn new(cache: Arc<dyn Cache>, origin: Option<String>) -> Self {
        RequestCache { cache, origin }
    }

    pub async fn invalidate<I>(&self, tags: I)
    where
        I: IntoIterator,
        I::Item: Into<String>,
    {
        let tags: Vec<String> = tags.into_iter().map(Into::into).collect();
        if !tags.is_empty() {
            self.cache.invalidate(tags, self.origin.clone()).await;
        }
    }
}

impl<S: Send + Sync> FromRequestParts<S> for RequestCache {
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        parts.extensions.get::<RequestCache>().cloned().ok_or((
            StatusCode::INTERNAL_SERVER_ERROR,
            "RequestCache used outside of a toapi router",
        ))
    }
}

#[cfg(test)]
mod tests {
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
    async fn pub_sub_passes_invalidations_to_subscribers() {
        let cache = PubSub::new();
        let mut subscription = cache.subscribe();
        cache
            .invalidate(vec!["users".into()], Some("session".into()))
            .await;

        let invalidation = subscription.recv().await.unwrap();
        assert_eq!(invalidation.tags, vec!["users".to_string()]);
        assert_eq!(invalidation.origin.as_deref(), Some("session"));
    }

    #[tokio::test]
    async fn request_cache_attributes_invalidations_to_its_client() {
        let cache: Arc<dyn Cache> = Arc::new(PubSub::new());
        let mut subscription = cache.subscribe();
        RequestCache::new(cache.clone(), Some("session".into()))
            .invalidate(["users"])
            .await;
        // nothing to invalidate, nothing sent
        RequestCache::new(cache, None)
            .invalidate(Vec::<String>::new())
            .await;

        let invalidation = subscription.recv().await.unwrap();
        assert_eq!(invalidation.origin.as_deref(), Some("session"));
        assert!(subscription.try_recv().is_err());
    }

    #[tokio::test]
    async fn in_memory_cache_stores_and_invalidates() {
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

        cache.invalidate(vec!["users".into()], None).await;
        assert!(matches!(cache.get("GET /users").await, Lookup::Miss(_)));
        assert_eq!(subscription.recv().await.unwrap().tags, vec!["users"]);
    }

    #[tokio::test]
    async fn in_memory_cache_skips_responses_older_than_an_invalidation() {
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
    async fn in_memory_cache_expires_entries() {
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
