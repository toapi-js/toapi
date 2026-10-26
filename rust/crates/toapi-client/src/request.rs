use reqwest::Method;
use serde::{Serialize, de::DeserializeOwned};

use crate::{Client, Error, MutationCall, QueryCall};

/// A request to one route and method of the API. Implemented by the types
/// `toapi-codegen` generates.
pub trait Request: Sized {
    /// [`Query`] for GET requests, [`Mutation`] for the others. Decides what
    /// [`Client::call`] returns.
    type Kind: Kind;
    type Response: DeserializeOwned;
    const METHOD: Method;
    /// The route template, e.g. `/users/{id}`.
    const TEMPLATE: &'static str;

    /// The path with the parameters filled in, e.g. `/users/1`.
    fn path(&self) -> String;

    /// The encoded query string, without the leading `?`.
    fn query(&self) -> Result<Option<String>, Error> {
        Ok(None)
    }

    /// The JSON encoded request body.
    fn body(&self) -> Result<Option<Vec<u8>>, Error> {
        Ok(None)
    }
}

/// The kind of GET requests: served from the cache and watchable. Calling one
/// returns a [`QueryCall`].
pub struct Query;

/// The kind of POST/PUT/PATCH/DELETE requests: they invalidate the tags they
/// return. Calling one returns a [`MutationCall`].
pub struct Mutation;

mod sealed {
    pub trait Sealed {}
    impl Sealed for super::Query {}
    impl Sealed for super::Mutation {}
}

/// [`Query`] or [`Mutation`].
pub trait Kind: sealed::Sealed + Sized {
    /// What [`Client::call`] returns for a request of this kind.
    type Call<R: Request<Kind = Self>>;

    #[doc(hidden)]
    fn call<R: Request<Kind = Self>>(client: &Client, request: R) -> Self::Call<R>;
}

impl Kind for Query {
    type Call<R: Request<Kind = Self>> = QueryCall<R>;

    fn call<R: Request<Kind = Self>>(client: &Client, request: R) -> QueryCall<R> {
        client.query_call(request)
    }
}

impl Kind for Mutation {
    type Call<R: Request<Kind = Self>> = MutationCall<R>;

    fn call<R: Request<Kind = Self>>(client: &Client, request: R) -> MutationCall<R> {
        client.mutation_call(request)
    }
}

const SEGMENT: &percent_encoding::AsciiSet = &percent_encoding::NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'_')
    .remove(b'.')
    .remove(b'~');

pub fn encode_path_segment(value: &impl std::fmt::Display) -> String {
    percent_encoding::utf8_percent_encode(&value.to_string(), SEGMENT).to_string()
}

pub fn query_string(query: &impl Serialize) -> Result<Option<String>, Error> {
    let encoded = serde_urlencoded::to_string(query).map_err(|e| Error::Encode(e.to_string()))?;
    Ok((!encoded.is_empty()).then_some(encoded))
}

pub fn json_body(body: &impl Serialize) -> Result<Option<Vec<u8>>, Error> {
    serde_json::to_vec(body)
        .map(Some)
        .map_err(|e| Error::Encode(e.to_string()))
}
