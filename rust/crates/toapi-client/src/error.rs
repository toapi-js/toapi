use reqwest::StatusCode;

#[derive(Debug)]
pub enum Error {
    /// The request could not be sent or the response not be read.
    Transport(reqwest::Error),
    /// The server responded with a non-success status.
    Status { status: StatusCode, body: String },
    /// The request could not be encoded.
    Encode(String),
    /// The response body did not match the expected type.
    Decode(serde_json::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Transport(e) => write!(f, "request failed: {e}"),
            Error::Status { status, body } => write!(f, "server responded with {status}: {body}"),
            Error::Encode(e) => write!(f, "could not encode request: {e}"),
            Error::Decode(e) => write!(f, "could not decode response: {e}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Transport(e) => Some(e),
            Error::Decode(e) => Some(e),
            Error::Status { .. } | Error::Encode(_) => None,
        }
    }
}

impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Self {
        Error::Transport(e)
    }
}
