use crate::core::linter::models::{LintIssue, LintSeverity};

/// Validates 5-field cron syntax (e.g. "0 */2 * * *")
pub fn is_valid_cron_expression(cron_str: &str) -> bool {
    let clean = cron_str.strip_prefix("cron:").unwrap_or(cron_str).trim();
    let fields: Vec<&str> = clean.split_whitespace().collect();
    if fields.len() != 5 {
        return false;
    }

    // Basic syntax sanity for 5 fields: minute, hour, day-of-month, month, day-of-week
    fields.iter().all(|f| {
        f.chars().all(|c| {
            c.is_ascii_alphanumeric()
                || c == '*'
                || c == '/'
                || c == ','
                || c == '-'
                || c == '?'
        })
    })
}

/// Validates Campaign JSON payload
pub fn lint_campaign_payload(
    campaign: &serde_json::Value,
    issues: &mut Vec<LintIssue>,
) {
    let name = campaign.get("name").and_then(|n| n.as_str()).unwrap_or("");
    if name.trim().is_empty() {
        issues.push(LintIssue {
            severity: LintSeverity::Error,
            message: "Campaign is missing required 'name' attribute.".to_string(),
            path: Some("name".to_string()),
            node_id: None,
            code: Some("MISSING_CAMPAIGN_NAME".to_string()),
        });
    }

    if let Some(concurrency) = campaign.get("concurrency").and_then(|c| c.as_i64()) {
        if concurrency <= 0 || concurrency > 100 {
            issues.push(LintIssue {
                severity: LintSeverity::Warning,
                message: format!(
                    "Campaign concurrency '{}' should be a positive integer between 1 and 100.",
                    concurrency
                ),
                path: Some("concurrency".to_string()),
                node_id: None,
                code: Some("INVALID_CONCURRENCY".to_string()),
            });
        }
    }

    let browsers = match campaign.get("browsers").and_then(|b| b.as_array()) {
        Some(arr) if !arr.is_empty() => arr,
        Some(_) => {
            issues.push(LintIssue {
                severity: LintSeverity::Warning,
                message: "Campaign 'browsers' list is empty. No tasks will be executed.".to_string(),
                path: Some("browsers".to_string()),
                node_id: None,
                code: Some("EMPTY_CAMPAIGN_BROWSERS".to_string()),
            });
            return;
        }
        None => {
            // Check legacy format with flat workflows list
            if let Some(workflows) = campaign.get("workflows").and_then(|w| w.as_array()) {
                if workflows.is_empty() {
                    issues.push(LintIssue {
                        severity: LintSeverity::Warning,
                        message: "Campaign 'workflows' list is empty.".to_string(),
                        path: Some("workflows".to_string()),
                        node_id: None,
                        code: Some("EMPTY_CAMPAIGN_WORKFLOWS".to_string()),
                    });
                }
                return;
            }

            issues.push(LintIssue {
                severity: LintSeverity::Error,
                message: "Campaign is missing required 'browsers' array.".to_string(),
                path: Some("browsers".to_string()),
                node_id: None,
                code: Some("MISSING_CAMPAIGN_BROWSERS".to_string()),
            });
            return;
        }
    };

    for (b_idx, browser) in browsers.iter().enumerate() {
        let b_id = browser
            .get("id")
            .or_else(|| browser.get("browserId"))
            .or_else(|| browser.get("name"))
            .and_then(|i| i.as_str())
            .unwrap_or("");

        if b_id.trim().is_empty() {
            issues.push(LintIssue {
                severity: LintSeverity::Warning,
                message: format!("Campaign browser #{} is missing 'id' or 'name'.", b_idx + 1),
                path: Some(format!("browsers[{}].id", b_idx)),
                node_id: None,
                code: Some("MISSING_BROWSER_ID".to_string()),
            });
        }

        if let Some(tasks) = browser.get("tasks").and_then(|t| t.as_array()) {
            if tasks.is_empty() {
                issues.push(LintIssue {
                    severity: LintSeverity::Warning,
                    message: format!(
                        "Browser '{}' has an empty 'tasks' list in campaign.",
                        b_id
                    ),
                    path: Some(format!("browsers[{}].tasks", b_idx)),
                    node_id: Some(b_id.to_string()),
                    code: Some("EMPTY_BROWSER_TASKS".to_string()),
                });
            }

            for (t_idx, task) in tasks.iter().enumerate() {
                let wf_id = task
                    .get("workflow_id")
                    .or_else(|| task.get("workflowId"))
                    .or_else(|| task.get("id"))
                    .and_then(|w| w.as_str())
                    .unwrap_or("");

                if wf_id.trim().is_empty() {
                    issues.push(LintIssue {
                        severity: LintSeverity::Error,
                        message: format!(
                            "Task #{} on browser '{}' is missing 'workflow_id'.",
                            t_idx + 1, b_id
                        ),
                        path: Some(format!("browsers[{}].tasks[{}].workflow_id", b_idx, t_idx)),
                        node_id: Some(b_id.to_string()),
                        code: Some("MISSING_TASK_WORKFLOW_ID".to_string()),
                    });
                }

                if let Some(schedule) = task.get("schedule").and_then(|s| s.as_str()) {
                    let trimmed_sched = schedule.trim();
                    if trimmed_sched.starts_with("cron:") {
                        let cron_expr = trimmed_sched.strip_prefix("cron:").unwrap_or("").trim();
                        if !is_valid_cron_expression(cron_expr) {
                            issues.push(LintIssue {
                                severity: LintSeverity::Error,
                                message: format!(
                                    "Invalid cron expression '{}' in task #{} on browser '{}'. Expected 5 standard fields (e.g. '0 */2 * * *').",
                                    cron_expr, t_idx + 1, b_id
                                ),
                                path: Some(format!("browsers[{}].tasks[{}].schedule", b_idx, t_idx)),
                                node_id: Some(b_id.to_string()),
                                code: Some("INVALID_CRON_EXPRESSION".to_string()),
                            });
                        }
                    }
                }
            }
        }
    }
}
