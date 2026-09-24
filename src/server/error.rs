use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PageError {
    #[error("Page not found")]
    NotFound,
    #[error("Could not read page: {0}")]
    Internal(#[from] std::io::Error),
}

impl IntoResponse for PageError {
    fn into_response(self) -> Response {
        let status = match self {
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };
        tracing::warn!(error = %self, %status, "page request failed");
        (status, self.to_string()).into_response()
    }
}
