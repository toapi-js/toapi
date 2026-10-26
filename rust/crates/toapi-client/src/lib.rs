//! Typed client for a Toapi API.
//!
//! `toapi-codegen` generates the API specific code from the OpenAPI
//! document: an `Api` path tree with call builders, plus one request struct
//! per route and method (e.g. `GetUsersByIdRequest`). This crate runs them:
//!
//! ```ignore
//! let api = my_api_client::Api::new("http://localhost:3000/api");
//!
//! let user = api.users.id(1).get().await?;
//! let mut live = api.users.id(1).get().watch();
//! api.users.id(1).put().name("Alice").await?;
//!
//! // the same, with the request structs
//! let user = api.client().call(GetUsersByIdRequest { id: 1 }).await?;
//! ```
//!
//! Query responses are cached under the tags the server sent with them. A
//! mutation invalidates its tags locally once it returns; mutations by other
//! clients arrive over the invalidation stream. Either way, invalidated
//! entries are dropped and entries with an active [`QueryCall::watch`] stream
//! are refetched and pushed to the stream.

mod cache;
mod client;
mod error;
mod request;
mod stream;

pub use client::{Client, ClientBuilder, MutationCall, QueryCall};
pub use error::Error;
pub use request::{Kind, Mutation, Query, Request};
pub use reqwest::Method;

pub mod state;

#[doc(hidden)]
pub mod __private {
    pub use futures::Stream;
    pub use serde;
    pub use serde_json;

    pub use crate::request::{encode_path_segment, json_body, query_string};
}
