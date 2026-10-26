//! Typed client for a Toapi API.
//!
//! The request types are generated from the API's OpenAPI document by
//! `toapi-codegen` (one struct per route and method, e.g. `GetUsersById`).
//! The client runs them:
//!
//! ```ignore
//! use toapi_client::{Client, get, put};
//! use my_api_client::*;
//!
//! let client = Client::new("http://localhost:3000/api");
//!
//! let user = client.query(get!("/users/{id}", 1)).await?;
//! let mut live = client.query(GetUsersById { id: 1 }).watch();
//! client.mutate(put!("/users/{id}", id = 1, { name: "Alice".into() })).await?;
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
pub use request::{MutationRequest, QueryRequest, Request};
pub use reqwest::Method;
pub use toapi_macros::{delete, get, patch, post, put};

#[doc(hidden)]
pub mod __private {
    pub use serde;
    pub use serde_json;

    pub use crate::request::{IntoField, encode_path_segment, into_field, json_body, query_string};
}
