//! Typed client for a Toapi API.
//!
//! [`generate!`] generates the API specific code from the OpenAPI document at
//! compile time: an `Api` path tree with call builders, plus one request
//! struct per route and method (e.g. `GetUsersByIdRequest`). This crate runs
//! them:
//!
//! ```ignore
//! toapi_client::generate!("../server/openapi.json");
//!
//! let api = Api::new("http://localhost:3000/api");
//!
//! let user = api.users.id(1).get().await?;
//! let mut live = api.users.id(1).get().watch();
//! api.users.id(1).put().name("Alice").await?;
//!
//! // the same, with the request structs
//! let user = api.client().call(GetUsersByIdRequest { id: 1 }).await?;
//! ```
//!
//! The cache works like the TypeScript client's:
//!
//! - The first query for a URL is cached right away, as a shared future, so
//!   concurrent queries for the same URL share one request. The request runs
//!   in its own task and completes even if nobody waits for it anymore.
//! - The response is parsed once and kept as the parsed value. Queries
//!   resolve to an `Arc` of it, shared with the cache and every other query
//!   for the URL. Mutations, which aren't cached, resolve to plain values.
//!   Failed requests are dropped from the cache after
//!   [`ClientBuilder::min_ttl`], expired ones once their TTL is over, plus
//!   up to [`ClientBuilder::max_overdue_ttl`].
//! - A mutation invalidates its tags locally once it returns; mutations by
//!   other clients arrive over the invalidation stream. Either way, the
//!   entries carrying those tags are deleted and their URLs are published to
//!   the subscriptions. Each [`QueryCall::watch`] stream subscribes a
//!   callback that refetches its URL and pushes the result to the stream;
//!   publishing awaits all callbacks, so a mutation resolves once the
//!   watched queries it invalidated were refetched.

mod cache;
mod client;
mod error;
mod pub_sub;
mod request;
mod stream;

pub use client::{Client, ClientBuilder, MutationCall, QueryCall};
pub use error::Error;
pub use request::{Kind, Mutation, Query, Request};
pub use reqwest::Method;
pub use toapi_macros::generate;

pub mod state;

#[doc(hidden)]
pub mod __private {
    pub use futures::Stream;
    pub use serde;
    pub use serde_json;

    pub use crate::request::{encode_path_segment, json_body, query_string};
}
