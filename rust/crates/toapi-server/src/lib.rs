//! Define a REST API once, on top of axum.
//!
//! ```ignore
//! let api = Api::new()
//!     .info("Users", "1.0.0")
//!     .route("/users", get(list_users).post(create_user))
//!     .route("/users/{id}", get(get_user).put(update_user).delete(delete_user));
//!
//! let openapi = api.openapi();                  // serde_json::Value, OpenAPI 3.1
//! let router = api.into_router().with_state(state);
//! ```
//!
//! - GET handlers return [`Cached<T>`]: the response is tagged so clients can
//!   cache it and know when to refetch it.
//! - POST/PUT/PATCH/DELETE handlers return [`Invalidates<T>`]: the tags are
//!   published to every connected client over the invalidation stream.
//! - Every extractor a handler takes implements [`OperationInput`], which is
//!   how the request schema ends up in the OpenAPI document.
//!
//! The wire protocol (headers, invalidation stream, session cookie, tag
//! hashing) matches the TypeScript `@toapi/server`.

mod api;
mod cache;
mod error;
mod handler;
mod invalidation;
mod openapi;
mod response;

pub use api::{Api, MethodRoutes, delete, get, patch, post, put};
pub use cache::{Cache, Invalidation, PubSub, RequestCache};
pub use error::HttpError;
pub use handler::{DocumentedHandler, HandlerOutput, MutationKind, QueryKind};
pub use openapi::{Generators, OperationDoc, OperationInput};
pub use response::{Cached, Invalidates};

pub use schemars;
