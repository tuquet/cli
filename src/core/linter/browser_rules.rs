use crate::core::linter::models::{LintIssue, LintSeverity};
use crate::core::linter::registry::VALID_PROXY_PROTOCOLS;

/// Validates Browser Profile JSON payload
pub fn lint_browser_payload(
    browser: &serde_json::Value,
    issues: &mut Vec<LintIssue>,
) {
    let id = browser
        .get("id")
        .or_else(|| browser.get("name"))
        .and_then(|i| i.as_str())
        .unwrap_or("");

    if id.trim().is_empty() {
        issues.push(LintIssue {
            severity: LintSeverity::Error,
            message: "Browser Profile is missing required 'id' or 'name'.".to_string(),
            path: Some("id".to_string()),
            node_id: None,
            code: Some("MISSING_BROWSER_ID".to_string()),
        });
    }

    // Validate Proxy configuration if present
    if let Some(proxy) = browser.get("proxy").and_then(|p| p.as_object()) {
        let protocol = proxy
            .get("protocol")
            .and_then(|pr| pr.as_str())
            .unwrap_or("http")
            .to_lowercase();

        if !VALID_PROXY_PROTOCOLS.contains(&protocol.as_str()) {
            issues.push(LintIssue {
                severity: LintSeverity::Error,
                message: format!(
                    "Unsupported proxy protocol '{}'. Supported protocols: http, https, socks4, socks5.",
                    protocol
                ),
                path: Some("proxy.protocol".to_string()),
                node_id: Some(id.to_string()),
                code: Some("INVALID_PROXY_PROTOCOL".to_string()),
            });
        }

        let host = proxy.get("host").and_then(|h| h.as_str()).unwrap_or("");
        if host.trim().is_empty() {
            issues.push(LintIssue {
                severity: LintSeverity::Error,
                message: "Proxy configuration is missing 'host' address.".to_string(),
                path: Some("proxy.host".to_string()),
                node_id: Some(id.to_string()),
                code: Some("EMPTY_PROXY_HOST".to_string()),
            });
        }

        if let Some(port) = proxy.get("port").and_then(|p| p.as_i64()) {
            if port <= 0 || port > 65535 {
                issues.push(LintIssue {
                    severity: LintSeverity::Error,
                    message: format!("Proxy port '{}' is outside valid TCP port range (1..65535).", port),
                    path: Some("proxy.port".to_string()),
                    node_id: Some(id.to_string()),
                    code: Some("INVALID_PROXY_PORT".to_string()),
                });
            }
        }
    }

    // Validate Viewport if present
    if let Some(viewport) = browser.get("viewport").and_then(|v| v.as_object()) {
        let width = viewport.get("width").and_then(|w| w.as_i64()).unwrap_or(0);
        let height = viewport.get("height").and_then(|h| h.as_i64()).unwrap_or(0);
        if width <= 0 || height <= 0 {
            issues.push(LintIssue {
                severity: LintSeverity::Warning,
                message: "Browser viewport dimensions 'width' and 'height' should be positive integers.".to_string(),
                path: Some("viewport".to_string()),
                node_id: Some(id.to_string()),
                code: Some("INVALID_VIEWPORT".to_string()),
            });
        }
    }
}
