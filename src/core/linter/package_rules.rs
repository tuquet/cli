use crate::core::linter::models::{LintIssue, LintMode, LintSeverity};
use crate::core::linter::workflow_rules::lint_workflow_graph;

/// Validates Package JSON payload
pub fn lint_package_payload(
    package: &serde_json::Value,
    mode: LintMode,
    issues: &mut Vec<LintIssue>,
) {
    let name = package
        .get("name")
        .or_else(|| package.get("id"))
        .and_then(|n| n.as_str())
        .unwrap_or("");

    if name.trim().is_empty() {
        issues.push(LintIssue {
            severity: LintSeverity::Error,
            message: "Package is missing required 'name' or 'id' property.".to_string(),
            path: Some("name".to_string()),
            node_id: None,
            code: Some("MISSING_PACKAGE_NAME".to_string()),
        });
    }

    // Inspect nested workflow nodes & edges if present
    if let Some(data) = package.get("data") {
        let nodes = data.get("nodes").and_then(|n| n.as_array()).cloned().unwrap_or_default();
        let edges = data.get("edges").and_then(|e| e.as_array()).cloned().unwrap_or_default();

        if !nodes.is_empty() {
            lint_workflow_graph(&nodes, &edges, mode, issues);
        }
    }
}
