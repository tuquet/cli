use axum::{
    extract::{Path, Json, Query, State},
};
use crate::AppState;
use crate::core::error::AutomaError;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

#[derive(Debug, Serialize, Deserialize, ToSchema, Clone)]
#[serde(rename_all = "camelCase")]
#[schema(example = json!({
    "id": "wf_bing_search",
    "name": "Bing Search Automation",
    "description": "Searches Bing and extracts results",
    "data": {
        "nodes": [{"id": "node_1", "type": "trigger", "label": "trigger"}],
        "edges": []
    },
    "version": "1.0.0",
    "icon": "search",
    "createdAt": "2026-08-26T00:00:00Z",
    "updatedAt": "2026-08-26T00:00:00Z"
}))]
/// Workflow descriptor stored in central SQLite database
pub struct WorkflowStorageItem {
    /// Unique workflow identifier
    pub id: String,
    /// Workflow display name
    pub name: String,
    /// Optional workflow description
    pub description: Option<String>,
    /// Workflow graph AST (nodes, edges, settings)
    #[schema(value_type = Object)]
    pub data: serde_json::Value,
    /// Semantic version of the workflow
    pub version: String,
    /// Optional UI icon identifier
    pub icon: Option<String>,
    /// Creation timestamp (ISO 8601)
    pub created_at: String,
    /// Last modification timestamp (ISO 8601)
    pub updated_at: String,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
#[schema(example = json!({
    "id": "wf_bing_search",
    "name": "Bing Search Automation",
    "description": "Searches Bing and extracts results",
    "data": {
        "nodes": [{"id": "node_1", "type": "trigger", "label": "trigger"}],
        "edges": []
    },
    "version": "1.0.0",
    "icon": "search"
}))]
/// Request payload for creating a new workflow in SQLite database
pub struct CreateWorkflowStorageRequest {
    /// Optional custom identifier (auto-generated if omitted)
    pub id: Option<String>,
    /// Workflow display name
    pub name: String,
    /// Optional workflow description
    pub description: Option<String>,
    /// Workflow graph AST (nodes, edges, settings)
    #[schema(value_type = Object)]
    pub data: serde_json::Value,
    /// Optional version (defaults to 1.0.0)
    pub version: Option<String>,
    /// Optional UI icon name
    pub icon: Option<String>,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
#[schema(example = json!({
    "name": "Updated Bing Search Automation",
    "description": "Updated description",
    "data": {
        "nodes": [{"id": "node_1", "type": "trigger", "label": "trigger"}],
        "edges": []
    },
    "version": "1.1.0",
    "icon": "search"
}))]
/// Request payload for updating an existing workflow in SQLite database
pub struct UpdateWorkflowStorageRequest {
    /// Optional updated name
    pub name: Option<String>,
    /// Optional updated description
    pub description: Option<String>,
    /// Optional updated workflow graph AST
    #[schema(value_type = Option<Object>)]
    pub data: Option<serde_json::Value>,
    /// Optional updated version
    pub version: Option<String>,
    /// Optional updated icon
    pub icon: Option<String>,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
#[schema(example = json!({
    "workflow": {
        "name": "Imported Google Search",
        "nodes": [],
        "edges": []
    }
}))]
/// Request payload for importing a workflow into SQLite database
pub struct ImportWorkflowStorageRequest {
    /// Optional custom ID to assign
    pub id: Option<String>,
    /// Raw workflow JSON content
    #[schema(value_type = Object)]
    pub workflow: serde_json::Value,
}

#[derive(Deserialize, IntoParams)]
pub struct GetWorkflowsQuery {
    /// Maximum number of workflows to return
    #[param(example = 50)]
    pub limit: Option<usize>,
    /// Number of items to skip for pagination (default 0)
    #[param(example = 0)]
    pub offset: Option<usize>,
    /// Optional search query to filter workflows by name, ID or description
    pub search: Option<String>,
}

#[utoipa::path(
    tag = "Storage",
    get,
    path = "/api/v1/storage/workflows",
    operation_id = "get_storage_workflows",
    summary = "List all workflows from storage database",
    description = "Retrieves all workflow items persisted in the central SQLite database.",
    params(GetWorkflowsQuery),
    responses(
        (status = 200, description = "List of workflows", body = Vec<WorkflowStorageItem>),
        (status = 500, description = "Database read error", body = crate::core::error::ApiErrorResponse)
    )
)]
pub async fn get_workflows(
    State(state): State<AppState>,
    Query(query): Query<GetWorkflowsQuery>,
) -> Result<Json<Vec<WorkflowStorageItem>>, AutomaError> {
    let db = state.db.lock().await;
    let list = db.workflows().get_workflows(query.limit, query.offset, query.search.as_deref())
        .map_err(|e| AutomaError::DatabaseError(e.to_string()))?;

    let items = list.into_iter().map(|w| {
        let parsed_data = serde_json::from_str::<serde_json::Value>(&w.data)
            .unwrap_or(serde_json::json!({}));
        WorkflowStorageItem {
            id: w.id,
            name: w.name,
            description: w.description,
            data: parsed_data,
            version: w.version,
            icon: w.icon,
            created_at: w.created_at,
            updated_at: w.updated_at,
        }
    }).collect();

    Ok(Json(items))
}

#[utoipa::path(
    tag = "Storage",
    get,
    path = "/api/v1/storage/workflows/{id}",
    operation_id = "get_storage_workflow",
    summary = "Get a workflow by ID from storage database",
    description = "Retrieves the full AST and metadata of a workflow by its unique ID from SQLite database.",
    params(
        ("id" = String, Path, description = "Unique workflow identifier")
    ),
    responses(
        (status = 200, description = "Workflow details", body = WorkflowStorageItem),
        (status = 404, description = "Workflow not found", body = crate::core::error::ApiErrorResponse),
        (status = 500, description = "Database read error", body = crate::core::error::ApiErrorResponse)
    )
)]
pub async fn get_workflow_by_id(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<WorkflowStorageItem>, AutomaError> {
    let db = state.db.lock().await;
    let item = db.workflows().get_workflow(&id)
        .map_err(|e| AutomaError::DatabaseError(e.to_string()))?
        .ok_or_else(|| AutomaError::NotFound(format!("Workflow '{}' not found in database", id)))?;

    let parsed_data = serde_json::from_str::<serde_json::Value>(&item.data)
        .unwrap_or(serde_json::json!({}));

    Ok(Json(WorkflowStorageItem {
        id: item.id,
        name: item.name,
        description: item.description,
        data: parsed_data,
        version: item.version,
        icon: item.icon,
        created_at: item.created_at,
        updated_at: item.updated_at,
    }))
}

#[utoipa::path(
    tag = "Storage",
    post,
    path = "/api/v1/storage/workflows",
    operation_id = "create_storage_workflow",
    summary = "Create or persist a workflow in database",
    description = "Creates a new workflow record or updates an existing record in the SQLite database.",
    request_body = CreateWorkflowStorageRequest,
    responses(
        (status = 200, description = "Workflow created successfully", body = WorkflowStorageItem),
        (status = 400, description = "Invalid workflow payload", body = crate::core::error::ApiErrorResponse),
        (status = 500, description = "Database write error", body = crate::core::error::ApiErrorResponse)
    )
)]
pub async fn create_storage_workflow(
    State(state): State<AppState>,
    Json(payload): Json<CreateWorkflowStorageRequest>,
) -> Result<Json<WorkflowStorageItem>, AutomaError> {
    let id = payload.id.unwrap_or_else(|| format!("wf_{}", uuid::Uuid::new_v4().simple()));
    let data_str = serde_json::to_string(&payload.data)
        .map_err(|e| AutomaError::BadRequest(format!("Invalid workflow JSON: {}", e)))?;

    let db = state.db.lock().await;
    let created = db.workflows().create_workflow(
        &id,
        &payload.name,
        payload.description.as_deref(),
        &data_str,
        payload.version.as_deref(),
        payload.icon.as_deref(),
    ).map_err(|e| AutomaError::DatabaseError(e.to_string()))?;

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

#[utoipa::path(
    tag = "Storage",
    put,
    path = "/api/v1/storage/workflows/{id}",
    operation_id = "update_storage_workflow",
    summary = "Update a workflow in storage database",
    description = "Modifies the metadata or graph AST of an existing workflow in SQLite database.",
    params(
        ("id" = String, Path, description = "Unique workflow identifier")
    ),
    request_body = UpdateWorkflowStorageRequest,
    responses(
        (status = 200, description = "Workflow updated successfully", body = WorkflowStorageItem),
        (status = 400, description = "Invalid update payload", body = crate::core::error::ApiErrorResponse),
        (status = 404, description = "Workflow not found", body = crate::core::error::ApiErrorResponse),
        (status = 500, description = "Database write error", body = crate::core::error::ApiErrorResponse)
    )
)]
pub async fn update_storage_workflow(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<UpdateWorkflowStorageRequest>,
) -> Result<Json<WorkflowStorageItem>, AutomaError> {
    let data_str = payload.data.as_ref().map(|d| serde_json::to_string(d)).transpose()
        .map_err(|e| AutomaError::BadRequest(format!("Invalid workflow JSON: {}", e)))?;

    let db = state.db.lock().await;
    let updated = db.workflows().upsert_workflow(
        &id,
        payload.name.as_deref(),
        payload.description.as_deref(),
        data_str.as_deref(),
        payload.version.as_deref(),
        payload.icon.as_deref(),
    ).map_err(|e| AutomaError::DatabaseError(e.to_string()))?;

    let _ = state.tx.send(serde_json::json!({
        "type": "storage_changed",
        "entity": "workflow",
        "action": "upsert",
        "id": updated.id,
        "name": updated.name
    }).to_string());

    let parsed_data = serde_json::from_str::<serde_json::Value>(&updated.data)
        .unwrap_or(serde_json::json!({}));

    Ok(Json(WorkflowStorageItem {
        id: updated.id,
        name: updated.name,
        description: updated.description,
        data: parsed_data,
        version: updated.version,
        icon: updated.icon,
        created_at: updated.created_at,
        updated_at: updated.updated_at,
    }))
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
#[schema(example = json!({"success": true, "message": "Workflow 'wf_1' deleted"}))]
/// Response returned after successfully deleting a workflow
pub struct DeleteWorkflowResponse {
    /// True if deletion succeeded
    pub success: bool,
    /// Confirmation message
    pub message: String,
}

#[utoipa::path(
    tag = "Storage",
    delete,
    path = "/api/v1/storage/workflows/{id}",
    operation_id = "delete_storage_workflow",
    summary = "Delete a workflow from storage database",
    description = "Removes a workflow record from the central SQLite database by ID.",
    params(
        ("id" = String, Path, description = "Unique workflow identifier")
    ),
    responses(
        (status = 200, description = "Workflow deleted successfully", body = DeleteWorkflowResponse),
        (status = 404, description = "Workflow not found", body = crate::core::error::ApiErrorResponse),
        (status = 500, description = "Database deletion error", body = crate::core::error::ApiErrorResponse)
    )
)]
pub async fn delete_storage_workflow(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<DeleteWorkflowResponse>, AutomaError> {
    let db = state.db.lock().await;
    let deleted = db.workflows().delete_workflow(&id)
        .map_err(|e| AutomaError::DatabaseError(e.to_string()))?;

    if !deleted {
        return Err(AutomaError::NotFound(format!("Workflow '{}' not found in database", id)));
    }

    let _ = state.tx.send(serde_json::json!({
        "type": "storage_changed",
        "entity": "workflow",
        "action": "delete",
        "id": id
    }).to_string());

    Ok(Json(DeleteWorkflowResponse {
        success: true,
        message: format!("Workflow '{}' deleted", id),
    }))
}

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
