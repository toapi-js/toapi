//! The client in a JavaScript host (`wasm32-unknown-unknown`):
//!
//! ```sh
//! cargo test -p toapi-client --target wasm32-unknown-unknown --test wasm
//! ```
//!
//! The tests run in Node (see `.cargo/config.toml`), which lets them start a
//! small HTTP server from JavaScript. That exercises fetch, streaming
//! responses, parsing, the cache, spawning and timers on the browser runtime.

#![cfg(all(target_arch = "wasm32", target_os = "unknown"))]

use std::{sync::Arc, time::Duration};

use futures::{StreamExt, future::join_all};
use toapi_client::{Client, Error, Mutation, Query, Request};
use wasm_bindgen::prelude::*;
use wasm_bindgen_test::wasm_bindgen_test;

#[wasm_bindgen(inline_js = r#"
export async function startServer() {
    const http = await import("node:http");
    let counter = 0;
    let failing = 0;
    const streams = new Set();
    const server = http.createServer((req, res) => {
        switch (`${req.method} ${req.url}`) {
            // responds with how often it was called, tagged `counter`
            case "GET /counter":
                setTimeout(() => {
                    counter += 1;
                    res.writeHead(200, { "X-TAPI-Tags": "counter" });
                    res.end(String(counter));
                }, 20);
                return;
            // fails on the first call, succeeds afterwards
            case "GET /failing":
                failing += 1;
                res.writeHead(failing === 1 ? 500 : 200);
                res.end(failing === 1 ? "failed" : String(failing));
                return;
            // invalidates `counter` for every other client
            case "POST /bump":
                for (const stream of streams) stream.write("counter\n");
                res.writeHead(200);
                res.end("null");
                return;
            case "GET /__tapi/invalidations":
                res.writeHead(200, { "Content-Type": "text/tapi-tags" });
                res.write("\n");
                streams.add(res);
                req.on("close", () => streams.delete(res));
                return;
            default:
                res.writeHead(404);
                res.end();
        }
    });
    await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
    server.unref();
    return `http://127.0.0.1:${server.address().port}`;
}
"#)]
extern "C" {
    #[wasm_bindgen(js_name = startServer)]
    async fn start_server() -> JsValue;
}

async fn server() -> String {
    start_server().await.as_string().unwrap()
}

macro_rules! request {
    ($name:ident, $kind:ty, $method:ident, $path:literal, $response:ty) => {
        struct $name;

        impl Request for $name {
            type Kind = $kind;
            type Response = $response;
            const METHOD: toapi_client::Method = toapi_client::Method::$method;
            const TEMPLATE: &'static str = $path;

            fn path(&self) -> String {
                $path.into()
            }
        }
    };
}

request!(GetCounter, Query, GET, "/counter", u32);
request!(GetFailing, Query, GET, "/failing", u32);
request!(PostBump, Mutation, POST, "/bump", ());

fn client(base_url: &str) -> Client {
    Client::builder(base_url)
        .invalidation_stream(false)
        .min_ttl(Duration::from_millis(20))
        .build()
}

#[wasm_bindgen_test]
async fn concurrent_queries_share_one_request_and_value() {
    let client = client(&server().await);
    let results = join_all((0..5).map(|_| client.call(GetCounter).into_future())).await;
    let first = results[0].as_ref().unwrap().clone();
    assert_eq!(*first, 1);
    for result in &results {
        assert!(Arc::ptr_eq(result.as_ref().unwrap(), &first));
    }
}

#[wasm_bindgen_test]
async fn invalidation_refetches_watch_streams() {
    let client = client(&server().await);
    let mut watch = Box::pin(client.call(GetCounter).watch());
    assert_eq!(*watch.next().await.unwrap().unwrap(), 1);
    client.invalidate(["counter"]).await;
    assert_eq!(*watch.next().await.unwrap().unwrap(), 2);
}

#[wasm_bindgen_test]
async fn failed_requests_are_dropped_after_min_ttl() {
    let client = client(&server().await);
    let err = client.call(GetFailing).await.unwrap_err();
    assert!(
        matches!(err, Error::Status { status, .. } if status == 500),
        "{err}"
    );
    // the failure is dropped from the cache by a timer task
    gloo_timers::future::sleep(Duration::from_millis(100)).await;
    assert_eq!(*client.call(GetFailing).await.unwrap(), 2);
}

#[wasm_bindgen_test]
async fn the_invalidation_stream_reaches_watch_streams() {
    let base_url = server().await;
    let watching = Client::builder(&base_url).build();
    let mutating = client(&base_url);

    let mut watch = Box::pin(watching.call(GetCounter).watch());
    assert_eq!(*watch.next().await.unwrap().unwrap(), 1);
    // let the invalidation stream connect, and skip refetches caused by it
    gloo_timers::future::sleep(Duration::from_millis(200)).await;

    mutating.call(PostBump).await.unwrap();
    let mut value = watch.next().await.unwrap().unwrap();
    while *value < 3 {
        value = watch.next().await.unwrap().unwrap();
    }
}
