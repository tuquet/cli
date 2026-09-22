use crate::core::linter::browser_rules::lint_browser_payload;
use crate::core::linter::campaign_rules::lint_campaign_payload;
use crate::core::linter::models::{
    LintIssue, LintRequest, LintResponse, LintSeverity, LintTargetType,
};
use crate::core::linter::package_rules::lint_package_payload;
use crate::core::linter::workflow_rules::{lint_workflow_graph, normalize_workflow_payload};

pub struct LinterEngine;

impl LinterEngine {
    /// Detects target asset type from JSON payload structure
    pub fn detect_target_type(payload: &serde_json::Value) -> LintTargetType {
        // 1. Campaign detection
        if payload.get("browsers").is_some() || (payload.get("workflows").is_some() && payload.get("nodes").is_none()) {
            return LintTargetType::Campaign;
        }

        // 2. Browser detection
        if (payload.get("userAgent").is_some() || payload.get("proxy").is_some()) && payload.get("nodes").is_none() {
            return LintTargetType::Browser;
        }

        // 3. Package detection
        if (payload.get("inputs").is_some() || payload.get("outputs").is_some()) && payload.get("drawflow").is_none() {
            return LintTargetType::Package;
        }

        // Default to Workflow
        LintTargetType::Workflow
    }

    /// Lints any Automa asset based on request configuration
    pub fn lint(request: LintRequest) -> LintResponse {
        let mode = request.mode.unwrap_or_default();
        let mut issues = Vec::new();

        // Check if raw content is provided
        let raw_val = if let Some(content) = &request.content {
            content.clone()
        } else {
            serde_json::to_value(&request).unwrap_or(serde_json::Value::Null)
        };

        let target_type = match request.target_type.unwrap_or(LintTargetType::Auto) {
            LintTargetType::Auto => Self::detect_target_type(&raw_val),
            explicit => explicit,
        };

        match target_type {
            LintTargetType::Campaign => {
                lint_campaign_payload(&raw_val, &mut issues);
            }
            LintTargetType::Browser => {
                lint_browser_payload(&raw_val, &mut issues);
            }
            LintTargetType::Package => {
                lint_package_payload(&raw_val, mode, &mut issues);
            }
            LintTargetType::Workflow | LintTargetType::Auto => {
                let (nodes_opt, edges_opt) = normalize_workflow_payload(
                    &request.nodes,
                    &request.edges,
                    &request.drawflow.or_else(|| raw_val.get("drawflow").cloned()),
                );

                // Fallback: Check if nodes exist directly in raw_val
                let final_nodes_opt = nodes_opt.or_else(|| {
                    raw_val.get("nodes").and_then(|n| n.as_array()).cloned()
                });
                let final_edges_opt = edges_opt.or_else(|| {
                    raw_val.get("edges").and_then(|e| e.as_array()).cloned()
                });

                match final_nodes_opt {
                    Some(nodes) => {
                        let edges = final_edges_opt.unwrap_or_default();
                        lint_workflow_graph(&nodes, &edges, mode, &mut issues);
                    }
                    None => {
                        issues.push(LintIssue {
                            severity: LintSeverity::Error,
                            message: "Workflow is missing 'nodes' array or valid 'drawflow' structure".to_string(),
                            path: Some("nodes".to_string()),
                            node_id: None,
                            code: Some("MISSING_NODES_ARRAY".to_string()),
                        });
                    }
                }
            }
        }

        let valid = !issues.iter().any(|i| i.severity == LintSeverity::Error);

        LintResponse {
            valid,
            target_type,
            mode,
            issues,
        }
    }
}
