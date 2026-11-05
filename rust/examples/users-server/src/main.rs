use users_server::{AppState, api};

#[tokio::main]
async fn main() {
    let router = axum::Router::new()
        .nest("/api", api().into_router())
        .with_state(AppState::default());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    println!("listening on http://127.0.0.1:3000/api");
    axum::serve(listener, router).await.unwrap();
}
