use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::infrastructure::db::AutomaDb;

use super::sources::{
    find_in_database, find_in_vaults, read_and_validate_file, resolve_by_path, resolve_inline_data,
};
use super::types::{ResolvedWorkflow, WorkflowResolveError};
use super::validator::{sanitize_workflow_data, validate_workflow_structure};

pub struct WorkflowResolver;

impl WorkflowResolver {
    pub async fn resolve(
        job_id: &str,
        data_dir: &str,
        workflow_id: Option<&str>,
        workflow_path: Option<&str>,
        workflow_data: Option<&serde_json::Value>,
        db: &Arc<Mutex<AutomaDb>>,
    ) -> Result<ResolvedWorkflow, WorkflowResolveError> {
        if let Some(data) = workflow_data {
            Self::resolve_inline_data(job_id, data_dir, data).await
        } else if let Some(wf_id) = workflow_id {
            Self::resolve_by_id(job_id, data_dir, wf_id, workflow_path, db).await
        } else if let Some(wp) = workflow_path {
            let trimmed = wp.trim();
            // Step 1: Check if input is inline JSON string
            if trimmed.starts_with('{')
                && let Ok(json_val) = serde_json::from_str::<serde_json::Value>(trimmed)
                    && json_val.is_object() && (json_val.get("nodes").is_some() || json_val.get("drawflow").is_some()) {
                        return Self::resolve_inline_data(job_id, data_dir, &json_val).await;
                    }

            // Step 2: Check if wp is an existing file path directly
            let path_obj = Path::new(wp);
            if path_obj.exists() && path_obj.is_file() {
                Self::resolve_by_path(wp).await
            } else {
                // Step 3 & 4: Fallback to ID/Vault/DB resolution
                Self::resolve_by_id(job_id, data_dir, wp, Some(wp), db).await
            }
        } else {
            Err(WorkflowResolveError::BadRequest(
                "Either workflowId, workflowData, or workflowPath must be provided".to_string(),
            ))
        }
    }

    pub async fn resolve_inline_data(
        job_id: &str,
        data_dir: &str,
        data: &serde_json::Value,
    ) -> Result<ResolvedWorkflow, WorkflowResolveError> {
        resolve_inline_data(job_id, data_dir, data).await
    }

    pub async fn resolve_by_id(
        job_id: &str,
        data_dir: &str,
        wf_id: &str,
        fallback_path: Option<&str>,
        db: &Arc<Mutex<AutomaDb>>,
    ) -> Result<ResolvedWorkflow, WorkflowResolveError> {
        if wf_id.contains("..") {
            return Err(WorkflowResolveError::BadRequest(
                "Invalid workflowId: path traversal sequences are not allowed".to_string(),
            ));
        }

        // Step 1 & 2: Database resolution
        let db_workflow = find_in_database(wf_id, db).await;

        if let Some(wf) = db_workflow {
            let mut json_data: serde_json::Value = serde_json::from_str(&wf.data)
                .map_err(|e| WorkflowResolveError::Internal(format!("Failed to parse database workflow JSON: {}", e)))?;

            if json_data.get("name").is_none()
                && let Some(obj) = json_data.as_object_mut() {
                    obj.insert("name".to_string(), serde_json::Value::String(wf.name.clone()));
                }

            Self::validate_workflow_structure(&json_data)?;

            let sanitized_data = sanitize_workflow_data(&json_data);

            let temp_dir = PathBuf::from(data_dir).join("temp_workflows");
            let _ = tokio::fs::create_dir_all(&temp_dir).await;
            let file_path = temp_dir.join(format!("{}.workflow.json", job_id));

            let content = serde_json::to_string_pretty(&sanitized_data)
                .unwrap_or(wf.data);

            tokio::fs::write(&file_path, content)
                .await
                .map_err(|e| WorkflowResolveError::Internal(format!("Failed to prepare workflow from database: {}", e)))?;

            return Ok(ResolvedWorkflow {
                path: file_path,
                data: sanitized_data,
            });
        }

        // Step 3 & 4: Vault resolution
        if let Some(resolved) = find_in_vaults(wf_id, data_dir).await {
            return Ok(resolved);
        }

        // Step 5: Check fallback path if provided
        if let Some(wp) = fallback_path
            && let Ok(canon) = tokio::fs::canonicalize(wp).await
                && canon.is_file() {
                    return Self::read_and_validate_file(&canon).await;
                }

        Err(WorkflowResolveError::NotFound(format!(
            "Workflow '{}' not found in database or vault (~/.specter/automa/workflows/). Use 'specter automa workflow list' to view available workflows.",
            wf_id
        )))
    }

    pub async fn resolve_by_path(workflow_path: &str) -> Result<ResolvedWorkflow, WorkflowResolveError> {
        resolve_by_path(workflow_path).await
    }

    pub async fn read_and_validate_file(path: &Path) -> Result<ResolvedWorkflow, WorkflowResolveError> {
        read_and_validate_file(path).await
    }

    pub fn validate_workflow_structure(val: &serde_json::Value) -> Result<(), WorkflowResolveError> {
        validate_workflow_structure(val)
    }
}
