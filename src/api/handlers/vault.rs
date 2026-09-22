use axum::{
    extract::Query,
    response::Response,
    body::Body,
    Json,
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use utoipa::{IntoParams, ToSchema};
use crate::core::error::AutomaError;

/// Detects the storage vault workspace directory
pub fn get_vault_root() -> PathBuf {
    let candidates = [
        PathBuf::from("automa-vault"),
        PathBuf::from("./automa-vault"),
        PathBuf::from("../automa-vault"),
        PathBuf::from("../../automa-vault"),
    ];

    for cand in &candidates {
        if cand.exists() && cand.is_dir() {
            if let Ok(canon) = cand.canonicalize() {
                return canon;
            }
            return cand.clone();
        }
    }

    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

/// Securely resolves and validates a workflow or campaign file path to prevent path traversal
pub fn resolve_safe_vault_path(raw_path: &str) -> Result<PathBuf, AutomaError> {
    if raw_path.is_empty() || raw_path.contains('\0') {
        return Err(AutomaError::BadRequest("Invalid path: empty or contains null character".into()));
    }

    let vault_root = get_vault_root();
    let path = Path::new(raw_path);

    // If relative path, join with vault_root
    let target = if path.is_relative() {
        vault_root.join(path)
    } else {
        path.to_path_buf()
    };

    // Normalize path and verify it stays inside allowed boundaries
    let normalized = if target.exists() {
        target.canonicalize().map_err(|e| AutomaError::BadRequest(format!("Failed to canonicalize path: {}", e)))?
    } else if let Some(parent) = target.parent() {
        if parent.exists() {
            let canon_parent = parent.canonicalize().map_err(|e| AutomaError::BadRequest(format!("Invalid parent directory: {}", e)))?;
            canon_parent.join(target.file_name().unwrap_or_default())
        } else {
            target
        }
    } else {
        target
    };

    // Check extension
    let file_name = normalized.file_name().and_then(|f| f.to_str()).unwrap_or_default();
    if !file_name.ends_with(".json") {
        return Err(AutomaError::BadRequest("Invalid file extension: only JSON workflow/campaign files are allowed".into()));
    }

    // Path traversal check
    if raw_path.contains("..") {
        if let Ok(canon_root) = vault_root.canonicalize() {
            if !normalized.starts_with(&canon_root) {
                return Err(AutomaError::BadRequest("Access Denied: Path traversal detected outside storage vault".into()));
            }
        }
    }

    Ok(normalized)
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct GetWorkflowParams {
    /// Full filesystem path or relative workspace path to the target .workflow.json file
    #[param(example = "workflows/search.workflow.json")]
    pub path: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[schema(example = json!({
    "path": "workflows/search.workflow.json",
    "content": {
        "name": "Search Workflow",
        "nodes": [],
        "edges": []
    }
}))]
/// Request payload for saving or updating a workflow file
pub struct SaveWorkflowPayload {
    /// Target filesystem path where the workflow should be written
    #[schema(example = "workflows/search.workflow.json")]
    pub path: String,
    /// Workflow JSON content object (nodes, edges, settings)
    #[schema(value_type = Object)]
    pub content: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[schema(example = json!({"success": true, "message": "Workflow saved successfully"}))]
/// Response returned after saving a workflow file
pub struct SaveWorkflowResponse {
    /// Whether the file was written successfully
    pub success: bool,
    /// Status description message
    pub message: String,
}

#[utoipa::path(
    tag = "Storage",
    get,
    path = "/api/v1/storage/workflow",
    operation_id = "get_workflow",
    summary = "Read workflow JSON file from filesystem",
    description = "Reads and parses an automation workflow file (.workflow.json) securely from the given filesystem path.",
    params(GetWorkflowParams),
    responses(
        (status = 200, description = "Workflow JSON file read successfully"),
        (status = 400, description = "Invalid path or unreadable workflow JSON", body = crate::core::error::ApiErrorResponse),
        (status = 404, description = "Workflow file not found on disk", body = crate::core::error::ApiErrorResponse)
    )
)]
pub async fn get_workflow(
    Query(params): Query<GetWorkflowParams>,
) -> Result<Response, AutomaError> {
    let safe_path = resolve_safe_vault_path(&params.path)?;
    if !safe_path.exists() {
        return Err(AutomaError::NotFound(format!("Workflow file does not exist: {}", params.path)));
    }

    let content = tokio::fs::read_to_string(&safe_path).await
        .map_err(|e| AutomaError::Internal(format!("Failed to read file: {}", e)))?;

    let _ = serde_json::from_str::<serde_json::Value>(&content)
        .map_err(|e| AutomaError::BadRequest(format!("Failed to parse workflow JSON: {}", e)))?;

    Ok(Response::builder()
        .header("content-type", "application/json")
        .body(Body::from(content))
        .map_err(|e| AutomaError::Internal(format!("Failed to build response: {}", e)))?)
}

#[utoipa::path(
    tag = "Storage",
    put,
    path = "/api/v1/storage/workflow",
    operation_id = "save_workflow",
    summary = "Save or update workflow JSON file",
    description = "Serializes and writes the workflow content securely to the specified path on disk, creating parent directories automatically if needed.",
    request_body = SaveWorkflowPayload,
    responses(
        (status = 200, description = "Workflow JSON file saved successfully", body = SaveWorkflowResponse),
        (status = 400, description = "Invalid payload or unwriteable path", body = crate::core::error::ApiErrorResponse)
    )
)]
pub async fn save_workflow(
    Json(payload): Json<SaveWorkflowPayload>,
) -> Result<Json<SaveWorkflowResponse>, AutomaError> {
    let safe_path = resolve_safe_vault_path(&payload.path)?;

    // Create parent directory if not exists
    if let Some(parent) = safe_path.parent() {
        if !parent.exists() {
            tokio::fs::create_dir_all(parent).await
                .map_err(|e| AutomaError::Internal(format!("Failed to create parent directory: {}", e)))?;
        }
    }

    let json_str = serde_json::to_string_pretty(&payload.content)
        .map_err(|e| AutomaError::BadRequest(format!("Failed to serialize workflow JSON: {}", e)))?;

    tokio::fs::write(&safe_path, json_str).await
        .map_err(|e| AutomaError::Internal(format!("Failed to write file: {}", e)))?;

    Ok(Json(SaveWorkflowResponse {
        success: true,
        message: format!("Workflow saved successfully to {}", payload.path),
    }))
}
