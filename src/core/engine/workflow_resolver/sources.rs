use std::path::{Path, PathBuf};

use super::types::{ResolvedWorkflow, WorkflowResolveError};
use super::validator::{sanitize_workflow_data, validate_workflow_structure};

pub async fn resolve_inline_data(
    job_id: &str,
    data_dir: &str,
    data: &serde_json::Value,
) -> Result<ResolvedWorkflow, WorkflowResolveError> {
    if !data.is_object() {
        return Err(WorkflowResolveError::BadRequest(
            "workflowData must be a valid JSON object".to_string(),
        ));
    }

    validate_workflow_structure(data)?;

    let sanitized_data = sanitize_workflow_data(data);

    let temp_dir = PathBuf::from(data_dir).join("temp_workflows");
    let _ = tokio::fs::create_dir_all(&temp_dir).await;
    let file_path = temp_dir.join(format!("{}.workflow.json", job_id));
    let content = serde_json::to_string_pretty(&sanitized_data).map_err(|e| {
        WorkflowResolveError::Internal(format!("Failed to serialize workflow: {}", e))
    })?;

    tokio::fs::write(&file_path, content).await.map_err(|e| {
        WorkflowResolveError::Internal(format!("Failed to save temporary workflow: {}", e))
    })?;

    Ok(ResolvedWorkflow {
        path: file_path,
        data: sanitized_data,
    })
}

pub async fn resolve_by_path(
    workflow_path: &str,
) -> Result<ResolvedWorkflow, WorkflowResolveError> {
    if workflow_path.trim().is_empty() {
        return Err(WorkflowResolveError::BadRequest(
            "workflowPath cannot be empty".to_string(),
        ));
    }

    let canon = tokio::fs::canonicalize(workflow_path).await.map_err(|_| {
        WorkflowResolveError::BadRequest("Invalid workflow path (does not exist)".to_string())
    })?;

    #[allow(unused_mut)]
    let mut final_path = canon;
    #[cfg(target_os = "windows")]
    {
        let s = final_path.to_string_lossy();
        if let Some(stripped) = s.strip_prefix(r"\\?\") {
            final_path = std::path::PathBuf::from(stripped);
        }
    }

    if !final_path.is_file() {
        return Err(WorkflowResolveError::BadRequest(
            "Invalid workflow path (not a file)".to_string(),
        ));
    }

    read_and_validate_file(&final_path).await
}

pub async fn read_and_validate_file(path: &Path) -> Result<ResolvedWorkflow, WorkflowResolveError> {
    let content = tokio::fs::read_to_string(path).await.map_err(|e| {
        WorkflowResolveError::BadRequest(format!("Failed to read workflow file: {}", e))
    })?;

    let json_val: serde_json::Value = serde_json::from_str(&content).map_err(|e| {
        WorkflowResolveError::BadRequest(format!("Failed to parse workflow file as JSON: {}", e))
    })?;

    validate_workflow_structure(&json_val)?;

    Ok(ResolvedWorkflow {
        path: path.to_path_buf(),
        data: json_val,
    })
}

/// Resolve a workflow document (graph + metadata) by UUID or name from the Cloud
/// table `automation.workflows` — the single source of truth (the local SQLite
/// workflow store was removed).
pub async fn find_in_cloud(wf_id: &str) -> Option<serde_json::Value> {
    let repo = crate::infrastructure::cloud::CloudWorkflowRepository::connect()
        .await
        .ok()?;
    let wf = repo.find(wf_id).await.ok().flatten()?;

    let mut doc = serde_json::json!({
        "id": wf.id,
        "name": wf.name,
        "description": wf.description,
        "version": wf.version,
        "settings": wf.settings,
    });
    if let (Some(obj), Some(graph)) = (doc.as_object_mut(), wf.graph_data.as_object()) {
        for (k, v) in graph {
            obj.insert(k.clone(), v.clone());
        }
    }
    Some(doc)
}
