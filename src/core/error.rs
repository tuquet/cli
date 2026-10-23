use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ApiErrorResponse {
    pub error: String,
    pub status: u16,
}

#[derive(Error, Debug)]
pub enum AutomationError {
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

impl From<rusqlite::Error> for AutomationError {
    fn from(err: rusqlite::Error) -> Self {
        AutomationError::DatabaseError(err.to_string())
    }
}

impl IntoResponse for AutomationError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AutomationError::NotFound(ref msg) => (StatusCode::NOT_FOUND, msg.clone()),
            AutomationError::BadRequest(ref msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            AutomationError::ExecutionFailed(ref msg) => {
                (StatusCode::INTERNAL_SERVER_ERROR, msg.clone())
            }
            AutomationError::DatabaseError(ref msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.clone()),
            AutomationError::IoError(ref err) => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()),
            AutomationError::JsonError(ref err) => (StatusCode::BAD_REQUEST, err.to_string()),
            AutomationError::Internal(ref msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.clone()),
        };

        let body = Json(ApiErrorResponse {
            error: message,
            status: status.as_u16(),
        });

        (status, body).into_response()
    }
}
