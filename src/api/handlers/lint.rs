use axum::Json;
use utoipa;
pub use crate::core::linter::{LinterEngine, LintIssue, LintMode, LintRequest, LintResponse, LintSeverity, LintTargetType};
pub use crate::core::linter::{AUTOMA_KNOWN_BLOCK_LABELS, AUTOMA_KNOWN_NODE_TYPES};

#[utoipa::path(
    tag = "Lint",
    post,
    path = "/api/v1/lint",
    operation_id = "lint_workflow",
    summary = "Validate and lint Automa assets (Workflows, Campaigns, Browsers, Packages)",
    description = "Performs static analysis on AST graph nodes, edge connections, parameter schemas, campaign schedules, and browser configs.",
    request_body = LintRequest,
    responses(
        (status = 200, description = "Lint diagnostic results", body = LintResponse)
    )
)]
pub async fn lint_workflow(
    Json(payload): Json<LintRequest>,
) -> Json<LintResponse> {
    let response = LinterEngine::lint(payload);
    Json(response)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_lint_valid_workflow() {
        let req = LintRequest {
            mode: Some(LintMode::Editor),
            target_type: Some(LintTargetType::Workflow),
            nodes: Some(vec![json!({
                "id": "trigger_1",
                "type": "BlockBasic",
                "label": "trigger",
                "data": {}
            })]),
            edges: Some(vec![]),
            drawflow: None,
            content: None,
        };
        let res = lint_workflow(Json(req)).await;
        assert!(res.valid);
        assert!(res.issues.is_empty());
    }

    #[tokio::test]
    async fn test_lint_missing_node_id_error() {
        let req = LintRequest {
            mode: Some(LintMode::Editor),
            target_type: Some(LintTargetType::Workflow),
            nodes: Some(vec![json!({"name": "Unnamed node"})]),
            edges: Some(vec![]),
            drawflow: None,
            content: None,
        };
        let res = lint_workflow(Json(req)).await;
        assert!(!res.valid);
        assert!(res.issues.iter().any(|i| i.severity == LintSeverity::Error && i.code.as_deref() == Some("MISSING_NODE_ID")));
    }

    #[tokio::test]
    async fn test_lint_invalid_node_id_format() {
        let req = LintRequest {
            mode: Some(LintMode::Runner),
            target_type: Some(LintTargetType::Workflow),
            nodes: Some(vec![json!({
                "id": "my bing search node",
                "type": "BlockBasic",
                "label": "trigger",
                "data": {}
            })]),
            edges: Some(vec![]),
            drawflow: None,
            content: None,
        };
        let res = lint_workflow(Json(req)).await;
        assert!(!res.valid);
        assert!(res.issues.iter().any(|i| i.code.as_deref() == Some("INVALID_NODE_ID_FORMAT")));
    }

    #[tokio::test]
    async fn test_lint_unknown_block_label_ai_hallucination() {
        let req = LintRequest {
            mode: Some(LintMode::Editor),
            target_type: Some(LintTargetType::Workflow),
            nodes: Some(vec![
                json!({ "id": "t1", "type": "BlockBasic", "label": "trigger", "data": {} }),
                json!({ "id": "b1", "type": "BlockBasic", "label": "bing-search", "data": {} })
            ]),
            edges: Some(vec![json!({ "source": "t1", "target": "b1" })]),
            drawflow: None,
            content: None,
        };
        let res = lint_workflow(Json(req)).await;
        assert!(!res.valid);
        assert!(res.issues.iter().any(|i| i.code.as_deref() == Some("UNKNOWN_BLOCK_LABEL")));
    }

    #[tokio::test]
    async fn test_lint_forbidden_extract_data_block() {
        let req = LintRequest {
            mode: Some(LintMode::Editor),
            target_type: Some(LintTargetType::Workflow),
            nodes: Some(vec![
                json!({ "id": "t1", "type": "BlockBasic", "label": "trigger", "data": {} }),
                json!({ "id": "e1", "type": "BlockBasic", "label": "extract-data", "data": {} })
            ]),
            edges: Some(vec![json!({ "source": "t1", "target": "e1" })]),
            drawflow: None,
            content: None,
        };
        let res = lint_workflow(Json(req)).await;
        assert!(!res.valid);
        assert!(res.issues.iter().any(|i| i.code.as_deref() == Some("FORBIDDEN_EXTRACT_DATA")));
    }

    #[tokio::test]
    async fn test_lint_campaign_valid_and_invalid_cron() {
        let invalid_req = LintRequest {
            mode: Some(LintMode::Editor),
            target_type: Some(LintTargetType::Campaign),
            nodes: None,
            edges: None,
            drawflow: None,
            content: Some(json!({
                "name": "Daily Scraping",
                "browsers": [{
                    "id": "chrome_1",
                    "tasks": [{
                        "workflow_id": "scrape.workflow.json",
                        "schedule": "cron: invalid_cron_str"
                    }]
                }]
            })),
        };
        let res = lint_workflow(Json(invalid_req)).await;
        assert!(!res.valid);
        assert!(res.issues.iter().any(|i| i.code.as_deref() == Some("INVALID_CRON_EXPRESSION")));

        let valid_req = LintRequest {
            mode: Some(LintMode::Editor),
            target_type: Some(LintTargetType::Campaign),
            nodes: None,
            edges: None,
            drawflow: None,
            content: Some(json!({
                "name": "Daily Scraping",
                "browsers": [{
                    "id": "chrome_1",
                    "tasks": [{
                        "workflow_id": "scrape.workflow.json",
                        "schedule": "cron: 0 */2 * * *"
                    }]
                }]
            })),
        };
        let valid_res = lint_workflow(Json(valid_req)).await;
        assert!(valid_res.valid);
    }

    #[tokio::test]
    async fn test_lint_browser_proxy_validation() {
        let invalid_proxy_req = LintRequest {
            mode: Some(LintMode::Editor),
            target_type: Some(LintTargetType::Browser),
            nodes: None,
            edges: None,
            drawflow: None,
            content: Some(json!({
                "id": "browser_us",
                "proxy": {
                    "protocol": "ftp",
                    "host": "proxy.example.com",
                    "port": 99999
                }
            })),
        };
        let res = lint_workflow(Json(invalid_proxy_req)).await;
        assert!(!res.valid);
        assert!(res.issues.iter().any(|i| i.code.as_deref() == Some("INVALID_PROXY_PROTOCOL")));
        assert!(res.issues.iter().any(|i| i.code.as_deref() == Some("INVALID_PROXY_PORT")));
    }
}
