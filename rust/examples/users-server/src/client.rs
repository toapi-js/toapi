//! Serves the `users-client` frontend (built with `wasm-pack build --target web`
//! into `users-client/pkg`) next to the API.

use axum::{
    Router,
    http::header,
    response::{Html, IntoResponse},
    routing::get,
};

async fn index() -> Html<&'static str> {
    Html(include_str!("../../users-client/index.html"))
}

async fn js_shim() -> impl IntoResponse {
    (
        // Module scripts are rejected by browsers unless served as JavaScript.
        [(header::CONTENT_TYPE, "text/javascript")],
        include_str!("../../users-client/pkg/users_client.js"),
    )
}

async fn wasm_module() -> impl IntoResponse {
    (
        // Required for `WebAssembly.instantiateStreaming`.
        [(header::CONTENT_TYPE, "application/wasm")],
        include_bytes!("../../users-client/pkg/users_client_bg.wasm").as_slice(),
    )
}

/// The frontend: the wasm bundle under `/pkg` and `index.html` for every other
/// path, so client side routes work on reload. Merge this into the app router
/// after nesting the API under `/api`.
pub fn router<S: Clone + Send + Sync + 'static>() -> Router<S> {
    Router::new()
        .route("/pkg/users_client.js", get(js_shim))
        .route("/pkg/users_client_bg.wasm", get(wasm_module))
        .fallback(index)
}
