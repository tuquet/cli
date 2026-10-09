use axum::extract::{Json, State};
use crate::AppState;
use crate::core::error::AutomaError;

use super::types::{ImportWorkflowStorageRequest, WorkflowStorageItem};

#[utoipa::path(
    tag = "Storage",
    post,
    path = "/api/v1/storage/workflows/import",
    operation_id = "import_storage_workflow",
    summary = "Import a workflow JSON into storage database",
    description = "Parses and imports a workflow JSON object (from file or client) into SQLite database.",
    request_body = ImportWorkflowStorageRequest,
    responses(
        (status = 200, description = "Workflow imported successfully", body = WorkflowStorageItem),
        (status = 400, description = "Invalid workflow JSON", body = crate::core::error::ApiErrorResponse),
        (status = 500, description = "Database write error", body = crate::core::error::ApiErrorResponse)
    )
)]
pub async fn import_storage_workflow(
    State(state): State<AppState>,
    Json(payload): Json<ImportWorkflowStorageRequest>,
) -> Result<Json<WorkflowStorageItem>, AutomaError> {
    let wf = &payload.workflow;
    if !wf.is_object() {
        return Err(AutomaError::BadRequest("Import payload must contain a valid JSON object".into()));
    }

    let id = payload.id
        .or_else(|| wf.get("id").and_then(|v| v.as_str()).map(|s| s.to_string()))
        .unwrap_or_else(|| format!("wf_{}", uuid::Uuid::new_v4().simple()));

    let name = wf.get("name")
        .and_then(|v| v.as_str())
        .unwrap_or(&id)
        .to_string();

    let description = wf.get("description")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let version = wf.get("version")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let icon = wf.get("icon")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let data_str = serde_json::to_string(wf)
        .map_err(|e| AutomaError::BadRequest(format!("Failed to serialize workflow: {}", e)))?;

    let db = state.db.lock().await;
    let created = db.workflows().create_workflow(
        &id,
        &name,
        description.as_deref(),
        &data_str,
        version.as_deref(),
        icon.as_deref(),
    ).map_err(|e| AutomaError::DatabaseError(e.to_string()))?;

    // Also import nested sub-workflows from `includedWorkflows` if present (automa-webe export standard)
    if let Some(included) = wf.get("includedWorkflows").and_then(|v| v.as_object()) {
        for (sub_id, sub_wf) in included {
            if sub_wf.is_object() {
                let sub_name = sub_wf.get("name").and_then(|v| v.as_str()).unwrap_or(sub_id);
                let sub_desc = sub_wf.get("description").and_then(|v| v.as_str());
                let sub_ver = sub_wf.get("version").and_then(|v| v.as_str());
                let sub_icon = sub_wf.get("icon").and_then(|v| v.as_str());
                if let Ok(sub_data_str) = serde_json::to_string(sub_wf) {
                    let _ = db.workflows().create_workflow(
                        sub_id,
                        sub_name,
                        sub_desc,
                        &sub_data_str,
                        sub_ver,
                        sub_icon,
                    );
                }
            }
        }
    }

    let parsed_data = serde_json::from_str::<serde_json::Value>(&created.data)
        .unwrap_or(serde_json::json!({}));

    Ok(Json(WorkflowStorageItem {
        id: created.id,
        name: created.name,
        description: created.description,
        data: parsed_data,
        version: created.version,
        icon: created.icon,
        created_at: created.created_at,
        updated_at: created.updated_at,
    }))
}
