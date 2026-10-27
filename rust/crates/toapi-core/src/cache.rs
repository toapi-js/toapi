//! The server cache interface, like the TypeScript `Cache`. Implemented by
//! `toapi-server`'s `PubSub` (the default) and `toapi-cache`'s caches.

use std::{future::Future, pin::Pin, time::Duration};

use bytes::Bytes;
use tokio::sync::broadcast;

/// A boxed, `Send` future, as returned by [`Cache`] methods.
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

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
/// `toapi_server::PubSub`, the default, caches nothing and only passes
/// invalidations on; `toapi_cache::InMemoryCache` caches within one process.
/// For several processes, implement it on top of Redis or Postgres.
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
