use axum::{http::StatusCode, response::IntoResponse, Json};
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
#[schema(example = json!({"status": "ok", "version": "1.0.0", "message": "Automa Core Daemon is running"}))]
/// Daemon health check response
pub struct HealthResponse {
    /// Overall operational status ("ok", "degraded")
    pub status: String,
    /// Semantic version of the running daemon binary
    pub version: String,
    /// Status message description
    pub message: String,
}

#[utoipa::path(
    tag = "System",
    get,
    path = "/api/v1/health",
    operation_id = "get_health",
    summary = "Check daemon health status",
    description = "Returns active operational health status, daemon version, and server readiness for clients.",
    responses(
        (status = 200, description = "Daemon is healthy and accepting requests", body = HealthResponse)
    )
)]
pub async fn health() -> impl IntoResponse {
    let response = HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        message: "Automa Core Daemon is running".to_string(),
    };
    
    (StatusCode::OK, Json(response)).into_response()
}
