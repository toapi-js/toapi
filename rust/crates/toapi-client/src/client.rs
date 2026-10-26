use std::{
    collections::HashSet,
    future::{Future, IntoFuture},
    marker::PhantomData,
    pin::Pin,
    sync::{Arc, Mutex, MutexGuard},
    time::{Duration, UNIX_EPOCH},
};

use bytes::Bytes;
use futures::{Stream, future::join_all};
use reqwest::header::CONTENT_TYPE;
use toapi_core::{EXPIRES_AT_HEADER, INVALIDATIONS_ROUTE, TAGS_HEADER};
use tokio::sync::broadcast;

use crate::{
    Error,
    cache::Cache,
    request::{MutationRequest, QueryRequest, Request},
    stream::spawn_listener,
};

/// Sent to watch streams after their entries were invalidated and refetched.
#[derive(Clone)]
pub(crate) enum CacheEvent {
    Invalidated(Arc<HashSet<String>>),
    All,
}

pub(crate) struct Inner {
    http: reqwest::Client,
    base_url: String,
    cache: Mutex<Cache>,
    events: broadcast::Sender<CacheEvent>,
    listener: Mutex<Option<tokio::task::AbortHandle>>,
}

impl Drop for Inner {
    fn drop(&mut self) {
        if let Some(listener) = self.listener.get_mut().unwrap().take() {
            listener.abort();
        }
    }
}

impl Inner {
    fn cache(&self) -> MutexGuard<'_, Cache> {
        self.cache.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Serves a GET from the cache, or fetches and caches it.
    async fn fetch(&self, url: &str) -> Result<Bytes, Error> {
        let generation = {
            let cache = self.cache();
            if let Some(body) = cache.get(url) {
                return Ok(body);
            }
            cache.generation()
        };
        let res = check_status(self.http.get(url).send().await?).await?;
        let tags = header_tags(&res);
        let expires_at = res
            .headers()
            .get(EXPIRES_AT_HEADER)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u64>().ok())
            .map(|ms| UNIX_EPOCH + Duration::from_millis(ms));
        let body = res.bytes().await?;
        self.cache()
            .insert(generation, url.to_string(), body.clone(), tags, expires_at);
        Ok(body)
    }

    /// Drops the entries carrying any of `tags`, refetches the watched ones
    /// and then notifies their watch streams.
    pub(crate) async fn invalidate_tags(&self, tags: &[String]) {
        if tags.is_empty() {
            return;
        }
        let (urls, watched) = {
            let mut cache = self.cache();
            let urls = cache.invalidate_tags(tags);
            let watched: Vec<String> = urls
                .iter()
                .filter(|url| cache.is_watched(url))
                .cloned()
                .collect();
            (urls, watched)
        };
        if urls.is_empty() {
            return;
        }
        // errors surface in the watch streams, which fetch again on a miss
        join_all(watched.iter().map(|url| self.fetch(url))).await;
        let _ = self.events.send(CacheEvent::Invalidated(Arc::new(
            urls.into_iter().collect(),
        )));
    }

    /// Drops the whole cache, e.g. after the invalidation stream reconnected
    /// and may have missed invalidations.
    pub(crate) async fn invalidate_all(&self) {
        let watched = {
            let mut cache = self.cache();
            cache.invalidate_all();
            cache.watched()
        };
        join_all(watched.iter().map(|url| self.fetch(url))).await;
        let _ = self.events.send(CacheEvent::All);
    }
}

async fn check_status(res: reqwest::Response) -> Result<reqwest::Response, Error> {
    if res.status().is_success() {
        Ok(res)
    } else {
        let status = res.status();
        let body = res.text().await.unwrap_or_default();
        Err(Error::Status { status, body })
    }
}

fn header_tags(res: &reqwest::Response) -> Vec<String> {
    res.headers()
        .get(TAGS_HEADER)
        .and_then(|v| v.to_str().ok())
        .map(|v| v.split_whitespace().map(String::from).collect())
        .unwrap_or_default()
}

fn decode<T: serde::de::DeserializeOwned>(body: &[u8]) -> Result<T, Error> {
    // an empty body decodes like `null`, which covers `()` responses
    let body = if body.is_empty() {
        b"null".as_slice()
    } else {
        body
    };
    serde_json::from_slice(body).map_err(Error::Decode)
}

/// Configures a [`Client`].
pub struct ClientBuilder {
    base_url: String,
    http: Option<reqwest::Client>,
    invalidation_stream: bool,
}

impl ClientBuilder {
    /// Uses a custom `reqwest::Client`. It should keep cookies
    /// (`cookie_store(true)`), so the server can tell this client's own
    /// invalidations apart on the invalidation stream.
    pub fn http_client(mut self, http: reqwest::Client) -> Self {
        self.http = Some(http);
        self
    }

    /// Whether to listen to the server's invalidation stream. Defaults to
    /// `true`; without it, only this client's own mutations invalidate its
    /// cache.
    pub fn invalidation_stream(mut self, enabled: bool) -> Self {
        self.invalidation_stream = enabled;
        self
    }

    /// # Panics
    ///
    /// When the invalidation stream is enabled and this isn't called from
    /// within a tokio runtime.
    pub fn build(self) -> Client {
        let http = self.http.unwrap_or_else(|| {
            reqwest::Client::builder()
                .cookie_store(true)
                .build()
                .expect("failed to build the HTTP client")
        });
        let base_url = self.base_url.trim_end_matches('/').to_string();
        let inner = Arc::new(Inner {
            http: http.clone(),
            base_url: base_url.clone(),
            cache: Mutex::default(),
            events: broadcast::channel(256).0,
            listener: Mutex::default(),
        });
        if self.invalidation_stream {
            let handle = spawn_listener(
                Arc::downgrade(&inner),
                http,
                format!("{base_url}{INVALIDATIONS_ROUTE}"),
            );
            *inner.listener.lock().unwrap() = Some(handle);
        }
        Client { inner }
    }
}

/// The client for one API. Cheap to clone; clones share the cache.
#[derive(Clone)]
pub struct Client {
    inner: Arc<Inner>,
}

impl Client {
    /// A client for the API at `base_url`, e.g. `http://localhost:3000/api`.
    ///
    /// # Panics
    ///
    /// When not called from within a tokio runtime; see [`ClientBuilder::build`].
    pub fn new(base_url: impl Into<String>) -> Client {
        Self::builder(base_url).build()
    }

    pub fn builder(base_url: impl Into<String>) -> ClientBuilder {
        ClientBuilder {
            base_url: base_url.into(),
            http: None,
            invalidation_stream: true,
        }
    }

    fn url<R: Request>(&self, request: &R) -> Result<String, Error> {
        let mut url = format!("{}{}", self.inner.base_url, request.path());
        if let Some(query) = request.query()? {
            url.push('?');
            url.push_str(&query);
        }
        Ok(url)
    }

    /// Runs a GET request. `.await` it for the response, or turn it into a
    /// stream of responses with [`QueryCall::watch`].
    pub fn query<R: QueryRequest>(&self, request: R) -> QueryCall<R> {
        QueryCall {
            inner: self.inner.clone(),
            url: self.url(&request),
            request: PhantomData,
        }
    }

    /// Runs a POST/PUT/PATCH/DELETE request. Awaiting it resolves once the
    /// response arrived and the queries it invalidated were refetched.
    pub fn mutate<R: MutationRequest>(&self, request: R) -> MutationCall<R> {
        let prepared = self
            .url(&request)
            .and_then(|url| Ok((url, request.body()?)));
        MutationCall {
            inner: self.inner.clone(),
            prepared,
            request: PhantomData,
        }
    }

    /// Invalidates tags locally, as if the server had sent them.
    pub async fn invalidate<I>(&self, tags: I)
    where
        I: IntoIterator,
        I::Item: Into<String>,
    {
        let tags: Vec<String> = tags.into_iter().map(Into::into).collect();
        self.inner.invalidate_tags(&tags).await;
    }
}

type BoxFuture<T> = Pin<Box<dyn Future<Output = T> + Send>>;

/// A GET request, returned by [`Client::query`].
#[must_use = "a query does nothing until it is awaited or watched"]
pub struct QueryCall<R> {
    inner: Arc<Inner>,
    url: Result<String, Error>,
    request: PhantomData<fn() -> R>,
}

impl<R> QueryCall<R>
where
    R: QueryRequest,
    R::Response: Send + 'static,
{
    /// A stream that yields the response now and again every time it is
    /// invalidated, by a mutation of this client or through the
    /// invalidation stream.
    pub fn watch(self) -> impl Stream<Item = Result<R::Response, Error>> + Send + 'static {
        let QueryCall { inner, url, .. } = self;
        async_stream::stream! {
            let url = match url {
                Ok(url) => url,
                Err(e) => {
                    yield Err(e);
                    return;
                }
            };
            let _watch = WatchGuard::new(inner.clone(), url.clone());
            let mut events = inner.events.subscribe();
            loop {
                yield inner.fetch(&url).await.and_then(|body| decode(&body));
                loop {
                    match events.recv().await {
                        Ok(CacheEvent::Invalidated(urls)) if !urls.contains(&url) => continue,
                        Ok(_) | Err(broadcast::error::RecvError::Lagged(_)) => break,
                        Err(broadcast::error::RecvError::Closed) => return,
                    }
                }
            }
        }
    }
}

impl<R> IntoFuture for QueryCall<R>
where
    R: QueryRequest,
    R::Response: Send + 'static,
{
    type Output = Result<R::Response, Error>;
    type IntoFuture = BoxFuture<Self::Output>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async move {
            let body = self.inner.fetch(&self.url?).await?;
            decode(&body)
        })
    }
}

struct WatchGuard {
    inner: Arc<Inner>,
    url: String,
}

impl WatchGuard {
    fn new(inner: Arc<Inner>, url: String) -> Self {
        inner.cache().watch(&url);
        WatchGuard { inner, url }
    }
}

impl Drop for WatchGuard {
    fn drop(&mut self) {
        self.inner.cache().unwatch(&self.url);
    }
}

/// A POST/PUT/PATCH/DELETE request, returned by [`Client::mutate`].
#[must_use = "a mutation does nothing until it is awaited"]
pub struct MutationCall<R> {
    inner: Arc<Inner>,
    prepared: Result<(String, Option<Vec<u8>>), Error>,
    request: PhantomData<fn() -> R>,
}

impl<R> IntoFuture for MutationCall<R>
where
    R: MutationRequest,
    R::Response: Send + 'static,
{
    type Output = Result<R::Response, Error>;
    type IntoFuture = BoxFuture<Self::Output>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async move {
            let (url, body) = self.prepared?;
            let mut request = self.inner.http.request(R::METHOD, &url);
            if let Some(body) = body {
                request = request.header(CONTENT_TYPE, "application/json").body(body);
            }
            let res = check_status(request.send().await?).await?;
            let tags = header_tags(&res);
            let body = res.bytes().await?;
            self.inner.invalidate_tags(&tags).await;
            decode(&body)
        })
    }
}
