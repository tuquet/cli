use axum::{
    extract::{Json, Query},
    response::Response,
    body::Body,
};
use std::path::PathBuf;
use crate::core::error::AutomaError;

use super::types::{GetWorkflowFileParams, SaveWorkflowFilePayload, SaveWorkflowFileResponse};

pub fn resolve_safe_workflow_path(raw_path: &str) -> Result<PathBuf, AutomaError> {
    if raw_path.is_empty() || raw_path.contains('\0') {
        return Err(AutomaError::BadRequest("Invalid path: empty or contains null character".into()));
    }
    let path = PathBuf::from(raw_path);
    let normalized = if path.exists() {
        path.canonicalize().map_err(|e| AutomaError::BadRequest(format!("Failed to canonicalize path: {}", e)))?
    } else {
        path
    };
    Ok(normalized)
}

#[utoipa::path(
    tag = "Storage",
    get,
    path = "/api/v1/storage/workflow",
    operation_id = "get_workflow_file",
    summary = "Read workflow JSON file from filesystem",
    params(GetWorkflowFileParams),
    responses(
        (status = 200, description = "Workflow JSON file read successfully"),
        (status = 404, description = "Workflow file not found on disk")
    )
)]
pub async fn get_workflow(
    Query(params): Query<GetWorkflowFileParams>,
) -> Result<Response, AutomaError> {
    let safe_path = resolve_safe_workflow_path(&params.path)?;
    if !safe_path.exists() {
        return Err(AutomaError::NotFound(format!("Workflow file does not exist: {}", params.path)));
    }
    let content = tokio::fs::read_to_string(&safe_path).await
        .map_err(|e| AutomaError::Internal(format!("Failed to read file: {}", e)))?;
    let _ = serde_json::from_str::<serde_json::Value>(&content)
        .map_err(|e| AutomaError::BadRequest(format!("Failed to parse workflow JSON: {}", e)))?;
    Response::builder()
        .header("content-type", "application/json")
        .body(Body::from(content))
        .map_err(|e| AutomaError::Internal(format!("Failed to build response: {}", e)))
}

#[utoipa::path(
    tag = "Storage",
    put,
    path = "/api/v1/storage/workflow",
    operation_id = "save_workflow_file",
    summary = "Save or update workflow JSON file on filesystem",
    request_body = SaveWorkflowFilePayload,
    responses(
        (status = 200, description = "Workflow JSON file saved successfully", body = SaveWorkflowFileResponse)
    )
)]
pub async fn save_workflow(
    Json(payload): Json<SaveWorkflowFilePayload>,
) -> Result<Json<SaveWorkflowFileResponse>, AutomaError> {
    let safe_path = resolve_safe_workflow_path(&payload.path)?;
    if let Some(parent) = safe_path.parent()
        && !parent.exists() {
            tokio::fs::create_dir_all(parent).await
                .map_err(|e| AutomaError::Internal(format!("Failed to create parent directory: {}", e)))?;
        }
    let json_str = serde_json::to_string_pretty(&payload.content)
        .map_err(|e| AutomaError::BadRequest(format!("Failed to serialize workflow JSON: {}", e)))?;
    tokio::fs::write(&safe_path, json_str).await
        .map_err(|e| AutomaError::Internal(format!("Failed to write file: {}", e)))?;
    Ok(Json(SaveWorkflowFileResponse {
        success: true,
        message: format!("Workflow saved successfully to {}", payload.path),
    }))
}
