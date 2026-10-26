use std::{
    collections::HashSet,
    future::{Future, IntoFuture},
    marker::PhantomData,
    pin::Pin,
    sync::{Arc, Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use futures::{FutureExt, Stream, future::join_all};
use reqwest::header::CONTENT_TYPE;
use serde::de::DeserializeOwned;
use toapi_core::{EXPIRES_AT_HEADER, INVALIDATIONS_ROUTE, TAGS_HEADER};
use tokio::sync::watch;

use crate::{
    Error,
    cache::{Cache, SharedResult, Value},
    pub_sub::{Callback, PubSub},
    request::{Kind, Mutation, Query, Request},
    stream::spawn_listener,
};

/// Default of [`ClientBuilder::min_ttl`], as in the TypeScript client.
const DEFAULT_MIN_TTL: Duration = Duration::from_millis(100);
/// Default of [`ClientBuilder::max_overdue_ttl`], as in the TypeScript client.
const DEFAULT_MAX_OVERDUE_TTL: Duration = Duration::from_millis(1000);

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
    min_ttl: Duration,
    max_overdue_ttl: Duration,
    cache: Cache,
    /// Subscriptions to the URLs of invalidated entries, held by the watch
    /// streams.
    pub_sub: RwLock<PubSub>,
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
    fn pub_sub(&self) -> RwLockReadGuard<'_, PubSub> {
        self.pub_sub.read().unwrap_or_else(|e| e.into_inner())
    }

    fn pub_sub_mut(&self) -> RwLockWriteGuard<'_, PubSub> {
        self.pub_sub.write().unwrap_or_else(|e| e.into_inner())
    }

    /// The request for `url`: the cached one, or a new one that is cached
    /// right away, so concurrent callers share it.
    pub(crate) fn request<T: Cacheable>(self: &Arc<Self>, url: &str) -> SharedResult {
        self.cache
            .get_or_insert_with(url, |id| self.spawn_request::<T>(url.to_string(), id))
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
        let min_ttl = self.min_ttl;
        match loaded {
            Ok((_, meta)) => {
                self.cache.set_tags(&url, id, meta.tags.clone());
                if let Some(expires_at) = meta.expires_at {
                    let delay = expires_at
                        .duration_since(SystemTime::now())
                        .unwrap_or_default()
                        + jitter(self.max_overdue_ttl);
                    tokio::spawn(async move {
                        tokio::time::sleep(delay).await;
                        let Some(inner) = inner.upgrade() else { return };
                        if inner.cache.remove(&url, id) {
                            inner.publish(HashSet::from([url])).await;
                        }
                    });
                }
            }
            Err(_) => {
                tokio::spawn(async move {
                    tokio::time::sleep(min_ttl).await;
                    if let Some(inner) = inner.upgrade() {
                        inner.cache.remove(&url, id);
                    }
                });
            }
        }
    }

    /// The response for `url`: the parsed value in the cache, shared.
    async fn query<T: Cacheable>(self: &Arc<Self>, url: &str) -> Result<Arc<T>, Error> {
        let value = self.request::<T>(url).await?;
        value.downcast::<T>().map_err(|_| {
            Error::Internal(format!(
                "the cache entry for {url} holds a different response type"
            ))
        })
    }

    /// Drops the entries carrying any of `tags` and resolves once the
    /// subscribers refetched them.
    pub(crate) async fn invalidate_tags(self: &Arc<Self>, tags: &[String]) {
        if tags.is_empty() {
            return;
        }
        let urls = self.cache.invalidate_tags(tags);
        self.publish(urls).await;
    }

    /// Drops the whole cache, e.g. after the invalidation stream reconnected
    /// and may have missed invalidations.
    pub(crate) async fn invalidate_all(self: &Arc<Self>) {
        let urls = self.cache.invalidate_all();
        self.publish(urls).await;
    }

    /// Calls every subscription with the URLs of dropped cache entries and
    /// resolves once all of them are done, like the TypeScript client's
    /// `PubSub.publish`. Watch streams refetch in their callbacks, so this
    /// resolves once the watched URLs among `urls` were refetched.
    async fn publish(self: &Arc<Self>, urls: HashSet<String>) {
        if urls.is_empty() {
            return;
        }
        let urls = Arc::new(urls);
        let callbacks = self.pub_sub().callbacks();
        join_all(
            callbacks
                .iter()
                .map(|callback| callback(self.clone(), urls.clone())),
        )
        .await;
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

/// A pseudo-random delay up to `max`.
fn jitter(max: Duration) -> Duration {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    max.mul_f64(f64::from(nanos % 1000) / 1000.0)
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
    min_ttl: Duration,
    max_overdue_ttl: Duration,
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

    /// How long a failed request stays in the cache, so a burst of identical
    /// queries doesn't hammer a failing server. Like the TypeScript client's
    /// `minTTL`; defaults to 100ms.
    pub fn min_ttl(mut self, min_ttl: Duration) -> Self {
        self.min_ttl = min_ttl;
        self
    }

    /// Responses with a TTL are dropped from the cache at a random point up
    /// to this long after they expired, so entries that expire together
    /// don't all refetch at once. Like the TypeScript client's
    /// `maxOverdueTTL`; defaults to 1s.
    pub fn max_overdue_ttl(mut self, max_overdue_ttl: Duration) -> Self {
        self.max_overdue_ttl = max_overdue_ttl;
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
            min_ttl: self.min_ttl,
            max_overdue_ttl: self.max_overdue_ttl,
            cache: Cache::default(),
            pub_sub: RwLock::default(),
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
            min_ttl: DEFAULT_MIN_TTL,
            max_overdue_ttl: DEFAULT_MAX_OVERDUE_TTL,
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
    R::Response: Send + Sync + 'static,
{
    /// A stream that yields the response now and again every time it is
    /// invalidated, by a mutation of this client or through the
    /// invalidation stream.
    ///
    /// The responses are shared with the cache, like the result of awaiting
    /// the query.
    pub fn watch(self) -> impl Stream<Item = Result<Arc<R::Response>, Error>> + Send + 'static {
        let QueryCall { inner, url, .. } = self;

        // subscribe right away, like the TypeScript client: the callback
        // refetches the URL when it is invalidated and hands the result to
        // the stream. The channel keeps only the latest result, so a stream
        // nobody polls doesn't pile up responses or hold up invalidations.
        let (sender, mut receiver) = watch::channel(None);
        let subscription = url.as_ref().ok().map(|url| {
            let sender = Arc::new(sender);
            let url = url.clone();
            let callback: Callback = Arc::new(move |inner, urls| {
                let sender = sender.clone();
                let url = url.clone();
                async move {
                    if urls.contains(&url) {
                        let result = inner.query::<R::Response>(&url).await;
                        sender.send_replace(Some(result));
                    }
                }
                .boxed()
            });
            Subscription::new(inner.clone(), callback)
        });

        async_stream::stream! {
            let _subscription = subscription;
            let url = match url {
                Ok(url) => url,
                Err(e) => {
                    yield Err(e);
                    return;
                }
            };
            yield inner.query::<R::Response>(&url).await;
            while receiver.changed().await.is_ok() {
                let latest = receiver.borrow_and_update().clone();
                if let Some(result) = latest {
                    yield result;
                }
            }
        }
    }
}

/// Resolves to the parsed response, shared with the cache and every other
/// query for the same URL.
impl<R> IntoFuture for QueryCall<R>
where
    R: Request<Kind = Query>,
    R::Response: Send + Sync + 'static,
{
    type Output = Result<Arc<R::Response>, Error>;
    type IntoFuture = BoxFuture<Self::Output>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async move { self.inner.query::<R::Response>(&self.url?).await })
    }
}

/// Unsubscribes its callback when dropped, i.e. when the watch stream is.
struct Subscription {
    inner: Arc<Inner>,
    id: u64,
}

impl Subscription {
    fn new(inner: Arc<Inner>, callback: Callback) -> Self {
        let id = inner.pub_sub_mut().subscribe(callback);
        Subscription { inner, id }
    }
}

impl Drop for Subscription {
    fn drop(&mut self) {
        self.inner.pub_sub_mut().unsubscribe(self.id);
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
