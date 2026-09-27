use axum::Json;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum LintMode {
    Editor,
    Runner,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum LintTargetType {
    Workflow,
    Campaign,
    Browser,
    Package,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum LintSeverity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct LintIssue {
    pub severity: LintSeverity,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct LintRequest {
    #[serde(default)]
    pub mode: Option<LintMode>,
    #[serde(default)]
    pub target_type: Option<LintTargetType>,
    #[serde(default)]
    pub nodes: Option<Vec<serde_json::Value>>,
    #[serde(default)]
    pub edges: Option<Vec<serde_json::Value>>,
    #[serde(default)]
    pub drawflow: Option<serde_json::Value>,
    #[serde(default)]
    pub content: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct LintResponse {
    pub valid: bool,
    pub issues: Vec<LintIssue>,
}

#[utoipa::path(
    tag = "Lint",
    post,
    path = "/api/v1/lint",
    operation_id = "lint_workflow",
    summary = "Validate and lint Automa assets (Bridge Stub)",
    description = "Bridge passthrough stub. In the Tuquet architecture, workflow graph validation is performed client-side by Web Studio and runtime blocks are executed by the browser worker.",
    request_body = LintRequest,
    responses(
        (status = 200, description = "Lint diagnostic results", body = LintResponse)
    )
)]
pub async fn lint_workflow(
    Json(_payload): Json<LintRequest>,
) -> Json<LintResponse> {
    Json(LintResponse {
        valid: true,
        issues: vec![],
    })
}
