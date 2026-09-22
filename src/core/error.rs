use thiserror::Error;
use axum::response::{IntoResponse, Response};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ApiErrorResponse {
    pub error: String,
    pub status: u16,
}

#[derive(Error, Debug)]
pub enum AutomaError {
    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Internal server error: {0}")]
    Internal(String),
}

impl From<rusqlite::Error> for AutomaError {
    fn from(err: rusqlite::Error) -> Self {
        AutomaError::DatabaseError(err.to_string())
    }
}

impl From<anyhow::Error> for AutomaError {
    fn from(err: anyhow::Error) -> Self {
        AutomaError::Internal(err.to_string())
    }
}

impl IntoResponse for AutomaError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AutomaError::NotFound(ref msg) => (StatusCode::NOT_FOUND, msg.clone()),
            AutomaError::BadRequest(ref msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            AutomaError::ExecutionFailed(ref msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.clone()),
            AutomaError::DatabaseError(ref msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.clone()),
            AutomaError::IoError(ref err) => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()),
            AutomaError::JsonError(ref err) => (StatusCode::BAD_REQUEST, err.to_string()),
            AutomaError::Internal(ref msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.clone()),
        };

        let body = Json(ApiErrorResponse {
            error: message,
            status: status.as_u16(),
        });

        (status, body).into_response()
    }
}
