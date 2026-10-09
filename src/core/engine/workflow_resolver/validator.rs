use super::types::WorkflowResolveError;

pub fn validate_workflow_structure(val: &serde_json::Value) -> Result<(), WorkflowResolveError> {
    if let Some(nodes) = val.get("nodes")
        && !nodes.is_array() {
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
    if let Ok(mut wf) = serde_json::from_value::<crate::core::models::workflow::Workflow>(data.clone()) {
        wf = crate::core::engine::sanitizer::sanitize_workflow(wf);
        if let Ok(mut val) = serde_json::to_value(wf) {
            if let Some(df_obj) = val.get_mut("drawflow").and_then(|d| d.as_object_mut())
                && let Some(edges) = df_obj.get_mut("edges").and_then(|e| e.as_array_mut()) {
                    for edge in edges.iter_mut() {
                        if let Some(edge_map) = edge.as_object_mut() {
                            let src = edge_map.get("source").and_then(|s| s.as_str()).unwrap_or("").to_string();
                            let tgt = edge_map.get("target").and_then(|t| t.as_str()).unwrap_or("").to_string();
                            if !edge_map.contains_key("sourceHandle") && !src.is_empty() {
                                edge_map.insert("sourceHandle".to_string(), serde_json::json!(format!("{}-output-1", src)));
                            }
                            if !edge_map.contains_key("targetHandle") && !tgt.is_empty() {
                                edge_map.insert("targetHandle".to_string(), serde_json::json!(format!("{}-input-1", tgt)));
                            }
                        }
                    }
                }
            sanitized_data = val;
        }
    }
    sanitized_data
}
