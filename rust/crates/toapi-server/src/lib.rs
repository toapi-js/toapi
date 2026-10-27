//! Define a REST API once, on top of axum.
//!
//! ```ignore
//! let api = Api::new("Users", "1.0.0")
//!     .cache(Arc::new(InMemoryCache::new()))
//!     .route("/users", get(public, list_users).post(signed_in, create_user))
//!     .route("/users/{id}", get(public, get_user).put(signed_in, update_user));
//!
//! let openapi = api.openapi();                  // serde_json::Value, OpenAPI 3.1
//! let router = api.into_router().with_state(state);
//! ```
//!
//! - Every route method takes an [`Authorize`] first, like the `authorize`
//!   of a TypeScript handler ([`public`] for open routes). Handlers read its
//!   result with the [`Auth`] extractor.
//! - GET handlers return [`Cached<T>`]: the response is tagged so clients can
//!   cache it and know when to refetch it. With tags or a TTL it's also
//!   stored in the server's [`Cache`] under its URL, after `authorize` and
//!   before the handler.
//! - POST/PUT/PATCH/DELETE handlers return [`Invalidates<T>`]: the tags are
//!   published to every connected client over the invalidation stream.
//! - Every extractor a handler takes implements [`OperationInput`], which is
//!   how the request schema ends up in the OpenAPI document.
//!
//! The wire protocol (headers, invalidation stream, session cookie, tag
//! hashing) matches the TypeScript `@toapi/server`.

mod api;
mod auth;
mod cache;
mod error;
mod handler;
mod invalidation;
mod openapi;
mod response;
mod route;

pub use api::{Api, MethodRoutes, delete, get, patch, post, put};
pub use auth::{Auth, Authorize, public};
pub use cache::{Cache, CacheEntry, InMemoryCache, Invalidation, Lookup, PubSub, RequestCache};
pub use error::HttpError;
pub use handler::{DocumentedHandler, HandlerOutput, MutationKind, QueryKind};
pub use openapi::{Generators, OperationDoc, OperationInput};
pub use response::{Cached, Invalidates};

pub use schemars;
