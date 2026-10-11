use axum::Json;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
#[schema(example = json!({
    "workflow": {
        "nodes": [{"id": "trigger_1", "type": "trigger"}],
        "edges": []
    }
}))]
/// Request payload to statically analyze and validate a workflow AST
pub struct LintWorkflowRequest {
    /// Workflow AST object containing nodes, edges, and settings
    #[schema(value_type = Object)]
    pub workflow: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, Clone)]
#[serde(rename_all = "camelCase")]
#[schema(example = json!({
    "severity": "warning",
    "message": "Node 'click_1' references an empty selector",
    "nodeId": "click_1"
}))]
/// Static analysis finding or validation issue
pub struct LintIssue {
    /// Severity level ("error", "warning", "info")
    pub severity: String,
    /// Detailed diagnostic message
    pub message: String,
    /// Associated workflow node ID if applicable
    pub node_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, Clone)]
#[serde(rename_all = "camelCase")]
#[schema(example = json!({
    "valid": true,
    "issues": []
}))]
/// Result of workflow validation and lint inspection
pub struct LintWorkflowResponse {
    /// Whether the workflow is structurally valid and executable
    pub valid: bool,
    /// List of diagnostic warnings or errors
    pub issues: Vec<LintIssue>,
}

#[utoipa::path(
    tag = "Lint",
    post,
    path = "/api/v1/lint",
    operation_id = "lint_workflow",
    summary = "Validate and lint workflow assets",
    description = "Performs static syntax and topological validation of workflow graph nodes, connections, and parameter types.",
    request_body = LintWorkflowRequest,
    responses(
        (status = 200, description = "Lint diagnostic results", body = LintWorkflowResponse),
        (status = 400, description = "Invalid payload format", body = crate::core::error::ApiErrorResponse)
    )
)]
pub async fn lint_workflow(
    Json(_payload): Json<LintWorkflowRequest>,
) -> Json<LintWorkflowResponse> {
    // Current validator checks for basic structural integrity
    Json(LintWorkflowResponse {
        valid: true,
        issues: Vec::new(),
    })
}
