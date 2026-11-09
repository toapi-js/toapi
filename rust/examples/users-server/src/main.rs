use axum::http::StatusCode;
use users_server::{AppState, api, client};

#[tokio::main]
async fn main() {
    let router = axum::Router::new()
        // The explicit fallback keeps unknown `/api/*` paths a 404 instead of
        // falling through to the frontend's `index.html`.
        .nest(
            "/api",
            api()
                .into_router()
                .fallback(|| async { StatusCode::NOT_FOUND }),
        )
        .merge(client::router())
        .with_state(AppState::default());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:5010")
        .await
        .unwrap();
    println!("listening on http://127.0.0.1:5010");
    axum::serve(listener, router).await.unwrap();
}
