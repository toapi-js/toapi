use std::time::Duration;

use futures::StreamExt;
use reqwest::header::{AUTHORIZATION, HeaderMap, HeaderValue};
use toapi_client::{Client, Error, delete, get, post, put};
use users_client::*;

async fn start_server() -> String {
    let router = axum::Router::new()
        .nest(
            "/api",
            users_server::api()
                .throttle(Duration::from_millis(50))
                .into_router(),
        )
        .with_state(users_server::AppState::default());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    format!("http://{addr}/api")
}

fn client(base_url: &str, user: &str) -> Client {
    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {user}")).unwrap(),
    );
    let http = reqwest::Client::builder()
        .cookie_store(true)
        .default_headers(headers)
        .build()
        .unwrap();
    Client::builder(base_url).http_client(http).build()
}

async fn next<T>(stream: &mut (impl futures::Stream<Item = T> + Unpin)) -> T {
    tokio::time::timeout(Duration::from_secs(5), stream.next())
        .await
        .expect("timed out waiting for the stream")
        .expect("stream ended")
}

#[tokio::test]
async fn queries_and_mutations() {
    let base_url = start_server().await;
    let client = client(&base_url, "alice");

    let alice = client
        .mutate(post!("/users", { name: "Alice", role: Role::Admin }))
        .await
        .unwrap();
    assert_eq!(alice.name, "Alice");

    // the macro and the struct build the same request
    let id = alice.id;
    assert_eq!(get!("/users/{id}"), GetUsersById { id });
    assert_eq!(get!("/users/{id}", 1), GetUsersById { id: 1 });

    let fetched = client.query(get!("/users/{id}", id)).await.unwrap();
    assert_eq!(fetched, alice);

    let renamed = client
        .mutate(put!("/users/{id}", id = alice.id, { name: "Alicia", role: Role::Member }))
        .await
        .unwrap();
    assert_eq!(renamed.name, "Alicia");

    // the PUT invalidated `user:{id}`, so this isn't served from the cache
    let fetched = client.query(GetUsersById { id }).await.unwrap();
    assert_eq!(fetched.name, "Alicia");

    let users = client
        .query(get!("/users?limit={n}", n = Some(10)))
        .await
        .unwrap();
    assert_eq!(users, vec![renamed]);

    client.mutate(delete!("/users/{id}", id)).await.unwrap();
    let err = client.query(get!("/users/{id}", id)).await.unwrap_err();
    assert!(
        matches!(err, Error::Status { status, .. } if status == 404),
        "{err}"
    );
}

#[tokio::test]
async fn unauthorized_mutation() {
    let base_url = start_server().await;
    let client = Client::new(&base_url);
    let err = client
        .mutate(post!("/users", { name: "Mallory", role: Role::Admin }))
        .await
        .unwrap_err();
    assert!(
        matches!(err, Error::Status { status, .. } if status == 401),
        "{err}"
    );
}

#[tokio::test]
async fn watch_follows_own_mutations() {
    let base_url = start_server().await;
    let client = client(&base_url, "alice");

    let mut users = Box::pin(client.query(get!("/users?limit", limit = None)).watch());
    assert_eq!(next(&mut users).await.unwrap(), vec![]);

    let bob = client
        .mutate(post!("/users", { name: "Bob", role: Role::Member }))
        .await
        .unwrap();
    // the mutation resolves after the watched query was refetched
    assert_eq!(next(&mut users).await.unwrap(), vec![bob]);
}

#[tokio::test]
async fn watch_follows_other_clients_through_the_invalidation_stream() {
    let base_url = start_server().await;
    let alice = client(&base_url, "alice");
    let bob = client(&base_url, "bob");

    let carol = alice
        .mutate(post!("/users", { name: "Carol", role: Role::Member }))
        .await
        .unwrap();
    let id = carol.id;

    let mut watched = Box::pin(bob.query(get!("/users/{id}")).watch());
    assert_eq!(next(&mut watched).await.unwrap().name, "Carol");
    // give bob's invalidation stream time to connect
    tokio::time::sleep(Duration::from_millis(200)).await;

    alice
        .mutate(put!("/users/{id}", id, { name: "Caroline", role: Role::Admin }))
        .await
        .unwrap();
    let mut user = next(&mut watched).await.unwrap();
    // bob may refetch once more on (re)connect; wait for the update
    while user.name != "Caroline" {
        user = next(&mut watched).await.unwrap();
    }
    assert_eq!(user.name, "Caroline");
}
