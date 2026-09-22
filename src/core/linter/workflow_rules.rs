use std::collections::{HashMap, HashSet};
use crate::core::linter::models::{LintIssue, LintMode, LintSeverity};
use crate::core::linter::registry::{
    AUTOMA_KNOWN_BLOCK_LABELS, AUTOMA_KNOWN_NODE_TYPES, EXCLUDE_GROUP_BLOCKS, EXCLUDE_ON_ERROR,
    VALID_COMPARE_TYPES,
};

/// Validates whether a Node ID conforms to URL/DOM-safe standard
pub fn is_valid_node_id(id: &str) -> bool {
    let trimmed = id.trim();
    if trimmed.is_empty() || trimmed.len() > 64 {
        return false;
    }
    trimmed
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// Validates whether an Edge ID conforms to VueFlow/Automa standard
pub fn is_valid_edge_id(id: &str) -> bool {
    let trimmed = id.trim();
    if trimmed.is_empty() || trimmed.len() > 128 {
        return false;
    }
    trimmed
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == ':')
}

/// Normalizes workflow nodes and edges from either direct arrays or nested drawflow
pub fn normalize_workflow_payload(
    nodes_opt: &Option<Vec<serde_json::Value>>,
    edges_opt: &Option<Vec<serde_json::Value>>,
    drawflow_opt: &Option<serde_json::Value>,
) -> (Option<Vec<serde_json::Value>>, Option<Vec<serde_json::Value>>) {
    if nodes_opt.is_some() || edges_opt.is_some() {
        return (nodes_opt.clone(), edges_opt.clone());
    }

    if let Some(drawflow) = drawflow_opt {
        // Modern drawflow format: { nodes: [...], edges: [...] }
        if let Some(nodes) = drawflow.get("nodes").and_then(|n| n.as_array()) {
            let edges = drawflow
                .get("edges")
                .and_then(|e| e.as_array())
                .cloned()
                .unwrap_or_default();
            return (Some(nodes.clone()), Some(edges));
        }

        // Legacy drawflow format: { Home: { data: { "n1": { id: "n1", ... } } } }
        if let Some(home_data) = drawflow
            .get("Home")
            .and_then(|h| h.get("data"))
            .and_then(|d| d.as_object())
        {
            let nodes: Vec<serde_json::Value> = home_data.values().cloned().collect();
            return (Some(nodes), Some(Vec::new()));
        }
    }

    (None, None)
}

/// Executes complete AST graph validation on workflow nodes and edges
pub fn lint_workflow_graph(
    nodes: &[serde_json::Value],
    edges: &[serde_json::Value],
    mode: LintMode,
    issues: &mut Vec<LintIssue>,
) {
    let mut node_ids = HashSet::new();
    let mut node_map: HashMap<String, (usize, &serde_json::Value)> = HashMap::new();
    let mut trigger_count = 0;

    for (i, node) in nodes.iter().enumerate() {
        let id_val = node.get("id").and_then(|v| v.as_str());

        let node_id = match id_val {
            Some(id) if !id.trim().is_empty() => {
                let id_str = id.trim().to_string();

                if !is_valid_node_id(&id_str) {
                    let severity = match mode {
                        LintMode::Editor => LintSeverity::Warning,
                        LintMode::Runner => LintSeverity::Error,
                    };
                    issues.push(LintIssue {
                        severity,
                        message: format!(
                            "Node ID '{}' contains illegal characters or exceeds 64 characters. IDs must only contain letters, numbers, hyphens, and underscores.",
                            id_str
                        ),
                        path: Some(format!("nodes[{}].id", i)),
                        node_id: Some(id_str.clone()),
                        code: Some("INVALID_NODE_ID_FORMAT".to_string()),
                    });
                }

                if !node_ids.insert(id_str.clone()) {
                    issues.push(LintIssue {
                        severity: LintSeverity::Error,
                        message: format!("Duplicate node ID detected: '{}'", id_str),
                        path: Some(format!("nodes[{}].id", i)),
                        node_id: Some(id_str.clone()),
                        code: Some("DUPLICATE_NODE_ID".to_string()),
                    });
                }
                id_str
            }
            _ => {
                issues.push(LintIssue {
                    severity: LintSeverity::Error,
                    message: "Node is missing required property 'id'".to_string(),
                    path: Some(format!("nodes[{}]", i)),
                    node_id: None,
                    code: Some("MISSING_NODE_ID".to_string()),
                });
                format!("unnamed_{}", i)
            }
        };

        // Node Component Type check
        if let Some(comp_type) = node.get("type").and_then(|t| t.as_str()) {
            let is_block_pascal = comp_type.starts_with("Block") && comp_type.len() > 5;
            let is_known_type = AUTOMA_KNOWN_NODE_TYPES.contains(&comp_type);
            let is_known_label = AUTOMA_KNOWN_BLOCK_LABELS.contains(&comp_type.to_lowercase().as_str());

            if !is_block_pascal && !is_known_type && !is_known_label {
                issues.push(LintIssue {
                    severity: LintSeverity::Warning,
                    message: format!(
                        "Node '{}' has unrecognized component type '{}'. Supported types: BlockBasic, BlockDelay, BlockConditions, BlockElementExists, BlockLoopData, etc.",
                        node_id, comp_type
                    ),
                    path: Some(format!("nodes[{}].type", i)),
                    node_id: Some(node_id.clone()),
                    code: Some("UNRECOGNIZED_COMPONENT_TYPE".to_string()),
                });
            }
        } else {
            let severity = match mode {
                LintMode::Editor => LintSeverity::Warning,
                LintMode::Runner => LintSeverity::Error,
            };
            issues.push(LintIssue {
                severity,
                message: "Node is missing property 'type'. Defaulting to BlockBasic.".to_string(),
                path: Some(format!("nodes[{}].type", i)),
                node_id: Some(node_id.clone()),
                code: Some("MISSING_NODE_TYPE".to_string()),
            });
        }

        // Node Label Whitelist check (with fallback to name, then type)
        let raw_label = node
            .get("label")
            .or_else(|| node.get("name"))
            .or_else(|| node.get("type"))
            .and_then(|l| l.as_str())
            .unwrap_or("");

        let label = if let Some(stripped) = raw_label.strip_prefix("Block") {
            if stripped.eq_ignore_ascii_case("basic") || stripped.eq_ignore_ascii_case("basicwithfallback") {
                node.get("label").or_else(|| node.get("name")).and_then(|l| l.as_str()).unwrap_or(raw_label)
            } else {
                raw_label
            }
        } else {
            raw_label
        };

        if label.is_empty() {
            issues.push(LintIssue {
                severity: LintSeverity::Error,
                message: format!("Node '{}' is missing required property 'label' or 'name'.", node_id),
                path: Some(format!("nodes[{}].label", i)),
                node_id: Some(node_id.clone()),
                code: Some("MISSING_BLOCK_LABEL".to_string()),
            });
        } else if label.eq_ignore_ascii_case("extract-data") {
            issues.push(LintIssue {
                severity: LintSeverity::Error,
                message: format!(
                    "Block 'extract-data' in node '{}' does not exist in Automa. Use 'get-text' or 'attribute-value' with saveData=true instead.",
                    node_id
                ),
                path: Some(format!("nodes[{}].label", i)),
                node_id: Some(node_id.clone()),
                code: Some("FORBIDDEN_EXTRACT_DATA".to_string()),
            });
        } else {
            let lower_label = label.to_lowercase();
            let is_known = AUTOMA_KNOWN_BLOCK_LABELS.contains(&lower_label.as_str())
                || (lower_label.starts_with("block") && AUTOMA_KNOWN_BLOCK_LABELS.iter().any(|b| lower_label.ends_with(b.replace('-', "").as_str())));

            if !is_known {
                issues.push(LintIssue {
                    severity: LintSeverity::Error,
                    message: format!(
                        "Unknown block label '{}' on node '{}'. Automa only supports official blocks (e.g. 'new-tab', 'forms', 'event-click', 'press-key', 'get-text', 'conditions', 'loop-data').",
                        label, node_id
                    ),
                    path: Some(format!("nodes[{}].label", i)),
                    node_id: Some(node_id.clone()),
                    code: Some("UNKNOWN_BLOCK_LABEL".to_string()),
                });
            }
        }

        // Position layout validation for VueFlow canvas grid
        if let Some(pos) = node.get("position") {
            let has_x = pos.get("x").and_then(|x| x.as_f64()).is_some();
            let has_y = pos.get("y").and_then(|y| y.as_f64()).is_some();
            if !has_x || !has_y {
                issues.push(LintIssue {
                    severity: LintSeverity::Warning,
                    message: format!(
                        "Node '{}' position must contain valid numeric 'x' and 'y' coordinates.",
                        node_id
                    ),
                    path: Some(format!("nodes[{}].position", i)),
                    node_id: Some(node_id.clone()),
                    code: Some("INVALID_NODE_POSITION".to_string()),
                });
            }
        }

        if label.eq_ignore_ascii_case("trigger") {
            trigger_count += 1;
        }

        // Deep block data inspection
        if let Some(data) = node.get("data") {
            validate_block_data(&node_id, label, i, data, mode, issues);
        }

        node_map.insert(node_id, (i, node));
    }

    // Graph Topology: Trigger node validation
    if trigger_count == 0 {
        let severity = match mode {
            LintMode::Editor => LintSeverity::Warning,
            LintMode::Runner => LintSeverity::Error,
        };
        issues.push(LintIssue {
            severity,
            message: "Workflow does not contain a 'trigger' block to start execution.".to_string(),
            path: Some("nodes".to_string()),
            node_id: None,
            code: Some("NO_TRIGGER_NODE".to_string()),
        });
    }

    // Graph Topology: Edge connections, handles, and broken references
    let mut nodes_with_incoming = HashSet::new();
    let mut nodes_with_outgoing = HashSet::new();
    let mut loop_back_edges: HashMap<String, HashSet<String>> = HashMap::new();

    for (e_idx, edge) in edges.iter().enumerate() {
        if let Some(edge_id) = edge.get("id").and_then(|id| id.as_str()) {
            if !is_valid_edge_id(edge_id) {
                issues.push(LintIssue {
                    severity: LintSeverity::Warning,
                    message: format!(
                        "Edge ID '{}' contains illegal characters. IDs must be DOM-safe.",
                        edge_id
                    ),
                    path: Some(format!("edges[{}].id", e_idx)),
                    node_id: None,
                    code: Some("INVALID_EDGE_ID_FORMAT".to_string()),
                });
            }
        }

        let source = edge.get("source").and_then(|s| s.as_str());
        let target = edge.get("target").and_then(|t| t.as_str());
        let source_handle = edge.get("sourceHandle").and_then(|h| h.as_str());
        let target_handle = edge.get("targetHandle").and_then(|h| h.as_str());

        let valid_source = match source {
            Some(src) if node_ids.contains(src) => {
                nodes_with_outgoing.insert(src.to_string());
                Some(src)
            }
            Some(src) => {
                issues.push(LintIssue {
                    severity: LintSeverity::Error,
                    message: format!("Edge connects from non-existent source node '{}'", src),
                    path: Some(format!("edges[{}].source", e_idx)),
                    node_id: None,
                    code: Some("BROKEN_EDGE_SOURCE".to_string()),
                });
                None
            }
            None => {
                issues.push(LintIssue {
                    severity: LintSeverity::Error,
                    message: "Edge is missing required property 'source'".to_string(),
                    path: Some(format!("edges[{}]", e_idx)),
                    node_id: None,
                    code: Some("MISSING_EDGE_SOURCE".to_string()),
                });
                None
            }
        };

        let valid_target = match target {
            Some(tgt) if node_ids.contains(tgt) => {
                if let Some((_, target_node)) = node_map.get(tgt) {
                    let tgt_label = target_node
                        .get("label")
                        .or_else(|| target_node.get("name"))
                        .and_then(|l| l.as_str())
                        .unwrap_or("");
                    if tgt_label.eq_ignore_ascii_case("trigger") {
                        issues.push(LintIssue {
                            severity: LintSeverity::Error,
                            message: format!(
                                "Trigger block '{}' has 0 inputs and cannot be the target of an incoming edge.",
                                tgt
                            ),
                            path: Some(format!("edges[{}].target", e_idx)),
                            node_id: Some(tgt.to_string()),
                            code: Some("TRIGGER_CANNOT_HAVE_INPUT".to_string()),
                        });
                    }
                }

                nodes_with_incoming.insert(tgt.to_string());
                if let Some(src) = source {
                    loop_back_edges
                        .entry(tgt.to_string())
                        .or_default()
                        .insert(src.to_string());
                }
                Some(tgt)
            }
            Some(tgt) => {
                issues.push(LintIssue {
                    severity: LintSeverity::Error,
                    message: format!("Edge connects to non-existent target node '{}'", tgt),
                    path: Some(format!("edges[{}].target", e_idx)),
                    node_id: None,
                    code: Some("BROKEN_EDGE_TARGET".to_string()),
                });
                None
            }
            None => {
                issues.push(LintIssue {
                    severity: LintSeverity::Error,
                    message: "Edge is missing required property 'target'".to_string(),
                    path: Some(format!("edges[{}]", e_idx)),
                    node_id: None,
                    code: Some("MISSING_EDGE_TARGET".to_string()),
                });
                None
            }
        };

        // Source Handle & Target Handle Consistency Validation
        if let (Some(src), Some(src_h)) = (valid_source, source_handle) {
            if let Some((_, src_node)) = node_map.get(src) {
                let src_label = src_node
                    .get("label")
                    .or_else(|| src_node.get("name"))
                    .and_then(|l| l.as_str())
                    .unwrap_or("")
                    .to_lowercase();

                if src_h.contains("-output-") {
                    let prefix = src_h.split("-output-").next().unwrap_or("");
                    if !prefix.is_empty() && prefix != src {
                        issues.push(LintIssue {
                            severity: LintSeverity::Error,
                            message: format!(
                                "Edge sourceHandle '{}' belongs to node '{}', but edge source is '{}'. This mismatch will cause silent execution failure.",
                                src_h, prefix, src
                            ),
                            path: Some(format!("edges[{}].sourceHandle", e_idx)),
                            node_id: Some(src.to_string()),
                            code: Some("HANDLE_NODE_MISMATCH".to_string()),
                        });
                    }
                }

                if (src_h.ends_with("-output-fallback") || src_h == "fallback")
                    && EXCLUDE_ON_ERROR.contains(&src_label.as_str())
                    && src_label != "conditions"
                    && src_label != "element-exists"
                    && src_label != "while-loop"
                {
                    issues.push(LintIssue {
                        severity: LintSeverity::Error,
                        message: format!(
                            "Block '{}' ({}) is in excludeOnError and cannot have fallback handle '{}'.",
                            src_label, src, src_h
                        ),
                        path: Some(format!("edges[{}].sourceHandle", e_idx)),
                        node_id: Some(src.to_string()),
                        code: Some("INVALID_FALLBACK_HANDLE".to_string()),
                    });
                }

                if src_label == "conditions" {
                    let is_fallback = src_h.ends_with("-output-fallback") || src_h == "fallback";
                    if !is_fallback {
                        let cond_id = if src_h.contains("-output-") {
                            src_h.split("-output-").nth(1).unwrap_or("")
                        } else {
                            src_h
                        };

                        let condition_ids: Vec<String> = src_node
                            .get("data")
                            .and_then(|d| d.get("conditions"))
                            .and_then(|c| c.as_array())
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|c| c.get("id").and_then(|id| id.as_str()))
                                    .map(|s| s.to_string())
                                    .collect()
                            })
                            .unwrap_or_default();

                        if !condition_ids.is_empty() && !condition_ids.contains(&cond_id.to_string()) {
                            issues.push(LintIssue {
                                severity: LintSeverity::Warning,
                                message: format!(
                                    "Condition block '{}' has handle '{}' which does not match any condition ID in its data table.",
                                    src, src_h
                                ),
                                path: Some(format!("edges[{}].sourceHandle", e_idx)),
                                node_id: Some(src.to_string()),
                                code: Some("INVALID_CONDITION_HANDLE".to_string()),
                            });
                        }
                    }
                }
            }
        }

        if let (Some(tgt), Some(tgt_h)) = (valid_target, target_handle) {
            if tgt_h.contains("-input-") {
                let prefix = tgt_h.split("-input-").next().unwrap_or("");
                if !prefix.is_empty() && prefix != tgt {
                    issues.push(LintIssue {
                        severity: LintSeverity::Error,
                        message: format!(
                            "Edge targetHandle '{}' belongs to node '{}', but edge target is '{}'.",
                            tgt_h, prefix, tgt
                        ),
                        path: Some(format!("edges[{}].targetHandle", e_idx)),
                        node_id: Some(tgt.to_string()),
                        code: Some("HANDLE_NODE_MISMATCH".to_string()),
                    });
                }
            }
        }
    }

    // Graph Topology: Orphan / Disconnected Nodes Check
    for (node_id, (i, node)) in &node_map {
        let label = node.get("label").and_then(|l| l.as_str()).unwrap_or("");
        let is_trigger = label.eq_ignore_ascii_case("trigger");

        let has_in = nodes_with_incoming.contains(node_id);
        let has_out = nodes_with_outgoing.contains(node_id);

        if !has_in && !has_out && !is_trigger && nodes.len() > 1 {
            issues.push(LintIssue {
                severity: LintSeverity::Warning,
                message: format!("Node '{}' is disconnected from the workflow graph.", node_id),
                path: Some(format!("nodes[{}]", i)),
                node_id: Some(node_id.clone()),
                code: Some("ORPHAN_NODE".to_string()),
            });
        }

        // Loop block: Check for loop returning edge
        if label.eq_ignore_ascii_case("loop-data") {
            let has_return_edge = loop_back_edges
                .get(node_id)
                .map(|sources| !sources.is_empty())
                .unwrap_or(false);

            if !has_return_edge && nodes.len() > 2 {
                issues.push(LintIssue {
                    severity: LintSeverity::Warning,
                    message: format!(
                        "Loop block '{}' is missing a returning loop edge to increment iteration index.",
                        node_id
                    ),
                    path: Some(format!("nodes[{}].data", i)),
                    node_id: Some(node_id.clone()),
                    code: Some("LOOP_MISSING_BACK_EDGE".to_string()),
                });
            }
        }
    }

    if edges.is_empty() && nodes.len() > 1 {
        issues.push(LintIssue {
            severity: LintSeverity::Warning,
            message: "Workflow has no edges defined between multiple nodes".to_string(),
            path: Some("edges".to_string()),
            node_id: None,
            code: Some("NO_EDGES_DEFINED".to_string()),
        });
    }
}

/// Validates individual block-level data schema
fn validate_block_data(
    node_id: &str,
    label: &str,
    index: usize,
    data: &serde_json::Value,
    mode: LintMode,
    issues: &mut Vec<LintIssue>,
) {
    let lower_label = label.to_lowercase();

    match lower_label.as_str() {
        "new-tab" | "open-url" | "link" => {
            let url = data.get("url").and_then(|u| u.as_str()).unwrap_or("");
            if url.trim().is_empty() {
                let severity = match mode {
                    LintMode::Editor => LintSeverity::Warning,
                    LintMode::Runner => LintSeverity::Error,
                };
                issues.push(LintIssue {
                    severity,
                    message: format!("Block '{}' ({}) requires a target URL.", label, node_id),
                    path: Some(format!("nodes[{}].data.url", index)),
                    node_id: Some(node_id.to_string()),
                    code: Some("EMPTY_URL".to_string()),
                });
            } else if !url.starts_with("http://")
                && !url.starts_with("https://")
                && !url.starts_with("{{")
                && !url.starts_with('$')
                && !url.starts_with("chrome://")
                && !url.starts_with("about:")
            {
                issues.push(LintIssue {
                    severity: LintSeverity::Warning,
                    message: format!(
                        "URL '{}' in block '{}' should start with http://, https:// or variable expression.",
                        url, node_id
                    ),
                    path: Some(format!("nodes[{}].data.url", index)),
                    node_id: Some(node_id.to_string()),
                    code: Some("SUSPICIOUS_URL".to_string()),
                });
            }
        }
        "click-element" | "event-click" | "get-text" | "forms" | "hover-element" | "element-scroll" => {
            let selector = data.get("selector").and_then(|s| s.as_str()).unwrap_or("");
            if selector.trim().is_empty() {
                let severity = match mode {
                    LintMode::Editor => LintSeverity::Warning,
                    LintMode::Runner => LintSeverity::Error,
                };
                issues.push(LintIssue {
                    severity,
                    message: format!(
                        "DOM interaction block '{}' ({}) has an empty element selector.",
                        label, node_id
                    ),
                    path: Some(format!("nodes[{}].data.selector", index)),
                    node_id: Some(node_id.to_string()),
                    code: Some("EMPTY_SELECTOR".to_string()),
                });
            }
        }
        "press-key" => {
            let action = data.get("action").and_then(|a| a.as_str()).unwrap_or("press-key");
            let keys = data.get("keys").and_then(|k| k.as_str()).unwrap_or("");
            let keys_to_press = data.get("keysToPress").and_then(|k| k.as_str()).unwrap_or("");

            if action == "press-key" && keys.trim().is_empty() {
                issues.push(LintIssue {
                    severity: LintSeverity::Warning,
                    message: format!("Press key block '{}' has an empty 'keys' field.", node_id),
                    path: Some(format!("nodes[{}].data.keys", index)),
                    node_id: Some(node_id.to_string()),
                    code: Some("EMPTY_PRESS_KEY".to_string()),
                });
            } else if action == "multiple-keys" && keys_to_press.trim().is_empty() {
                issues.push(LintIssue {
                    severity: LintSeverity::Warning,
                    message: format!("Press key block '{}' has an empty 'keysToPress' field.", node_id),
                    path: Some(format!("nodes[{}].data.keysToPress", index)),
                    node_id: Some(node_id.to_string()),
                    code: Some("EMPTY_PRESS_KEY".to_string()),
                });
            }
        }
        "loop-data" | "while-loop" | "loop-elements" => {
            let loop_id = data.get("loopId").and_then(|l| l.as_str()).unwrap_or("");
            if loop_id.trim().is_empty() {
                let severity = match mode {
                    LintMode::Editor => LintSeverity::Warning,
                    LintMode::Runner => LintSeverity::Error,
                };
                issues.push(LintIssue {
                    severity,
                    message: format!("Loop block '{}' ({}) has an empty 'loopId'.", label, node_id),
                    path: Some(format!("nodes[{}].data.loopId", index)),
                    node_id: Some(node_id.to_string()),
                    code: Some("EMPTY_LOOP_ID".to_string()),
                });
            }
        }
        "execute-workflow" => {
            let workflow_id = data.get("workflowId").and_then(|w| w.as_str()).unwrap_or("");
            let execute_id = data.get("executeId").and_then(|e| e.as_str()).unwrap_or("");
            if workflow_id.trim().is_empty() && execute_id.trim().is_empty() {
                issues.push(LintIssue {
                    severity: LintSeverity::Warning,
                    message: format!("Execute workflow block '{}' has no target workflowId or executeId.", node_id),
                    path: Some(format!("nodes[{}].data.workflowId", index)),
                    node_id: Some(node_id.to_string()),
                    code: Some("EMPTY_EXECUTE_WORKFLOW_ID".to_string()),
                });
            }
        }
        "proxy" => {
            let host = data.get("host").and_then(|h| h.as_str()).unwrap_or("");
            let clear_proxy = data.get("clearProxy").and_then(|c| c.as_bool()).unwrap_or(false);
            if !clear_proxy && host.trim().is_empty() {
                issues.push(LintIssue {
                    severity: LintSeverity::Warning,
                    message: format!("Proxy block '{}' has an empty 'host' address.", node_id),
                    path: Some(format!("nodes[{}].data.host", index)),
                    node_id: Some(node_id.to_string()),
                    code: Some("EMPTY_PROXY_HOST".to_string()),
                });
            }
        }
        "google-sheets" => {
            let sheet_id = data.get("spreadsheetId").and_then(|s| s.as_str()).unwrap_or("");
            let range = data.get("range").and_then(|r| r.as_str()).unwrap_or("");
            if sheet_id.trim().is_empty() || range.trim().is_empty() {
                issues.push(LintIssue {
                    severity: LintSeverity::Warning,
                    message: format!("Google Sheets block '{}' requires both 'spreadsheetId' and 'range'.", node_id),
                    path: Some(format!("nodes[{}].data", index)),
                    node_id: Some(node_id.to_string()),
                    code: Some("EMPTY_SHEET_PARAMS".to_string()),
                });
            }
        }
        "conditions" => {
            if let Some(conditions) = data.get("conditions").and_then(|c| c.as_array()) {
                for (c_idx, cond) in conditions.iter().enumerate() {
                    if let Some(op_type) = cond.get("type").and_then(|t| t.as_str()) {
                        if !VALID_COMPARE_TYPES.contains(&op_type) {
                            issues.push(LintIssue {
                                severity: LintSeverity::Error,
                                message: format!(
                                    "Condition #{} in block '{}' uses invalid comparison operator '{}'. Valid operators: eq, eqi, nq, gt, gte, lt, lte, cnt, cni, nct, nci, stw, enw, rgx, itr, ifl.",
                                    c_idx + 1, node_id, op_type
                                ),
                                path: Some(format!("nodes[{}].data.conditions[{}].type", index, c_idx)),
                                node_id: Some(node_id.to_string()),
                                code: Some("INVALID_CONDITION_OPERATOR".to_string()),
                            });
                        }
                    }
                }
            }
        }
        "blocks-group" => {
            if let Some(nested_blocks) = data.get("blocks").and_then(|b| b.as_array()) {
                for (b_idx, nested) in nested_blocks.iter().enumerate() {
                    let nested_label = nested
                        .get("label")
                        .or_else(|| nested.get("name"))
                        .and_then(|l| l.as_str())
                        .unwrap_or("")
                        .to_lowercase();
                    if EXCLUDE_GROUP_BLOCKS.contains(&nested_label.as_str()) {
                        issues.push(LintIssue {
                            severity: LintSeverity::Error,
                            message: format!(
                                "Block '{}' inside blocks-group '{}' is in excludeGroupBlocks and cannot be grouped.",
                                nested_label, node_id
                            ),
                            path: Some(format!("nodes[{}].data.blocks[{}]", index, b_idx)),
                            node_id: Some(node_id.to_string()),
                            code: Some("ILLEGAL_NESTED_GROUP_BLOCK".to_string()),
                        });
                    }
                }
            }
        }
        "javascript-code" => {
            let code = data.get("code").and_then(|c| c.as_str()).unwrap_or("");
            if code.contains("{{variables.")
                || code.contains("{{secrets.")
                || code.contains("{{loopData.")
                || code.contains("{{table.")
            {
                issues.push(LintIssue {
                    severity: LintSeverity::Error,
                    message: format!(
                        "JavaScript block '{}' contains Mustache syntax '{{...}}'. Use 'automaRefData(\"variables\", \"name\")' instead.",
                        node_id
                    ),
                    path: Some(format!("nodes[{}].data.code", index)),
                    node_id: Some(node_id.to_string()),
                    code: Some("JS_MUSTACHE_FORBIDDEN".to_string()),
                });
            }
        }
        "switch-frame" | "switch-to" => {
            let switch_type = data.get("type").or_else(|| data.get("windowType")).and_then(|t| t.as_str()).unwrap_or("");
            let selector = data.get("selector").and_then(|s| s.as_str()).unwrap_or("");
            if (switch_type == "switch-to-iframe" || switch_type == "iframe" || switch_type.is_empty()) && selector.trim().is_empty() {
                issues.push(LintIssue {
                    severity: LintSeverity::Warning,
                    message: format!(
                        "Iframe switch block '{}' requires an iframe element selector.",
                        node_id
                    ),
                    path: Some(format!("nodes[{}].data.selector", index)),
                    node_id: Some(node_id.to_string()),
                    code: Some("EMPTY_IFRAME_SELECTOR".to_string()),
                });
            }
        }
        _ => {}
    }
}
