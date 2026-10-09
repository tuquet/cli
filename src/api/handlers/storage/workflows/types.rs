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

#[derive(Debug, Deserialize, IntoParams)]
pub struct GetWorkflowFileParams {
    pub path: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SaveWorkflowFilePayload {
    pub path: String,
    pub content: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SaveWorkflowFileResponse {
    pub success: bool,
    pub message: String,
}
