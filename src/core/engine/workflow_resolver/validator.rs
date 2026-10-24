use std::sync::OnceLock;

use super::types::WorkflowResolveError;

/// The workflow document schema (Automa-derived) is the **SSOT for workflow structure**.
/// It used to be decorative — no code loaded it. It is now compiled once and enforced on
/// every document that enters the engine.
static WORKFLOW_SCHEMA: OnceLock<Result<jsonschema::Validator, String>> = OnceLock::new();

fn workflow_schema() -> Result<&'static jsonschema::Validator, WorkflowResolveError> {
    WORKFLOW_SCHEMA
        .get_or_init(|| {
            let raw = include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/schema/workflow.schema.json"
            ));
            let schema: serde_json::Value = serde_json::from_str(raw)
                .map_err(|e| format!("workflow.schema.json is not valid JSON: {}", e))?;
            jsonschema::validator_for(&schema)
                .map_err(|e| format!("workflow.schema.json failed to compile: {}", e))
        })
        .as_ref()
        .map_err(|e| WorkflowResolveError::BadRequest(e.clone()))
}

/// Enforce the workflow contract before anything executes:
///   1. the document must match `workflow.schema.json` (the declared structure), and
///   2. every block label it uses must actually be implemented by this engine.
///
/// Doctrine (*Automa schema = SSOT*, *declared ⇒ satisfied*): a canvas may legitimately
/// author blocks we do not implement yet — silently skipping them would make the canvas
/// fake authoring and hide real gaps, so the document is rejected instead.
pub fn validate_workflow_document(val: &serde_json::Value) -> Result<(), WorkflowResolveError> {
    let validator = workflow_schema()?;

    let mut errors: Vec<String> = validator
        .iter_errors(val)
        .take(8)
        .map(|e| format!("{} (at {})", e, e.instance_path))
        .collect();
    if !errors.is_empty() {
        errors.sort();
        return Err(WorkflowResolveError::BadRequest(format!(
            "workflow does not match workflow.schema.json: {}",
            errors.join(" | ")
        )));
    }

    let supported = worker::drivers::cdp::supported_block_labels();
    let canvas_only = worker::drivers::cdp::CANVAS_ONLY_BLOCKS;

    // A document may carry its graph as a flat `nodes` array (Tuquet form) or inside
    // `drawflow` (what the Automa canvas actually exports). Both must be checked —
    // otherwise the coverage check silently passes for every real canvas export.
    let mut unsupported: Vec<String> = val
        .get("nodes")
        .and_then(|n| n.as_array())
        .into_iter()
        .flatten()
        .chain(
            val.get("drawflow")
                .and_then(|d| d.get("nodes"))
                .and_then(|n| n.as_array())
                .into_iter()
                .flatten(),
        )
        .filter_map(|node| node.get("label").and_then(|l| l.as_str()))
        .filter(|label| !supported.contains(label) && !canvas_only.contains(label))
        .map(str::to_string)
        .collect();
    if !unsupported.is_empty() {
        unsupported.sort();
        unsupported.dedup();
        return Err(WorkflowResolveError::BadRequest(format!(
            "workflow uses block(s) this engine does not implement: {} — refusing to run (declared => satisfied)",
            unsupported.join(", ")
        )));
    }

    Ok(())
}

pub fn validate_workflow_structure(val: &serde_json::Value) -> Result<(), WorkflowResolveError> {
    if let Some(nodes) = val.get("nodes")
        && !nodes.is_array()
    {
        return Err(WorkflowResolveError::BadRequest(
            "workflowData.nodes must be an array".to_string(),
        ));
    }

    if let Some(drawflow) = val.get("drawflow") {
        if let Some(s) = drawflow.as_str() {
            if serde_json::from_str::<serde_json::Value>(s).is_err() {
                return Err(WorkflowResolveError::BadRequest(
                    "Workflow 'drawflow' string is not valid JSON".to_string(),
                ));
            }
        } else if !drawflow.is_object() {
            return Err(WorkflowResolveError::BadRequest(
                "Workflow 'drawflow' must be an object or a JSON string".to_string(),
            ));
        }
    }

    if val.get("nodes").is_none() && val.get("drawflow").is_none() {
        return Err(WorkflowResolveError::BadRequest(
            "Workflow must contain either 'drawflow' or 'nodes'".to_string(),
        ));
    }

    Ok(())
}

pub fn sanitize_workflow_data(data: &serde_json::Value) -> serde_json::Value {
    let mut sanitized_data = data.clone();
    if let Ok(mut wf) =
        serde_json::from_value::<crate::core::models::workflow::Workflow>(data.clone())
    {
        wf = crate::core::engine::sanitizer::sanitize_workflow(wf);
        if let Ok(mut val) = serde_json::to_value(wf) {
            if let Some(df_obj) = val.get_mut("drawflow").and_then(|d| d.as_object_mut())
                && let Some(edges) = df_obj.get_mut("edges").and_then(|e| e.as_array_mut())
            {
                for edge in edges.iter_mut() {
                    if let Some(edge_map) = edge.as_object_mut() {
                        let src = edge_map
                            .get("source")
                            .and_then(|s| s.as_str())
                            .unwrap_or("")
                            .to_string();
                        let tgt = edge_map
                            .get("target")
                            .and_then(|t| t.as_str())
                            .unwrap_or("")
                            .to_string();
                        if !edge_map.contains_key("sourceHandle") && !src.is_empty() {
                            edge_map.insert(
                                "sourceHandle".to_string(),
                                serde_json::json!(format!("{}-output-1", src)),
                            );
                        }
                        if !edge_map.contains_key("targetHandle") && !tgt.is_empty() {
                            edge_map.insert(
                                "targetHandle".to_string(),
                                serde_json::json!(format!("{}-input-1", tgt)),
                            );
                        }
                    }
                }
            }
            sanitized_data = val;
        }
    }
    strip_nulls(sanitized_data)
}

/// The workflow document format has no notion of `null`: an absent field and a null field mean
/// the same thing, while the schema (`workflow.schema.json`) rejects `null` for typed fields.
/// Serializing our own Rust model emits `null` for every `None`, so normalise on the way out —
/// otherwise our own pipeline would produce documents that violate our own contract.
fn strip_nulls(value: serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => serde_json::Value::Object(
            map.into_iter()
                .filter(|(_, v)| !v.is_null())
                .map(|(k, v)| (k, strip_nulls(v)))
                .collect(),
        ),
        serde_json::Value::Array(items) => {
            serde_json::Value::Array(items.into_iter().map(strip_nulls).collect())
        }
        other => other,
    }
}

#[cfg(test)]
mod contract_tests {
    use super::*;
    use serde_json::json;

    fn supported_graph() -> serde_json::Value {
        json!({
            "name": "contract-test",
            "nodes": [
                {"id": "t", "label": "trigger", "data": {"type": "cron-job",
                    "parameters": [{"name": "mode", "defaultValue": "dry-run"}]}},
                {"id": "a", "label": "new-tab", "data": {"url": "https://example.com"}},
                {"id": "b", "label": "javascript-code", "data": {"code": "return 1"}}
            ],
            "edges": []
        })
    }

    #[test]
    fn accepts_document_with_supported_blocks() {
        assert!(validate_workflow_document(&supported_graph()).is_ok());
    }

    #[test]
    fn rejects_block_the_engine_does_not_implement() {
        let mut g = supported_graph();
        g["nodes"][1]["label"] = json!("google-sheets");
        let err = validate_workflow_document(&g).unwrap_err().to_string();
        assert!(err.contains("does not implement"), "unexpected error: {err}");
        assert!(err.contains("google-sheets"), "unexpected error: {err}");
    }

    #[test]
    fn accepts_canvas_only_block() {
        let mut g = supported_graph();
        g["nodes"][1]["label"] = json!("blocks-group");
        assert!(validate_workflow_document(&g).is_ok());
    }

    #[test]
    fn rejects_structure_violating_the_schema() {
        let g = json!({"nodes": "not-an-array"});
        let err = validate_workflow_document(&g).unwrap_err().to_string();
        assert!(err.contains("workflow.schema.json"), "unexpected error: {err}");
    }

    #[test]
    fn sanitize_strips_null_fields_so_our_own_output_conforms() {
        let raw = json!({"name": "x", "nodes": [], "edges": null, "settings": null});
        let clean = sanitize_workflow_data(&raw);
        assert!(clean.get("edges").is_none(), "null fields must be dropped: {clean}");
        assert!(validate_workflow_document(&clean).is_ok());
    }

    #[test]
    fn rejects_document_without_nodes_or_drawflow() {
        let g = json!({"name": "empty"});
        assert!(validate_workflow_document(&g).is_err());
    }
}

/// Real documents exported by the Automa canvas (bundled starter workflows, captured
/// verbatim in `cli/fixtures/`). These are the ground truth for "does our contract still
/// match what the canvas actually produces" — the drift this test caught: Automa writes
/// `data.delay` as a **string** from its form inputs, our schema demanded a number.
#[cfg(test)]
mod automa_export_tests {
    use super::*;

    const EXPORTS: &[(&str, &str)] = &[
        ("twitter-to-google-sheets", include_str!("../../../../fixtures/automa-export-twitter-to-google-sheets.json")),
        ("google-search", include_str!("../../../../fixtures/automa-export-google-search.json")),
        ("lorem-ipsum", include_str!("../../../../fixtures/automa-export-lorem-ipsum.json")),
        ("producthunt-search", include_str!("../../../../fixtures/automa-export-producthunt-search.json")),
        ("google-keyword-research", include_str!("../../../../fixtures/automa-export-google-keyword-research.json")),
    ];

    #[test]
    fn every_real_automa_export_matches_the_schema() {
        for (name, raw) in EXPORTS {
            let doc: serde_json::Value = serde_json::from_str(raw).unwrap();
            if let Err(err) = validate_workflow_document(&doc) {
                let msg = err.to_string();
                // Block coverage is engine capability, not document validity — the schema
                // (structure) part must always pass.
                assert!(
                    msg.contains("does not implement"),
                    "Automa export '{name}' failed schema validation: {msg}"
                );
            }
        }
    }

    #[test]
    fn documents_the_one_remaining_unimplemented_automa_block() {
        // `export-data`, `execute-workflow` and the `note` annotation block are implemented
        // now, so every starter export except the Google Sheets one runs as-is.
        let sheets: serde_json::Value = serde_json::from_str(EXPORTS[0].1).unwrap();
        let err = validate_workflow_document(&sheets).unwrap_err().to_string();
        assert!(
            err.contains("google-sheets"),
            "expected google-sheets to remain the only rejection, got: {err}"
        );
        assert!(
            !err.contains("export-data"),
            "export-data is implemented and must not be rejected: {err}"
        );
    }

    #[test]
    fn every_automa_export_except_google_sheets_is_runnable() {
        for (name, raw) in EXPORTS {
            let doc: serde_json::Value = serde_json::from_str(raw).unwrap();
            let result = validate_workflow_document(&doc);
            match *name {
                "twitter-to-google-sheets" => assert!(result.is_err(), "{name} should still need google-sheets"),
                _ => assert!(
                    result.is_ok(),
                    "real Automa export '{name}' must be runnable end-to-end, got: {:?}",
                    result.err().map(|e| e.to_string())
                ),
            }
        }
    }
}
