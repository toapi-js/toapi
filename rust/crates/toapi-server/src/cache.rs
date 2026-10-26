use std::sync::Arc;

use axum::{
    extract::FromRequestParts,
    http::{StatusCode, request::Parts},
};
use tokio::sync::broadcast;

/// A set of invalidated tags, as the [`Cache`] hands it to its subscribers.
#[derive(Debug, Clone)]
pub struct Invalidation {
    pub tags: Vec<String>,
    /// The session of the client that caused the invalidation. Its own
    /// invalidation stream skips it, because the client already invalidated
    /// its cache when the mutation returned.
    pub origin: Option<String>,
}

/// The server's cache, like the TypeScript server's `Cache`: it takes the
/// invalidated tags and hands them to its subscribers, the invalidation
/// streams.
///
/// It doesn't cache responses yet (the TypeScript interface's `get` and
/// `set`), so it only distributes invalidations. [`PubSub`] covers a single
/// server process; for several processes, implement it on top of Redis
/// pub/sub or Postgres `LISTEN/NOTIFY` and forward the messages into a
/// broadcast channel.
pub trait Cache: Send + Sync + 'static {
    /// Invalidates `tags`. `origin` is the session of the client that caused
    /// it, if any (the TypeScript interface's `meta.clientId`).
    fn invalidate(&self, tags: Vec<String>, origin: Option<String>);

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
}

impl Default for PubSub {
    fn default() -> Self {
        Self::new()
    }
}

impl Cache for PubSub {
    fn invalidate(&self, tags: Vec<String>, origin: Option<String>) {
        // no receivers is fine: nobody is listening
        let _ = self.sender.send(Invalidation { tags, origin });
    }

    fn subscribe(&self) -> broadcast::Receiver<Invalidation> {
        self.sender.subscribe()
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

    pub fn invalidate<I>(&self, tags: I)
    where
        I: IntoIterator,
        I::Item: Into<String>,
    {
        let tags: Vec<String> = tags.into_iter().map(Into::into).collect();
        if !tags.is_empty() {
            self.cache.invalidate(tags, self.origin.clone());
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

    #[tokio::test]
    async fn pub_sub_passes_invalidations_to_subscribers() {
        let cache = PubSub::new();
        let mut subscription = cache.subscribe();
        cache.invalidate(vec!["users".into()], Some("session".into()));

        let invalidation = subscription.recv().await.unwrap();
        assert_eq!(invalidation.tags, vec!["users".to_string()]);
        assert_eq!(invalidation.origin.as_deref(), Some("session"));
    }

    #[tokio::test]
    async fn request_cache_attributes_invalidations_to_its_client() {
        let cache: Arc<dyn Cache> = Arc::new(PubSub::new());
        let mut subscription = cache.subscribe();
        RequestCache::new(cache.clone(), Some("session".into())).invalidate(["users"]);
        // nothing to invalidate, nothing sent
        RequestCache::new(cache, None).invalidate(Vec::<String>::new());

        let invalidation = subscription.recv().await.unwrap();
        assert_eq!(invalidation.origin.as_deref(), Some("session"));
        assert!(subscription.try_recv().is_err());
    }
}
