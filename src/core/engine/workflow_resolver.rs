use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::infrastructure::db::AutomaDb;

#[derive(Debug, Clone)]
pub struct ResolvedWorkflow {
    pub path: PathBuf,
    pub data: serde_json::Value,
}

#[derive(Debug)]
pub enum WorkflowResolveError {
    BadRequest(String),
    NotFound(String),
    Internal(String),
}

impl std::fmt::Display for WorkflowResolveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadRequest(msg) => write!(f, "Bad Request: {}", msg),
            Self::NotFound(msg) => write!(f, "Not Found: {}", msg),
            Self::Internal(msg) => write!(f, "Internal Error: {}", msg),
        }
    }
}

impl std::error::Error for WorkflowResolveError {}

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
            Self::resolve_by_path(wp).await
        } else {
            Err(WorkflowResolveError::BadRequest(
                "Either workflowId, workflowData, or workflowPath must be provided".to_string(),
            ))
        }
    }

    async fn resolve_inline_data(
        job_id: &str,
        data_dir: &str,
        data: &serde_json::Value,
    ) -> Result<ResolvedWorkflow, WorkflowResolveError> {
        if !data.is_object() {
            return Err(WorkflowResolveError::BadRequest(
                "workflowData must be a valid JSON object".to_string(),
            ));
        }

        Self::validate_workflow_structure(data)?;

        let temp_dir = PathBuf::from(data_dir).join("temp_workflows");
        let _ = tokio::fs::create_dir_all(&temp_dir).await;
        let file_path = temp_dir.join(format!("{}.workflow.json", job_id));
        let content = serde_json::to_string_pretty(data)
            .map_err(|e| WorkflowResolveError::Internal(format!("Failed to serialize workflow: {}", e)))?;

        tokio::fs::write(&file_path, content)
            .await
            .map_err(|e| WorkflowResolveError::Internal(format!("Failed to save temporary workflow: {}", e)))?;

        Ok(ResolvedWorkflow {
            path: file_path,
            data: data.clone(),
        })
    }

    async fn resolve_by_id(
        job_id: &str,
        data_dir: &str,
        wf_id: &str,
        fallback_path: Option<&str>,
        db: &Arc<Mutex<AutomaDb>>,
    ) -> Result<ResolvedWorkflow, WorkflowResolveError> {
        let db_workflow = {
            let guard = db.lock().await;
            guard.workflows().get_workflow(wf_id).ok().flatten()
        };

        if let Some(wf) = db_workflow {
            let json_data: serde_json::Value = serde_json::from_str(&wf.data)
                .map_err(|e| WorkflowResolveError::Internal(format!("Failed to parse database workflow JSON: {}", e)))?;

            Self::validate_workflow_structure(&json_data)?;

            let temp_dir = PathBuf::from(data_dir).join("temp_workflows");
            let _ = tokio::fs::create_dir_all(&temp_dir).await;
            let file_path = temp_dir.join(format!("{}.workflow.json", job_id));

            tokio::fs::write(&file_path, &wf.data)
                .await
                .map_err(|e| WorkflowResolveError::Internal(format!("Failed to prepare workflow from database: {}", e)))?;

            return Ok(ResolvedWorkflow {
                path: file_path,
                data: json_data,
            });
        }

        let vault_root = crate::api::handlers::vault::get_vault_root();
        let candidates = [
            vault_root.join(format!("workflows/{}.workflow.json", wf_id)),
            vault_root.join(format!("{}.workflow.json", wf_id)),
            vault_root.join(format!("workflows/{}.json", wf_id)),
            vault_root.join(format!("{}.json", wf_id)),
            PathBuf::from(wf_id),
        ];

        for c in &candidates {
            if c.exists() && c.is_file() {
                return Self::read_and_validate_file(c).await;
            }
        }

        if let Some(wp) = fallback_path {
            if let Ok(canon) = tokio::fs::canonicalize(wp).await {
                if canon.is_file() {
                    return Self::read_and_validate_file(&canon).await;
                }
            }
            return Err(WorkflowResolveError::BadRequest(format!(
                "Workflow ID '{}' not found in database or file system",
                wf_id
            )));
        }

        Err(WorkflowResolveError::NotFound(format!(
            "Workflow ID '{}' not found in database",
            wf_id
        )))
    }

    async fn resolve_by_path(workflow_path: &str) -> Result<ResolvedWorkflow, WorkflowResolveError> {
        if workflow_path.trim().is_empty() {
            return Err(WorkflowResolveError::BadRequest(
                "workflowPath cannot be empty".to_string(),
            ));
        }

        let canon = tokio::fs::canonicalize(workflow_path)
            .await
            .map_err(|_| WorkflowResolveError::BadRequest("Invalid workflow path (does not exist)".to_string()))?;

        if !canon.is_file() {
            return Err(WorkflowResolveError::BadRequest(
                "Invalid workflow path (not a file)".to_string(),
            ));
        }

        Self::read_and_validate_file(&canon).await
    }

    async fn read_and_validate_file(path: &Path) -> Result<ResolvedWorkflow, WorkflowResolveError> {
        let content = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| WorkflowResolveError::BadRequest(format!("Failed to read workflow file: {}", e)))?;

        let json_val: serde_json::Value = serde_json::from_str(&content)
            .map_err(|e| WorkflowResolveError::BadRequest(format!("Failed to parse workflow file as JSON: {}", e)))?;

        Self::validate_workflow_structure(&json_val)?;

        Ok(ResolvedWorkflow {
            path: path.to_path_buf(),
            data: json_val,
        })
    }

    fn validate_workflow_structure(val: &serde_json::Value) -> Result<(), WorkflowResolveError> {
        if let Some(nodes) = val.get("nodes") {
            if !nodes.is_array() {
                return Err(WorkflowResolveError::BadRequest(
                    "workflowData.nodes must be an array".to_string(),
                ));
            }
        }

        if let Some(drawflow) = val.get("drawflow") {
            if !drawflow.is_object() {
                return Err(WorkflowResolveError::BadRequest(
                    "Workflow 'drawflow' must be an object".to_string(),
                ));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_workflow_structure_valid() {
        let valid = serde_json::json!({
            "name": "Test Workflow",
            "nodes": [],
            "drawflow": { "nodes": [] }
        });
        assert!(WorkflowResolver::validate_workflow_structure(&valid).is_ok());
    }

    #[test]
    fn test_validate_workflow_structure_invalid_nodes() {
        let invalid = serde_json::json!({
            "name": "Invalid Workflow",
            "nodes": "not-an-array"
        });
        assert!(WorkflowResolver::validate_workflow_structure(&invalid).is_err());
    }

    #[test]
    fn test_validate_workflow_structure_invalid_drawflow() {
        let invalid = serde_json::json!({
            "name": "Invalid Workflow",
            "nodes": [],
            "drawflow": "not-an-object"
        });
        assert!(WorkflowResolver::validate_workflow_structure(&invalid).is_err());
    }

    #[tokio::test]
    async fn test_resolve_inline_data_creates_temp_file() {
        let temp_dir = std::env::temp_dir().join(uuid::Uuid::new_v4().to_string());
        let _ = tokio::fs::create_dir_all(&temp_dir).await;

        let data = serde_json::json!({
            "name": "Inline Flow",
            "nodes": []
        });

        let res = WorkflowResolver::resolve_inline_data(
            "job_123",
            &temp_dir.to_string_lossy(),
            &data,
        ).await;

        assert!(res.is_ok());
        let resolved = res.unwrap();
        assert_eq!(resolved.data["name"], "Inline Flow");
        assert!(resolved.path.exists());

        let _ = tokio::fs::remove_dir_all(&temp_dir).await;
    }

    #[tokio::test]
    async fn test_resolve_missing_all_params_fails() {
        let db = Arc::new(Mutex::new(AutomaDb::new_in_memory().unwrap()));
        let res = WorkflowResolver::resolve(
            "job_empty",
            "/tmp",
            None,
            None,
            None,
            &db,
        ).await;

        assert!(res.is_err());
        match res.unwrap_err() {
            WorkflowResolveError::BadRequest(msg) => {
                assert!(msg.contains("Either workflowId, workflowData, or workflowPath"));
            }
            _ => panic!("Expected BadRequest error"),
        }
    }
}

