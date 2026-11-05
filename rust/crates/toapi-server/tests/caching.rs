//! The server-side response cache, driven through the router.

use std::{
    convert::Infallible,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use axum::{
    Router,
    body::Body,
    extract::State,
    http::{HeaderMap, Request, StatusCode},
};
use toapi_cache::InMemoryCache;
use toapi_server::{Api, Auth, Cached, HttpError, Invalidates, get, public};
use tower::ServiceExt;

#[derive(Clone, Default)]
struct Calls(Arc<AtomicUsize>);

impl Calls {
    fn next(&self) -> usize {
        self.0.fetch_add(1, Ordering::SeqCst) + 1
    }

    fn count(&self) -> usize {
        self.0.load(Ordering::SeqCst)
    }
}

/// The `x-user` header, required by `signed_in`.
#[derive(Clone)]
struct User(String);

async fn signed_in(headers: HeaderMap) -> Result<User, HttpError> {
    headers
        .get("x-user")
        .and_then(|v| v.to_str().ok())
        .map(|name| User(name.to_string()))
        .ok_or_else(HttpError::unauthorized)
}

async fn admin() -> Result<u32, Infallible> {
    Ok(1)
}

async fn tagged(State(calls): State<Calls>) -> Cached<usize> {
    Cached::new(calls.next()).tags(["counter"])
}

async fn untagged(State(calls): State<Calls>) -> Cached<usize> {
    Cached::new(calls.next())
}

async fn short_ttl(State(calls): State<Calls>) -> Cached<usize> {
    Cached::new(calls.next()).ttl(Duration::from_millis(100))
}

async fn failing(State(calls): State<Calls>) -> Result<Cached<usize>, HttpError> {
    calls.next();
    Err(HttpError::not_found())
}

async fn me(Auth(User(name)): Auth<User>) -> Cached<String> {
    Cached::new(name)
}

async fn bump() -> Invalidates<()> {
    Invalidates::new(()).tags(["counter"])
}

fn router(calls: Calls) -> Router {
    Api::new("Test", "1.0.0")
        .cache(Arc::new(InMemoryCache::new()))
        .route("/tagged", get(public, tagged).post(public, bump))
        .route("/untagged", get(public, untagged))
        .route("/ttl", get(public, short_ttl))
        .route("/failing", get(public, failing))
        .route("/private", get(signed_in, tagged))
        .route("/me", get(signed_in, me))
        .into_router()
        .with_state(calls)
}

struct Reply {
    status: StatusCode,
    headers: HeaderMap,
    body: String,
}

async fn send(router: &Router, method: &str, uri: &str, user: Option<&str>) -> Reply {
    let mut request = Request::builder().method(method).uri(uri);
    if let Some(user) = user {
        request = request.header("x-user", user);
    }
    let res = router
        .clone()
        .oneshot(request.body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = res.status();
    let headers = res.headers().clone();
    let body = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    Reply {
        status,
        headers,
        body: String::from_utf8(body.to_vec()).unwrap(),
    }
}

#[tokio::test]
async fn tagged_responses_are_served_from_the_cache() {
    let calls = Calls::default();
    let router = router(calls.clone());

    let first = send(&router, "GET", "/tagged", None).await;
    let second = send(&router, "GET", "/tagged", None).await;
    assert_eq!(calls.count(), 1);
    assert_eq!((first.body.as_str(), second.body.as_str()), ("1", "1"));
    // the cached response carries its tags like the fresh one
    assert_eq!(second.headers["x-tapi-tags"], "counter");
    assert_eq!(second.headers["content-type"], "application/json");

    // the query string is part of the key
    assert_eq!(send(&router, "GET", "/tagged?page=2", None).await.body, "2");
}

#[tokio::test]
async fn responses_without_tags_or_ttl_are_never_cached() {
    let calls = Calls::default();
    let router = router(calls.clone());
    send(&router, "GET", "/untagged", None).await;
    send(&router, "GET", "/untagged", None).await;
    assert_eq!(calls.count(), 2);
}

#[tokio::test]
async fn errors_are_never_cached() {
    let calls = Calls::default();
    let router = router(calls.clone());
    let reply = send(&router, "GET", "/failing", None).await;
    assert_eq!(reply.status, StatusCode::NOT_FOUND);
    send(&router, "GET", "/failing", None).await;
    assert_eq!(calls.count(), 2);
}

#[tokio::test]
async fn responses_with_a_ttl_expire() {
    let calls = Calls::default();
    let router = router(calls.clone());

    let first = send(&router, "GET", "/ttl", None).await;
    let second = send(&router, "GET", "/ttl", None).await;
    assert_eq!(calls.count(), 1);
    // a cached response keeps its original expiry
    assert_eq!(
        first.headers["x-tapi-expires-at"],
        second.headers["x-tapi-expires-at"]
    );

    tokio::time::sleep(Duration::from_millis(150)).await;
    assert_eq!(send(&router, "GET", "/ttl", None).await.body, "2");
}

#[tokio::test]
async fn mutations_invalidate_cached_responses() {
    let calls = Calls::default();
    let router = router(calls.clone());

    assert_eq!(send(&router, "GET", "/tagged", None).await.body, "1");
    let reply = send(&router, "POST", "/tagged", None).await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(send(&router, "GET", "/tagged", None).await.body, "2");
}

#[tokio::test]
async fn authorize_runs_before_the_cache() {
    let calls = Calls::default();
    let router = router(calls.clone());

    let reply = send(&router, "GET", "/private", Some("alice")).await;
    assert_eq!(reply.body, "1");
    // cached now, but a request that fails authorize never sees it
    let rejected = send(&router, "GET", "/private", None).await;
    assert_eq!(rejected.status, StatusCode::UNAUTHORIZED);
    // any authorized request gets the cached response
    let reply = send(&router, "GET", "/private", Some("bob")).await;
    assert_eq!(reply.body, "1");
    assert_eq!(calls.count(), 1);
}

#[tokio::test]
async fn handlers_read_the_authorize_result() {
    let router = router(Calls::default());
    let reply = send(&router, "GET", "/me", Some("alice")).await;
    assert_eq!(reply.body, "\"alice\"");
}

#[test]
#[should_panic(expected = "GET /me: the handler extracts Auth<u32>, but its authorize returns")]
fn auth_must_match_the_authorize_result() {
    async fn wrong(Auth(n): Auth<u32>) -> Cached<u32> {
        Cached::new(n)
    }
    let _ = Api::<()>::new("Test", "1.0.0").route("/me", get(signed_in, wrong));
}

#[test]
fn auth_matching_the_authorize_result_is_accepted() {
    async fn right(Auth(n): Auth<u32>) -> Cached<u32> {
        Cached::new(n)
    }
    let _ = Api::<()>::new("Test", "1.0.0").route("/admin", get(admin, right));
}
