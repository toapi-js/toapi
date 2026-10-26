use std::{
    collections::HashSet,
    future::{Future, IntoFuture},
    marker::PhantomData,
    pin::Pin,
    sync::{Arc, Mutex, MutexGuard},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use futures::{FutureExt, Stream, future::join_all};
use reqwest::header::CONTENT_TYPE;
use serde::de::DeserializeOwned;
use toapi_core::{EXPIRES_AT_HEADER, INVALIDATIONS_ROUTE, TAGS_HEADER};
use tokio::sync::broadcast;

use crate::{
    Error,
    cache::{Cache, Reload, SharedResult, Value},
    request::{Kind, Mutation, Query, Request},
    stream::spawn_listener,
};

/// How long a failed request stays cached, so a burst of identical requests
/// doesn't hammer a failing server (like the TypeScript client's `minTTL`).
const ERROR_TTL: Duration = Duration::from_millis(100);
/// Expired responses are dropped up to this much later, so entries that
/// expire together don't all refetch at the same moment.
const MAX_OVERDUE: Duration = Duration::from_millis(1000);

/// A response type the cache can hold.
pub(crate) trait Cacheable: DeserializeOwned + Send + Sync + 'static {}
impl<T: DeserializeOwned + Send + Sync + 'static> Cacheable for T {}

struct Meta {
    tags: Vec<String>,
    expires_at: Option<SystemTime>,
}

pub(crate) struct Inner {
    http: reqwest::Client,
    base_url: String,
    cache: Mutex<Cache>,
    /// The URLs of invalidated entries, for the watch streams to refetch.
    events: broadcast::Sender<Arc<HashSet<String>>>,
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

    /// The request for `url`: the cached one, or a new one that is cached
    /// right away, so concurrent callers share it.
    pub(crate) fn request<T: Cacheable>(self: &Arc<Self>, url: &str) -> SharedResult {
        let mut cache = self.cache();
        if let Some(result) = cache.get(url) {
            return result;
        }
        let id = cache.next_id();
        let result = self.spawn_request::<T>(url.to_string(), id);
        cache.insert(url.to_string(), id, result.clone());
        result
    }

    /// Runs the request in its own task, so it completes (and lands in the
    /// cache) even when every caller stops waiting for it.
    fn spawn_request<T: Cacheable>(self: &Arc<Self>, url: String, id: u64) -> SharedResult {
        let http = self.http.clone();
        let inner = Arc::downgrade(self);
        let task = tokio::spawn(async move {
            let loaded = load::<T>(&http, &url).await;
            if let Some(inner) = inner.upgrade() {
                inner.settle(&url, id, &loaded);
            }
            loaded.map(|(value, _)| value)
        });
        async move {
            task.await
                .unwrap_or_else(|e| Err(Error::Internal(format!("request task failed: {e}"))))
        }
        .boxed()
        .shared()
    }

    /// Records the tags of a finished request, schedules its expiry, or drops
    /// it shortly after a failure.
    fn settle(self: &Arc<Self>, url: &str, id: u64, loaded: &Result<(Value, Meta), Error>) {
        let inner = Arc::downgrade(self);
        let url = url.to_string();
        match loaded {
            Ok((_, meta)) => {
                self.cache().set_tags(&url, id, meta.tags.clone());
                if let Some(expires_at) = meta.expires_at {
                    let delay = expires_at
                        .duration_since(SystemTime::now())
                        .unwrap_or_default()
                        + jitter();
                    tokio::spawn(async move {
                        tokio::time::sleep(delay).await;
                        let Some(inner) = inner.upgrade() else { return };
                        if inner.cache().remove(&url, id) {
                            inner.revalidate(HashSet::from([url])).await;
                        }
                    });
                }
            }
            Err(_) => {
                tokio::spawn(async move {
                    tokio::time::sleep(ERROR_TTL).await;
                    if let Some(inner) = inner.upgrade() {
                        inner.cache().remove(&url, id);
                    }
                });
            }
        }
    }

    /// The response for `url`, parsed once and cloned out of the cache.
    async fn query<T: Cacheable + Clone>(self: &Arc<Self>, url: &str) -> Result<T, Error> {
        let value = self.request::<T>(url).await?;
        value.downcast_ref::<T>().cloned().ok_or_else(|| {
            Error::Internal(format!(
                "the cache entry for {url} holds a different response type"
            ))
        })
    }

    /// Drops the entries carrying any of `tags` and revalidates them.
    pub(crate) async fn invalidate_tags(self: &Arc<Self>, tags: &[String]) {
        if tags.is_empty() {
            return;
        }
        let urls = self.cache().invalidate_tags(tags);
        self.revalidate(urls).await;
    }

    /// Drops the whole cache, e.g. after the invalidation stream reconnected
    /// and may have missed invalidations.
    pub(crate) async fn invalidate_all(self: &Arc<Self>) {
        let urls = self.cache().invalidate_all();
        self.revalidate(urls).await;
    }

    /// Tells the watch streams that `urls` were dropped from the cache, and
    /// resolves once the watched ones were refetched.
    ///
    /// The refetches start before the message goes out, so the watch streams
    /// pick up these requests from the cache instead of starting their own.
    async fn revalidate(self: &Arc<Self>, urls: HashSet<String>) {
        if urls.is_empty() {
            return;
        }
        let watched = self.cache().watched(&urls);
        let refetches: Vec<SharedResult> = watched
            .iter()
            .map(|(url, reload)| reload(self, url))
            .collect();
        let _ = self.events.send(Arc::new(urls));
        // errors surface in the watch streams
        join_all(refetches).await;
    }
}

async fn load<T: Cacheable>(http: &reqwest::Client, url: &str) -> Result<(Value, Meta), Error> {
    let res = check_status(http.get(url).send().await?).await?;
    let meta = Meta {
        tags: header_tags(&res),
        expires_at: res
            .headers()
            .get(EXPIRES_AT_HEADER)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u64>().ok())
            .map(|ms| UNIX_EPOCH + Duration::from_millis(ms)),
    };
    let body = res.bytes().await?;
    let value: T = decode(&body)?;
    Ok((Arc::new(value), meta))
}

/// A pseudo-random delay up to [`MAX_OVERDUE`].
fn jitter() -> Duration {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    MAX_OVERDUE.mul_f64(f64::from(nanos % 1000) / 1000.0)
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
    serde_json::from_slice(body).map_err(|e| Error::Decode(Arc::new(e)))
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

    /// Runs a request.
    ///
    /// - GET requests return a [`QueryCall`]: `.await` it for the response,
    ///   or turn it into a stream of responses with [`QueryCall::watch`].
    /// - POST/PUT/PATCH/DELETE requests return a [`MutationCall`]: awaiting
    ///   it resolves once the response arrived and the queries it
    ///   invalidated were refetched.
    pub fn call<R: Request>(&self, request: R) -> <R::Kind as Kind>::Call<R> {
        R::Kind::call(self, request)
    }

    pub(crate) fn query_call<R: Request<Kind = Query>>(&self, request: R) -> QueryCall<R> {
        QueryCall {
            inner: self.inner.clone(),
            url: self.url(&request),
            request: PhantomData,
        }
    }

    pub(crate) fn mutation_call<R: Request<Kind = Mutation>>(&self, request: R) -> MutationCall<R> {
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

/// A GET request, returned by [`Client::call`].
#[must_use = "a query does nothing until it is awaited or watched"]
pub struct QueryCall<R> {
    inner: Arc<Inner>,
    url: Result<String, Error>,
    request: PhantomData<fn() -> R>,
}

impl<R> QueryCall<R>
where
    R: Request<Kind = Query>,
    R::Response: Clone + Send + Sync + 'static,
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
            let reload: Reload = Arc::new(|inner, url| inner.request::<R::Response>(url));
            let _watch = WatchGuard::new(inner.clone(), url.clone(), reload);
            let mut events = inner.events.subscribe();
            loop {
                yield inner.query::<R::Response>(&url).await;
                loop {
                    match events.recv().await {
                        Ok(urls) if !urls.contains(&url) => continue,
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
    R: Request<Kind = Query>,
    R::Response: Clone + Send + Sync + 'static,
{
    type Output = Result<R::Response, Error>;
    type IntoFuture = BoxFuture<Self::Output>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async move { self.inner.query::<R::Response>(&self.url?).await })
    }
}

struct WatchGuard {
    inner: Arc<Inner>,
    url: String,
}

impl WatchGuard {
    fn new(inner: Arc<Inner>, url: String, reload: Reload) -> Self {
        inner.cache().watch(&url, reload);
        WatchGuard { inner, url }
    }
}

impl Drop for WatchGuard {
    fn drop(&mut self) {
        self.inner.cache().unwatch(&self.url);
    }
}

/// A POST/PUT/PATCH/DELETE request, returned by [`Client::call`].
#[must_use = "a mutation does nothing until it is awaited"]
pub struct MutationCall<R> {
    inner: Arc<Inner>,
    prepared: Result<(String, Option<Vec<u8>>), Error>,
    request: PhantomData<fn() -> R>,
}

impl<R> IntoFuture for MutationCall<R>
where
    R: Request<Kind = Mutation>,
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
