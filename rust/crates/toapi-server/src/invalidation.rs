use std::{
    collections::BTreeSet,
    convert::Infallible,
    future::Future,
    pin::Pin,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use axum::{
    body::Body,
    extract::{FromRequestParts, Request, State},
    http::{HeaderMap, HeaderValue, StatusCode, header, request::Parts},
    middleware::Next,
    response::{IntoResponse, Response},
};
use bytes::Bytes;
use sha2::{Digest, Sha256};
use toapi_core::{EXPIRES_AT_HEADER, SESSION_COOKIE_NAME, TAGS_CONTENT_TYPE, TAGS_HEADER};
use tokio::sync::{broadcast, mpsc};
use tokio_stream::wrappers::ReceiverStream;

use crate::response::{TagInfo, TagKind};

/// A set of invalidated tags.
#[derive(Debug, Clone)]
pub struct Invalidation {
    pub tags: Vec<String>,
    /// The session of the client that caused the invalidation. Its own
    /// invalidation stream skips it, because the client already invalidated
    /// its cache when the mutation returned.
    pub origin: Option<String>,
}

/// Distributes invalidations to every invalidation stream.
///
/// [`InMemoryBus`] covers a single server process; for several processes,
/// implement this on top of Redis pub/sub or Postgres `LISTEN/NOTIFY` and
/// forward the messages into a broadcast channel.
pub trait InvalidationBus: Send + Sync + 'static {
    fn publish(&self, invalidation: Invalidation);
    fn subscribe(&self) -> broadcast::Receiver<Invalidation>;
}

/// An [`InvalidationBus`] for a single process, backed by a broadcast channel.
pub struct InMemoryBus {
    sender: broadcast::Sender<Invalidation>,
}

impl InMemoryBus {
    pub fn new() -> Self {
        Self::with_capacity(1024)
    }

    /// `capacity` is how many invalidations a slow stream may fall behind
    /// before it is closed (the client then reconnects and drops its cache).
    pub fn with_capacity(capacity: usize) -> Self {
        InMemoryBus {
            sender: broadcast::channel(capacity).0,
        }
    }
}

impl Default for InMemoryBus {
    fn default() -> Self {
        Self::new()
    }
}

impl InvalidationBus for InMemoryBus {
    fn publish(&self, invalidation: Invalidation) {
        // no receivers is fine: nobody is listening
        let _ = self.sender.send(invalidation);
    }

    fn subscribe(&self) -> broadcast::Receiver<Invalidation> {
        self.sender.subscribe()
    }
}

/// Invalidates tags from inside a handler or from a background task.
///
/// As an extractor it remembers the calling client's session, so that client
/// isn't sent its own invalidation.
#[derive(Clone)]
pub struct Invalidator {
    bus: Arc<dyn InvalidationBus>,
    origin: Option<String>,
}

impl Invalidator {
    pub(crate) fn new(bus: Arc<dyn InvalidationBus>, origin: Option<String>) -> Self {
        Invalidator { bus, origin }
    }

    pub fn invalidate<I>(&self, tags: I)
    where
        I: IntoIterator,
        I::Item: Into<String>,
    {
        let tags: Vec<String> = tags.into_iter().map(Into::into).collect();
        if !tags.is_empty() {
            self.bus.publish(Invalidation {
                tags,
                origin: self.origin.clone(),
            });
        }
    }
}

impl<S: Send + Sync> FromRequestParts<S> for Invalidator {
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        parts.extensions.get::<Invalidator>().cloned().ok_or((
            StatusCode::INTERNAL_SERVER_ERROR,
            "Invalidator used outside of a toapi router",
        ))
    }
}

pub(crate) type TagFilter = Arc<dyn Fn(&str) -> bool + Send + Sync>;
pub(crate) type FilterFactory =
    Arc<dyn Fn(Parts) -> Pin<Box<dyn Future<Output = TagFilter> + Send>> + Send + Sync>;

pub(crate) struct StreamConfig {
    pub bus: Arc<dyn InvalidationBus>,
    pub throttle: Duration,
    pub keepalive: Duration,
    pub filter: Option<FilterFactory>,
    pub hash_secret: Option<String>,
}

impl StreamConfig {
    fn encode(&self, tags: &[String]) -> Vec<String> {
        match &self.hash_secret {
            Some(secret) => tags.iter().map(|tag| hash_tag(secret, tag)).collect(),
            None => tags.to_vec(),
        }
    }
}

/// SHA-256 of `secret + tag`, first 8 bytes as a big endian u64, in base 36.
/// Identical to the TypeScript implementation, so both clients understand it.
pub(crate) fn hash_tag(secret: &str, tag: &str) -> String {
    let digest = Sha256::new()
        .chain_update(secret.as_bytes())
        .chain_update(tag.as_bytes())
        .finalize();
    let mut n = u64::from_be_bytes(digest[..8].try_into().unwrap());
    if n == 0 {
        return "0".into();
    }
    let mut digits = Vec::new();
    while n > 0 {
        digits.push(char::from_digit((n % 36) as u32, 36).unwrap());
        n /= 36;
    }
    digits.iter().rev().collect()
}

fn session_id(headers: &HeaderMap) -> Option<String> {
    headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(';'))
        .find_map(|cookie| {
            let (name, value) = cookie.trim().split_once('=')?;
            (name == SESSION_COOKIE_NAME).then(|| value.to_string())
        })
}

/// Adds the tag headers to tagged responses and publishes the tags of
/// successful mutations.
pub(crate) async fn tag_middleware(
    State(config): State<Arc<StreamConfig>>,
    mut req: Request,
    next: Next,
) -> Response {
    let origin = session_id(req.headers());
    req.extensions_mut()
        .insert(Invalidator::new(config.bus.clone(), origin.clone()));
    let mut res = next.run(req).await;
    let Some(info) = res.extensions_mut().remove::<TagInfo>() else {
        return res;
    };
    if !info.tags.is_empty()
        && let Ok(value) = HeaderValue::from_str(&config.encode(&info.tags).join(" "))
    {
        res.headers_mut().insert(TAGS_HEADER, value);
    }
    if let Some(ttl) = info.ttl {
        let expires_at = SystemTime::now()
            .checked_add(ttl)
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|t| t.as_millis());
        if let Some(expires_at) = expires_at {
            res.headers_mut()
                .insert(EXPIRES_AT_HEADER, HeaderValue::from(expires_at as u64));
        }
    }
    if info.kind == TagKind::Invalidate && res.status().is_success() && !info.tags.is_empty() {
        config.bus.publish(Invalidation {
            tags: info.tags,
            origin,
        });
    }
    res
}

/// `GET /__tapi/invalidations`: a never ending response with one line of
/// space separated tags per batch of invalidations, plus empty keepalive lines.
///
/// The first batch after a quiet period is sent right away; invalidations
/// arriving within the following `throttle` window are sent as one batch at
/// its end. A stream that falls too far behind the bus is closed, because it
/// can't know what it missed; the client then reconnects and drops its cache.
pub(crate) async fn invalidation_stream(config: Arc<StreamConfig>, req: Request) -> Response {
    let (parts, _) = req.into_parts();
    let id = uuid::Uuid::new_v4().to_string();
    let filter = match &config.filter {
        Some(factory) => Some(factory(parts).await),
        None => None,
    };
    let mut subscription = config.bus.subscribe();
    let (tx, rx) = mpsc::channel::<Result<Bytes, Infallible>>(16);

    let origin = id.clone();
    tokio::spawn(async move {
        let send = |tags: Vec<String>| {
            let line = config.encode(&tags).join(" ") + "\n";
            tx.send(Ok(Bytes::from(line)))
        };
        let mut keepalive = tokio::time::interval(config.keepalive);
        let mut pending = BTreeSet::new();
        let mut window_end: Option<tokio::time::Instant> = None;
        loop {
            let window = async {
                match window_end {
                    Some(end) => tokio::time::sleep_until(end).await,
                    None => std::future::pending().await,
                }
            };
            tokio::select! {
                received = subscription.recv() => match received {
                    Ok(invalidation) => {
                        if invalidation.origin.as_deref() == Some(origin.as_str()) {
                            continue;
                        }
                        pending.extend(
                            invalidation
                                .tags
                                .into_iter()
                                .filter(|tag| filter.as_ref().is_none_or(|f| f(tag))),
                        );
                        if window_end.is_none() && !pending.is_empty() {
                            if send(std::mem::take(&mut pending).into_iter().collect()).await.is_err() {
                                return;
                            }
                            window_end = Some(tokio::time::Instant::now() + config.throttle);
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => return,
                    Err(broadcast::error::RecvError::Closed) => return,
                },
                _ = window => {
                    window_end = None;
                    if !pending.is_empty()
                        && send(std::mem::take(&mut pending).into_iter().collect()).await.is_err()
                    {
                        return;
                    }
                }
                // the first tick fires immediately, which flushes the headers
                _ = keepalive.tick() => {
                    if tx.send(Ok(Bytes::from_static(b"\n"))).await.is_err() {
                        return;
                    }
                }
                _ = tx.closed() => return,
            }
        }
    });

    let cookie = format!("{SESSION_COOKIE_NAME}={id}; Path=/; HttpOnly; SameSite=Strict");
    (
        [
            (header::CONTENT_TYPE, TAGS_CONTENT_TYPE.to_string()),
            (header::CACHE_CONTROL, "no-cache".to_string()),
            (header::SET_COOKIE, cookie),
        ],
        Body::from_stream(ReceiverStream::new(rx)),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hashes_like_the_typescript_server() {
        // reference value computed with @toapi/server's hashTag(te, "secret", "users")
        assert_eq!(hash_tag("secret", "users"), "35i5bow9gvj0w");
    }

    #[test]
    fn reads_the_session_cookie() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::COOKIE,
            HeaderValue::from_static("a=b; __tapi-session=abc; c=d"),
        );
        assert_eq!(session_id(&headers).as_deref(), Some("abc"));
    }
}
