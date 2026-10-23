use crate::infrastructure::db::AutomationDb;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;

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

pub async fn find_in_database(
    wf_id: &str,
    db: &Arc<Mutex<AutomationDb>>,
) -> Option<crate::infrastructure::db::workflows::DbWorkflow> {
    // Step 1: Active / ephemeral DB
    let active_result = {
        let guard = db.lock().await;
        guard
            .workflows()
            .get_workflow_by_id_or_name(wf_id)
            .ok()
            .flatten()
    };

    if active_result.is_some() {
        return active_result;
    }

    // Step 2: Check persistent SQLite database
    let persistent_config = crate::config::AppConfig::load();
    let persistent_db_path = PathBuf::from(&persistent_config.data_dir).join(crate::constants::FILE_AUTOMATION_SQLITE);
    if persistent_db_path.exists()
        && let Ok(pdb) = AutomationDb::new(&persistent_db_path)
    {
        return pdb
            .workflows()
            .get_workflow_by_id_or_name(wf_id)
            .ok()
            .flatten();
    }

    None
}
