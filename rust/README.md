# Toapi for Rust

The Toapi concepts in Rust: define the API once on the server, generate an
OpenAPI document from it, generate a typed client from that document, and keep
client caches fresh with tag-based invalidation and an invalidation stream.

```
users-server (axum handlers + toapi-server)
   │  api().openapi()
   ▼
openapi.json  (committed; a test fails when it is out of date)
   │  toapi_client::generate!(..) (or toapi-codegen in build.rs)
   ▼
users-client  (api.users.id(1).get() path tree + request types + toapi-client runtime)
```

The wire protocol (`X-TAPI-Tags`, `X-TAPI-Expires-At`, `/__tapi/invalidations`,
`/__tapi/openapi.json`, the `__tapi-session` cookie and tag hashing) is the
same as in the TypeScript packages.

## Crates

| Crate | |
|---|---|
| `toapi-core` | Wire constants and the route naming scheme shared by everything else |
| `toapi-server` | `Api` builder on axum, `Cached`/`Invalidates` responses, invalidation stream, OpenAPI |
| `toapi-codegen` | OpenAPI → path tree, call builders and request types; used by `generate!` and build scripts |
| `toapi-client` | `Client` with tag cache, `watch()` streams and the invalidation stream listener; re-exports `generate!` |
| `toapi-macros` | `generate!`: runs `toapi-codegen` at compile time |

## Server: define the API

The handler signatures are the definition. GET handlers return `Cached<T>`,
mutations return `Invalidates<T>`; anything else doesn't compile.

```rust
async fn get_user(
    State(state): State<AppState>,
    Path(UserPath { id }): Path<UserPath>,
) -> Result<Cached<User>, HttpError> {
    let user = state.find(id).ok_or_else(HttpError::not_found)?;
    Ok(Cached::new(user).tags([format!("user:{id}")]))
}

async fn update_user(
    _session: Session,
    State(state): State<AppState>,
    Path(UserPath { id }): Path<UserPath>,
    Json(body): Json<NewUser>,
) -> Result<Invalidates<User>, HttpError> {
    let user = state.update(id, body)?;
    Ok(Invalidates::new(user).tags(["users".to_string(), format!("user:{id}")]))
}

pub fn api() -> Api<AppState> {
    Api::new()
        .info("Users", "1.0.0")
        .route("/users", get(list_users).post(create_user))
        .route("/users/{id}", get(get_user).put(update_user).delete(delete_user))
}

let router = axum::Router::new()
    .nest("/api", api().into_router())
    .with_state(AppState::default());
```

- Every extractor implements `OperationInput`, which feeds the OpenAPI
  document. `Path`, `Query`, `Json`, `State`, `Extension`, `HeaderMap`,
  `Method`, `Uri` and `Invalidator` are covered; custom extractors (auth)
  implement it themselves, e.g. to add a `401` response.
- `route` panics when the `{placeholders}` don't match the handler's `Path<T>`.
- `Invalidator` invalidates tags imperatively, from a handler or (via
  `api.invalidator()`) from a background job.
- `.hash_secret(..)`, `.throttle(..)`, `.keepalive(..)`, `.invalidation_filter(..)`
  and `.invalidation_bus(..)` configure the invalidation stream.

## OpenAPI

`api().openapi()` returns the OpenAPI 3.1 document. The example commits it as
`openapi.json` and checks it with a test:

```sh
UPDATE_OPENAPI=1 cargo test -p users-server --test openapi
```

## Client

```rust
// src/lib.rs
toapi_client::generate!("../users-server/openapi.json");
```

`generate!` reads the OpenAPI document at compile time (the path is relative
to the crate's `Cargo.toml`) and expands to the client code. The crate is
rebuilt when the document changes.

Alternatively, generate the code from a build script, which leaves a readable
file in `OUT_DIR` that go-to-definition can jump into:

```rust
// build.rs (with toapi-codegen as a build-dependency)
fn main() {
    toapi_codegen::build("../users-server/openapi.json").unwrap();
}

// src/lib.rs
include!(concat!(env!("OUT_DIR"), "/toapi_client.rs"));
```

The generated code is a path tree rooted at `Api`. Static path segments are
fields, parameters are methods, and HTTP methods return call builders that
take query parameters and body fields through setters:

```rust
use users_client::*;

let api = Api::new("http://localhost:3000/api");

let user  = api.users.id(123).get().await?;                 // GET /users/{id}
let users = api.users.get().limit(10).await?;               // GET /users?limit=10
let user  = api.users.id(123).put().name("Alicia").role(Role::Member).await?;
api.users.id(123).delete().await?;

let mut live = api.users.id(123).get().watch();             // Stream<Item = Result<Arc<User>, Error>>
```

- Parameters are typed: `.id("123")` doesn't compile when `{id}` is a `u64`.
- Optional query parameters and body fields can be left out. Required ones are
  tracked in the builder's type: a call missing one can't be awaited, and the
  error names it (`PostUsersCall<Set, Unset<required::role>>` is not a future).
- `.query(..)` and `.body(..)` take a whole query or body struct instead,
  e.g. `api.users.post().body(new_user)`. A whole body sets all of its
  required fields; individual setters can still follow it.
- Only GET calls have `.watch()`.
- Queries are cached by URL the moment they start, so concurrent queries for
  the same URL share one request. Each response is parsed once; queries and
  `watch()` streams resolve to an `Arc` of the parsed value, shared with the
  cache (`Arc<User>`, `Arc<Vec<User>>`). Mutations aren't cached and resolve
  to plain values.
- Awaiting a mutation deletes the cache entries carrying the tags it
  returned, publishes their URLs to the `watch()` streams and resolves once
  the watched ones were refetched. Mutations by other clients arrive over the
  invalidation stream and are handled the same way.
- Failed requests are dropped from the cache after `min_ttl` (100ms by
  default), responses with a TTL once it's over, plus a random delay of up to
  `max_overdue_ttl` (1s by default). Both are set on the client builder, like
  `minTTL` and `maxOverdueTTL` in the TypeScript client:
  `Client::builder(url).min_ttl(Duration::from_millis(500)).build()`.
- When the invalidation stream reconnects, the whole cache is dropped.

### Generated names

| `GET /users/{id}` | |
|---|---|
| `GetUsersByIdRequest` | The whole request: path parameters, `query` and `body` |
| `GetUsersByIdQuery` | Query parameters |
| `PutUsersByIdRequestBody` | Request body |
| `GetUsersByIdResponseBody` | Response body |
| `GetUsersByIdCall` | Call builder, returned by `api.users.id(..).get()` |
| `UsersByIdRoute` | Tree node, returned by `api.users.id(..)` |

The request structs implement `toapi_client::Request` and can be run directly,
e.g. to pass requests around as values:

```rust
let request = api.users.id(123).get().into_request(); // GetUsersByIdRequest { id: 123 }
let user = api.client().call(request).await?;
let user = api.client().call(GetUsersByIdRequest { id: 123 }).await?;
```

## WebAssembly

`toapi-client` and generated clients work in the browser
(`wasm32-unknown-unknown`):

- Requests go through the browser's `fetch`, tasks run with
  `wasm_bindgen_futures::spawn_local`, timers use `gloo-timers`, and the clock
  is `Date.now()`.
- Futures and streams from the client are `Send` on native targets only
  (`toapi_client::__private::MaybeSend`); the browser runs everything on one
  thread.
- The browser keeps cookies, and the client sends them with every request, so
  the server can tell a client's own invalidations apart. A cross-origin API
  has to allow credentials in its CORS headers.

The tests in `crates/toapi-client/tests/wasm.rs` run the client in Node
against a small HTTP server written in JavaScript:

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version <wasm-bindgen version in Cargo.lock>
cargo test -p toapi-client --target wasm32-unknown-unknown --test wasm
```

## Not covered yet

- Server-side response caching (the TypeScript server's `Cache`); tags and
  TTLs are sent to clients and invalidations are published, but GET responses
  aren't cached on the server.
- Redis/Postgres invalidation buses (implement `InvalidationBus`).
- NDJSON streaming responses, multipart bodies, array query parameters.
