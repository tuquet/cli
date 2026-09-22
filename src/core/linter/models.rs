use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Execution context mode for the linter
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum LintMode {
    /// Permissive mode for IDE/Editor: reports non-critical deviations as warnings
    Editor,
    /// Strict mode for Runner: enforces all runtime prerequisites before execution
    Runner,
}

impl Default for LintMode {
    fn default() -> Self {
        Self::Editor
    }
}

/// Target asset type being validated
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum LintTargetType {
    Workflow,
    Campaign,
    Browser,
    Package,
    Auto,
}

impl Default for LintTargetType {
    fn default() -> Self {
        Self::Auto
    }
}

/// Severity level of detected lint diagnostic
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum LintSeverity {
    Error,
    Warning,
    Info,
}

impl std::fmt::Display for LintSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Error => write!(f, "error"),
            Self::Warning => write!(f, "warning"),
            Self::Info => write!(f, "info"),
        }
    }
}

/// Diagnostic issue report detected by the linter
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
#[schema(example = json!({
    "severity": "warning",
    "message": "Node is missing property 'type'",
    "path": "nodes[0]",
    "nodeId": "trigger_1",
    "code": "MISSING_NODE_TYPE"
}))]
pub struct LintIssue {
    /// Severity level ("warning", "error", "info")
    pub severity: LintSeverity,
    /// Detailed diagnostic explanation
    pub message: String,
    /// JSONPath location of the affected element
    pub path: Option<String>,
    /// Node ID associated with this diagnostic issue, if applicable
    pub node_id: Option<String>,
    /// Machine-readable rule code for IDE filtering and categorization
    pub code: Option<String>,
}

/// Request payload to lint workflows, campaigns, browsers, or packages
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
#[schema(example = json!({
    "mode": "editor",
    "nodes": [{"id": "trigger_1", "type": "BlockBasic", "label": "trigger", "data": {}}],
    "edges": []
}))]
pub struct LintRequest {
    /// Lint evaluation mode (default: "editor")
    #[serde(default)]
    pub mode: Option<LintMode>,
    /// Explicit target type or "auto" to detect from payload
    #[serde(default)]
    pub target_type: Option<LintTargetType>,
    /// Array of workflow block node definitions
    #[schema(value_type = Option<Vec<Object>>)]
    pub nodes: Option<Vec<serde_json::Value>>,
    /// Array of workflow edge connection descriptors
    #[schema(value_type = Option<Vec<Object>>)]
    pub edges: Option<Vec<serde_json::Value>>,
    /// Optional entire workflow payload containing nested drawflow
    #[schema(value_type = Option<Object>)]
    pub drawflow: Option<serde_json::Value>,
    /// Optional raw JSON content or document for auto-detection
    #[schema(value_type = Option<Object>)]
    pub content: Option<serde_json::Value>,
}

/// Result of static AST and schema validation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
#[schema(example = json!({
    "valid": true,
    "targetType": "workflow",
    "mode": "editor",
    "issues": [{
        "severity": "warning",
        "message": "Workflow contains no trigger block to start execution.",
        "path": "nodes",
        "nodeId": null,
        "code": "NO_TRIGGER_NODE"
    }]
}))]
pub struct LintResponse {
    /// Whether the asset passed validation without errors
    pub valid: bool,
    /// Detected asset target type
    pub target_type: LintTargetType,
    /// Mode used during evaluation
    pub mode: LintMode,
    /// List of detected validation warnings, errors, and info diagnostics
    pub issues: Vec<LintIssue>,
}
