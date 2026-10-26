use reqwest::Method;
use serde::{Serialize, de::DeserializeOwned};

use crate::Error;

/// A request to one route and method of the API. Implemented by the types
/// `toapi-codegen` generates.
pub trait Request {
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

/// A GET request: cached, deduplicated through the cache and watchable.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a GET request",
    label = "use `client.mutate(..)` for POST/PUT/PATCH/DELETE requests"
)]
pub trait QueryRequest: Request {}

/// A POST/PUT/PATCH/DELETE request: invalidates the tags it returns.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a POST/PUT/PATCH/DELETE request",
    label = "use `client.query(..)` for GET requests"
)]
pub trait MutationRequest: Request {}

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
