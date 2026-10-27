use std::time::{Duration, SystemTime, UNIX_EPOCH};

use axum::{
    Json,
    response::{IntoResponse, Response},
};
use serde::Serialize;

/// What the tag middleware does with the tags of a response.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TagKind {
    /// GET: tell the client which tags the response depends on.
    Cache,
    /// Mutation: tell every client that these tags are stale.
    Invalidate,
}

/// Response extension read by the tag middleware and the response cache.
#[derive(Debug, Clone)]
pub(crate) struct TagInfo {
    pub kind: TagKind,
    pub tags: Vec<String>,
    /// Milliseconds since the Unix epoch, sent as `X-TAPI-Expires-At`.
    pub expires_at: Option<u64>,
}

/// Milliseconds since the Unix epoch.
pub(crate) fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64)
}

/// The response of a GET handler: JSON data plus the cache tags it depends on.
///
/// A response with tags or a TTL is also stored in the server's
/// [`Cache`](crate::Cache), under the request's URL, until its TTL (or the
/// API's default TTL) runs out or one of its tags is invalidated. A response
/// with neither, e.g. one that depends on who asks, is never stored.
///
/// ```ignore
/// Ok(Cached::new(user).tags([format!("user:{id}"), "users".into()]).ttl(Duration::from_secs(60)))
/// ```
#[derive(Debug, Clone)]
pub struct Cached<T> {
    pub value: T,
    pub tags: Vec<String>,
    pub ttl: Option<Duration>,
}

impl<T> Cached<T> {
    pub fn new(value: T) -> Self {
        Cached {
            value,
            tags: Vec::new(),
            ttl: None,
        }
    }

    /// Adds tags; clients refetch the response once any of them is invalidated.
    pub fn tags<I>(mut self, tags: I) -> Self
    where
        I: IntoIterator,
        I::Item: Into<String>,
    {
        self.tags.extend(tags.into_iter().map(Into::into));
        self
    }

    /// How long the response may be served from the server's and the
    /// clients' caches.
    pub fn ttl(mut self, ttl: Duration) -> Self {
        self.ttl = Some(ttl);
        self
    }
}

impl<T: Serialize> IntoResponse for Cached<T> {
    fn into_response(self) -> Response {
        let mut res = Json(self.value).into_response();
        res.extensions_mut().insert(TagInfo {
            kind: TagKind::Cache,
            tags: self.tags,
            expires_at: self.ttl.map(|ttl| now_ms() + ttl.as_millis() as u64),
        });
        res
    }
}

/// The response of a mutation handler: JSON data plus the tags it invalidates.
///
/// ```ignore
/// Ok(Invalidates::new(user).tags(["users"]))
/// ```
#[derive(Debug, Clone)]
pub struct Invalidates<T> {
    pub value: T,
    pub tags: Vec<String>,
}

impl<T> Invalidates<T> {
    pub fn new(value: T) -> Self {
        Invalidates {
            value,
            tags: Vec::new(),
        }
    }

    /// Adds tags to invalidate once the handler succeeded.
    pub fn tags<I>(mut self, tags: I) -> Self
    where
        I: IntoIterator,
        I::Item: Into<String>,
    {
        self.tags.extend(tags.into_iter().map(Into::into));
        self
    }
}

impl<T: Serialize> IntoResponse for Invalidates<T> {
    fn into_response(self) -> Response {
        let mut res = Json(self.value).into_response();
        res.extensions_mut().insert(TagInfo {
            kind: TagKind::Invalidate,
            tags: self.tags,
            expires_at: None,
        });
        res
    }
}
