# Toapi for Rust

The Toapi concepts in Rust: define the API once on the server, generate an
OpenAPI document from it, generate a typed client from that document, and keep
client caches fresh with tag-based invalidation and an invalidation stream.

```
users-server (axum handlers + toapi-server)
   │  api().openapi()
   ▼
openapi.json  (committed; a test fails when it is out of date)
   │  toapi-codegen in build.rs
   ▼
users-client  (request types + toapi-client runtime + get!/post!/… macros)
```

The wire protocol (`X-TAPI-Tags`, `X-TAPI-Expires-At`, `/__tapi/invalidations`,
`/__tapi/openapi.json`, the `__tapi-session` cookie and tag hashing) is the
same as in the TypeScript packages.

## Crates

| Crate | |
|---|---|
| `toapi-core` | Wire constants and the route naming scheme shared by everything else |
| `toapi-server` | `Api` builder on axum, `Cached`/`Invalidates` responses, invalidation stream, OpenAPI |
| `toapi-codegen` | OpenAPI → request types and `Request` impls, for build scripts |
| `toapi-client` | `Client` with tag cache, `watch()` streams and the invalidation stream listener |
| `toapi-macros` | `get!`, `post!`, `put!`, `patch!`, `delete!` (re-exported by `toapi-client`) |

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
// build.rs
fn main() {
    toapi_codegen::build("../users-server/openapi.json").unwrap();
}

// src/lib.rs
include!(concat!(env!("OUT_DIR"), "/toapi_client.rs"));
```

Every route and method becomes a set of types named after it:

| | `GET /users/{id}` |
|---|---|
| Request | `GetUsersById { id, query?, body? }` |
| Query parameters | `GetUsersByIdQuery` |
| Request body | `PutUsersByIdRequestBody` |
| Response body | `GetUsersByIdResponseBody` |

```rust
use toapi_client::{Client, get, post, put};
use users_client::*;

let client = Client::new("http://localhost:3000/api");

let user = client.query(GetUsersById { id: 1 }).await?;
let user = client.query(get!("/users/{id}", 1)).await?;
let users = client.query(get!("/users?limit", limit = Some(10))).await?;

let mut live = client.query(get!("/users/{id}", id)).watch(); // Stream<Item = Result<User, Error>>

client.mutate(put!("/users/{id}", id, { name: "Alicia", role: Role::Member })).await?;
```

- `query` only takes GET requests, `mutate` only the others.
- Awaiting a mutation invalidates the tags it returned and refetches the
  watched queries before it resolves. Mutations by other clients arrive over
  the invalidation stream and update `watch()` streams the same way.
- When the invalidation stream reconnects, the whole cache is dropped.

### Macros

The macros derive the type names from the method and the path through
`toapi-core`, the same function the codegen uses, and expand to a struct
literal. They don't read the OpenAPI document; the generated types must be in
scope.

```rust
get!("/users/{id}", 123)                           // GetUsersById { id: 123 }
get!("/users/{id}", id = 123)
get!("/users/{id}")                                // captures a local `id`
get!("/users/{id}?a={x}&b", 123, x = "foo", b = 2) // .. query: GetUsersByIdQuery { a: "foo".into(), b: 2 }
post!("/users", { name: "Alice", role: Role::Admin })
```

All path and query parameters must be passed (optional ones as `None`). Wrong
routes, missing or unknown parameters and type mismatches are compile errors
pointing at the macro call.

## Not covered yet

- Server-side response caching (the TypeScript server's `Cache`); tags and
  TTLs are sent to clients and invalidations are published, but GET responses
  aren't cached on the server.
- Redis/Postgres invalidation buses (implement `InvalidationBus`).
- NDJSON streaming responses, multipart bodies, array query parameters.
- Deduplication of concurrent identical queries in the client.
