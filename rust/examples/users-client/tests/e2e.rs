use std::time::Duration;

use futures::StreamExt;
use reqwest::header::{AUTHORIZATION, HeaderMap, HeaderValue};
use toapi_client::{Client, Error};
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

fn api(base_url: &str, user: &str) -> Api {
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
    Api::from_client(Client::builder(base_url).http_client(http).build())
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
    let api = api(&base_url, "alice");

    let alice = api
        .users
        .post()
        .name("Alice")
        .role(Role::Admin)
        .await
        .unwrap();
    assert_eq!(alice.name, "Alice");
    let id = alice.id;

    let fetched = api.users.id(id).get().await.unwrap();
    assert_eq!(fetched, alice);

    let renamed = api
        .users
        .id(id)
        .put()
        .name("Alicia")
        .role(Role::Member)
        .await
        .unwrap();
    assert_eq!(renamed.name, "Alicia");

    // the PUT invalidated `user:{id}`, so this isn't served from the cache
    let fetched = api.users.id(id).get().await.unwrap();
    assert_eq!(fetched.name, "Alicia");

    let users = api.users.get().limit(10).await.unwrap();
    assert_eq!(users, vec![renamed]);

    api.users.id(id).delete().await.unwrap();
    let err = api.users.id(id).get().await.unwrap_err();
    assert!(
        matches!(err, Error::Status { status, .. } if status == 404),
        "{err}"
    );
}

#[tokio::test]
async fn calls_build_request_structs() {
    let api = Api::from_client(
        Client::builder("http://localhost")
            .invalidation_stream(false)
            .build(),
    );

    assert_eq!(
        api.users.id(7).get().into_request(),
        GetUsersByIdRequest { id: 7 }
    );
    assert_eq!(
        api.users.get().into_request(),
        GetUsersRequest {
            query: GetUsersQuery { limit: None }
        }
    );
    assert_eq!(
        api.users
            .id(7)
            .put()
            .role(Role::Admin)
            .name("Ada")
            .into_request(),
        PutUsersByIdRequest {
            id: 7,
            body: NewUser {
                name: "Ada".into(),
                role: Role::Admin
            }
        }
    );
}

#[tokio::test]
async fn request_structs_run_on_the_client() {
    let base_url = start_server().await;
    let api = api(&base_url, "alice");

    let request = PostUsersRequest {
        body: NewUser {
            name: "Bob".into(),
            role: Role::Member,
        },
    };
    let bob = api.client().mutate(request).await.unwrap();
    let fetched = api
        .client()
        .query(GetUsersByIdRequest { id: bob.id })
        .await
        .unwrap();
    assert_eq!(fetched, bob);
}

#[tokio::test]
async fn unauthorized_mutation() {
    let base_url = start_server().await;
    let api = Api::new(&base_url);
    let err = api
        .users
        .post()
        .name("Mallory")
        .role(Role::Admin)
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
    let api = api(&base_url, "alice");

    let mut users = Box::pin(api.users.get().watch());
    assert_eq!(next(&mut users).await.unwrap(), vec![]);

    let bob = api
        .users
        .post()
        .name("Bob")
        .role(Role::Member)
        .await
        .unwrap();
    // the mutation resolves after the watched query was refetched
    assert_eq!(next(&mut users).await.unwrap(), vec![bob]);
}

#[tokio::test]
async fn watch_follows_other_clients_through_the_invalidation_stream() {
    let base_url = start_server().await;
    let alice = api(&base_url, "alice");
    let bob = api(&base_url, "bob");

    let carol = alice
        .users
        .post()
        .name("Carol")
        .role(Role::Member)
        .await
        .unwrap();
    let id = carol.id;

    let mut watched = Box::pin(bob.users.id(id).get().watch());
    assert_eq!(next(&mut watched).await.unwrap().name, "Carol");
    // give bob's invalidation stream time to connect
    tokio::time::sleep(Duration::from_millis(200)).await;

    alice
        .users
        .id(id)
        .put()
        .name("Caroline")
        .role(Role::Admin)
        .await
        .unwrap();
    let mut user = next(&mut watched).await.unwrap();
    // bob may refetch once more on (re)connect; wait for the update
    while user.name != "Caroline" {
        user = next(&mut watched).await.unwrap();
    }
    assert_eq!(user.name, "Caroline");
}
