use std::{sync::Arc, time::Duration};

use axum::{
    extract::FromRequestParts,
    http::{StatusCode, request::Parts},
};
use futures::FutureExt;
use toapi_core::{BoxFuture, Cache, CacheEntry, Invalidation, Lookup};
use tokio::sync::broadcast;

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
}
