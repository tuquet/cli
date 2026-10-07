use axum::Json;
use serde_json::{json, Value};

#[utoipa::path(
    tag = "Lint",
    post,
    path = "/api/v1/lint",
    operation_id = "lint_workflow",
    summary = "Validate and lint Automa assets (Bridge Stub)",
    responses(
        (status = 200, description = "Lint diagnostic results")
    )
)]
pub async fn lint_workflow(
    Json(_payload): Json<Value>,
) -> Json<Value> {
    Json(json!({
        "valid": true,
        "issues": []
    }))
}
