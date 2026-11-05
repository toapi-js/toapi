use std::sync::Arc;

use reqwest::StatusCode;

/// Errors are `Clone` because everyone waiting on the same cached request
/// gets the same result.
#[derive(Debug, Clone)]
pub enum Error {
    /// The request could not be sent or the response not be read.
    Transport(Arc<reqwest::Error>),
    /// The server responded with a non-success status.
    Status { status: StatusCode, body: String },
    /// The request could not be encoded.
    Encode(String),
    /// The response body did not match the expected type.
    Decode(Arc<serde_json::Error>),
    /// A bug or an unexpected condition inside the client.
    Internal(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Transport(e) => write!(f, "request failed: {e}"),
            Error::Status { status, body } => write!(f, "server responded with {status}: {body}"),
            Error::Encode(e) => write!(f, "could not encode request: {e}"),
            Error::Decode(e) => write!(f, "could not decode response: {e}"),
            Error::Internal(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Transport(e) => Some(e.as_ref()),
            Error::Decode(e) => Some(e.as_ref()),
            Error::Status { .. } | Error::Encode(_) | Error::Internal(_) => None,
        }
    }
}

impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Self {
        Error::Transport(Arc::new(e))
    }
}
