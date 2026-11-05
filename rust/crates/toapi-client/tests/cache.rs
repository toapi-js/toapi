// runs against a local axum server, so native targets only; see wasm.rs for
// the browser
#![cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]

//! The cache against a small counting server: requests are shared, responses
//! are parsed once, and invalidation drops entries.

use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use axum::{Router, http::StatusCode, routing::get};
use futures::{StreamExt, future::join_all};
use serde::{Deserialize, Deserializer};
use toapi_client::{Client, Error, Query, Request};

/// Counts how often a response body is parsed.
static PARSED: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug, Clone, PartialEq)]
struct Counter(usize);

impl<'de> Deserialize<'de> for Counter {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        PARSED.fetch_add(1, Ordering::SeqCst);
        usize::deserialize(deserializer).map(Counter)
    }
}

/// `GET /counter`: responds with how often it was called, tagged `counter`.
struct GetCounter;

impl Request for GetCounter {
    type Kind = Query;
    type Response = Counter;
    const METHOD: toapi_client::Method = toapi_client::Method::GET;
    const TEMPLATE: &'static str = "/counter";

    fn path(&self) -> String {
        "/counter".into()
    }
}

/// `GET /counter` again, with a plain response type that doesn't touch
/// `PARSED`, for tests running in parallel to the one reading it.
struct GetCount;

impl Request for GetCount {
    type Kind = Query;
    type Response = usize;
    const METHOD: toapi_client::Method = toapi_client::Method::GET;
    const TEMPLATE: &'static str = "/counter";

    fn path(&self) -> String {
        "/counter".into()
    }
}

/// `GET /failing`: fails on the first call, succeeds afterwards.
struct GetFailing;

impl Request for GetFailing {
    type Kind = Query;
    type Response = usize;
    const METHOD: toapi_client::Method = toapi_client::Method::GET;
    const TEMPLATE: &'static str = "/failing";

    fn path(&self) -> String {
        "/failing".into()
    }
}

async fn start_server() -> (String, Arc<AtomicUsize>) {
    let hits = Arc::new(AtomicUsize::new(0));
    let counter_hits = hits.clone();
    let failing_hits = Arc::new(AtomicUsize::new(0));
    let router = Router::new()
        .route(
            "/counter",
            get(move || async move {
                // slow enough for concurrent requests to overlap
                tokio::time::sleep(Duration::from_millis(50)).await;
                let n = counter_hits.fetch_add(1, Ordering::SeqCst) + 1;
                ([("X-TAPI-Tags", "counter")], n.to_string())
            }),
        )
        .route(
            "/failing",
            get(move || async move {
                match failing_hits.fetch_add(1, Ordering::SeqCst) {
                    0 => Err(StatusCode::INTERNAL_SERVER_ERROR),
                    n => Ok(n.to_string()),
                }
            }),
        );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    (format!("http://{addr}"), hits)
}

fn client(base_url: &str) -> Client {
    Client::builder(base_url).invalidation_stream(false).build()
}

// the PARSED counter is global, so the tests that read it run in one test
#[tokio::test]
async fn shares_requests_and_parses_once() {
    let (base_url, hits) = start_server().await;
    let client = client(&base_url);
    PARSED.store(0, Ordering::SeqCst);

    // concurrent requests share the first one
    let results = join_all((0..5).map(|_| client.call(GetCounter).into_future())).await;
    for result in results {
        assert_eq!(*result.unwrap(), Counter(1));
    }
    assert_eq!(hits.load(Ordering::SeqCst), 1);

    // later requests are served from the parsed value
    for _ in 0..3 {
        assert_eq!(*client.call(GetCounter).await.unwrap(), Counter(1));
    }
    assert_eq!(hits.load(Ordering::SeqCst), 1);
    assert_eq!(PARSED.load(Ordering::SeqCst), 1);

    // and get the very same parsed value, not a copy
    let a = client.call(GetCounter).await.unwrap();
    let b = client.call(GetCounter).await.unwrap();
    assert!(Arc::ptr_eq(&a, &b));

    // invalidation drops the entry, the next request refetches
    client.invalidate(["counter"]).await;
    assert_eq!(*client.call(GetCounter).await.unwrap(), Counter(2));
    assert_eq!(hits.load(Ordering::SeqCst), 2);
    assert_eq!(PARSED.load(Ordering::SeqCst), 2);

    // a watch stream refetches on invalidation, and the refetch is shared
    let mut watch = Box::pin(client.call(GetCounter).watch());
    assert_eq!(*watch.next().await.unwrap().unwrap(), Counter(2));
    client.invalidate(["counter"]).await;
    // invalidate resolves once the watched query was refetched
    assert_eq!(hits.load(Ordering::SeqCst), 3);
    assert_eq!(*watch.next().await.unwrap().unwrap(), Counter(3));
    assert_eq!(*client.call(GetCounter).await.unwrap(), Counter(3));
    assert_eq!(hits.load(Ordering::SeqCst), 3);
    assert_eq!(PARSED.load(Ordering::SeqCst), 3);

    // other tags leave the entry alone
    client.invalidate(["something-else"]).await;
    assert_eq!(*client.call(GetCounter).await.unwrap(), Counter(3));
    assert_eq!(hits.load(Ordering::SeqCst), 3);
}

#[tokio::test]
async fn drops_failed_requests_shortly_after() {
    let (base_url, _) = start_server().await;
    let client = client(&base_url);

    let err = client.call(GetFailing).await.unwrap_err();
    assert!(
        matches!(err, Error::Status { status, .. } if status == 500),
        "{err}"
    );
    // right after the failure, the failed request is still shared
    assert!(client.call(GetFailing).await.is_err());

    tokio::time::sleep(Duration::from_millis(200)).await;
    assert_eq!(*client.call(GetFailing).await.unwrap(), 1);
}

#[tokio::test]
async fn watch_subscriptions_refetch_without_being_polled() {
    let (base_url, hits) = start_server().await;
    let client = client(&base_url);

    // subscribed as soon as `watch()` is called, before the first poll
    let mut watch = Box::pin(client.call(GetCount).watch());
    assert_eq!(*client.call(GetCount).await.unwrap(), 1);

    // nobody polls the stream, yet the invalidation completes and its
    // callback refetched the URL
    tokio::time::timeout(Duration::from_secs(5), client.invalidate(["counter"]))
        .await
        .expect("invalidation waited for the stream to be polled");
    assert_eq!(hits.load(Ordering::SeqCst), 2);

    // the stream starts with the current value, then the pushed refetch
    assert_eq!(*watch.next().await.unwrap().unwrap(), 2);
    client.invalidate(["counter"]).await;
    assert_eq!(*watch.next().await.unwrap().unwrap(), 3);

    // a dropped stream unsubscribes: invalidations no longer refetch
    drop(watch);
    client.invalidate(["counter"]).await;
    assert_eq!(hits.load(Ordering::SeqCst), 3);
}

#[tokio::test]
async fn dropping_an_unpolled_watch_stream_unsubscribes() {
    let (base_url, hits) = start_server().await;
    let client = client(&base_url);
    assert_eq!(*client.call(GetCount).await.unwrap(), 1);

    // subscribed by `watch()`, dropped without ever being polled
    let watch = client.call(GetCount).watch();
    drop(watch);

    client.invalidate(["counter"]).await;
    assert_eq!(hits.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn min_ttl_is_configurable() {
    let (base_url, _) = start_server().await;
    let client = Client::builder(&base_url)
        .invalidation_stream(false)
        .min_ttl(Duration::from_secs(5))
        .build();

    assert!(client.call(GetFailing).await.is_err());
    // well past the default of 100ms, the failure is still cached
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert!(client.call(GetFailing).await.is_err());
}
