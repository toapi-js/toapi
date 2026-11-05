use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

/// An error with an HTTP status, rendered as a plain text response.
#[derive(Debug, Clone)]
pub struct HttpError {
    pub status: StatusCode,
    pub message: String,
}

impl HttpError {
    pub fn new(status: StatusCode, message: impl Into<String>) -> Self {
        HttpError {
            status,
            message: message.into(),
        }
    }

    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, message)
    }

    pub fn unauthorized() -> Self {
        Self::new(StatusCode::UNAUTHORIZED, "Unauthorized")
    }

    pub fn forbidden() -> Self {
        Self::new(StatusCode::FORBIDDEN, "Forbidden")
    }

    pub fn not_found() -> Self {
        Self::new(StatusCode::NOT_FOUND, "Not Found")
    }
}

impl std::fmt::Display for HttpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.status, self.message)
    }
}

impl std::error::Error for HttpError {}

impl IntoResponse for HttpError {
    fn into_response(self) -> Response {
        (self.status, self.message).into_response()
    }
}
