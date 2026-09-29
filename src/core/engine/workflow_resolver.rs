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

        let temp_dir = PathBuf::from(data_dir).join("temp_workflows");
        let _ = tokio::fs::create_dir_all(&temp_dir).await;
        let file_path = temp_dir.join(format!("{}.workflow.json", job_id));
        let content = serde_json::to_string_pretty(&sanitized_data)
            .map_err(|e| WorkflowResolveError::Internal(format!("Failed to serialize workflow: {}", e)))?;

        tokio::fs::write(&file_path, content)
            .await
            .map_err(|e| WorkflowResolveError::Internal(format!("Failed to save temporary workflow: {}", e)))?;

        Ok(ResolvedWorkflow {
            path: file_path,
            data: sanitized_data,
        })
    }

    async fn resolve_by_id(
        job_id: &str,
        data_dir: &str,
        wf_id: &str,
        fallback_path: Option<&str>,
        db: &Arc<Mutex<AutomaDb>>,
    ) -> Result<ResolvedWorkflow, WorkflowResolveError> {
        // Step 1: Check active / ephemeral DB
        let mut db_workflow = {
            let guard = db.lock().await;
            guard.workflows().get_workflow_by_id_or_name(wf_id).ok().flatten()
        };

        // Step 2: Check persistent SQLite database if not found in active DB
        if db_workflow.is_none() {
            let persistent_config = crate::config::AppConfig::load();
            let persistent_db_path = PathBuf::from(&persistent_config.data_dir).join("automa.sqlite");
            if persistent_db_path.exists()
                && let Ok(pdb) = AutomaDb::new(&persistent_db_path) {
                    db_workflow = pdb.workflows().get_workflow_by_id_or_name(wf_id).ok().flatten();
                }
        }

        if let Some(wf) = db_workflow {
            let mut json_data: serde_json::Value = serde_json::from_str(&wf.data)
                .map_err(|e| WorkflowResolveError::Internal(format!("Failed to parse database workflow JSON: {}", e)))?;

            if json_data.get("name").is_none()
                && let Some(obj) = json_data.as_object_mut() {
                    obj.insert("name".to_string(), serde_json::Value::String(wf.name.clone()));
                }

            Self::validate_workflow_structure(&json_data)?;

            let mut sanitized_data = json_data.clone();
            if let Ok(mut wf_model) = serde_json::from_value::<crate::core::models::workflow::Workflow>(json_data.clone()) {
                wf_model = crate::core::engine::sanitizer::sanitize_workflow(wf_model);
                if let Ok(mut val) = serde_json::to_value(wf_model) {
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

        // Step 3: Check Vault candidate directories
        let persistent_config = crate::config::AppConfig::load();
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .unwrap_or_else(|_| ".".to_string());
        let tuquet_vault = PathBuf::from(&home).join(".tuquet").join("workflows");
        let config_vault = PathBuf::from(&persistent_config.data_dir).join("workflows");
        let local_vault = PathBuf::from(data_dir).join("workflows");

        let candidates = [
            tuquet_vault.join(format!("{}.workflow.json", wf_id)),
            tuquet_vault.join(format!("{}.json", wf_id)),
            tuquet_vault.join(wf_id),
            config_vault.join(format!("{}.workflow.json", wf_id)),
            config_vault.join(format!("{}.json", wf_id)),
            config_vault.join(wf_id),
            local_vault.join(format!("{}.workflow.json", wf_id)),
            local_vault.join(format!("{}.json", wf_id)),
            PathBuf::from(data_dir).join(format!("{}.workflow.json", wf_id)),
            PathBuf::from(data_dir).join(format!("{}.json", wf_id)),
            PathBuf::from(wf_id),
        ];

        for c in &candidates {
            if c.exists() && c.is_file() {
                return Self::read_and_validate_file(c).await;
            }
        }

        // Step 4: Case-insensitive / slug search in vaults
        let target_lower = wf_id.to_lowercase();
        let vault_dirs = [tuquet_vault, config_vault, local_vault];
        for vdir in &vault_dirs {
            if let Ok(mut entries) = tokio::fs::read_dir(vdir).await {
                while let Ok(Some(entry)) = entries.next_entry().await {
                    let path = entry.path();
                    if path.is_file()
                        && let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                            let stem_clean = stem.trim_end_matches(".workflow");
                            if stem_clean.eq_ignore_ascii_case(&target_lower) || stem.eq_ignore_ascii_case(&target_lower) {
                                return Self::read_and_validate_file(&path).await;
                            }
                        }
                }
            }
        }

        // Step 5: Check fallback path if provided
        if let Some(wp) = fallback_path
            && let Ok(canon) = tokio::fs::canonicalize(wp).await
                && canon.is_file() {
                    return Self::read_and_validate_file(&canon).await;
                }

        Err(WorkflowResolveError::NotFound(format!(
            "Workflow '{}' not found in database or vault (~/.tuquet/workflows/). Use 'tuquet automa workflow list' to view available workflows.",
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

        Self::read_and_validate_file(&final_path).await
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

    #[tokio::test]
    async fn test_resolve_by_db_id_and_name() {
        let db = Arc::new(Mutex::new(AutomaDb::new_in_memory().unwrap()));
        {
            let guard = db.lock().await;
            guard.workflows().create_workflow(
                "flow_db_test",
                "My Database Flow",
                Some("Test description"),
                r#"{"nodes":[{"id":"node_1","type":"trigger"}]}"#,
                Some("1.0.0"),
                None,
            ).unwrap();
        }

        let temp_dir = std::env::temp_dir().join(uuid::Uuid::new_v4().to_string());
        let _ = tokio::fs::create_dir_all(&temp_dir).await;

        // Resolve by exact ID via workflow_path (simulating `automa run flow_db_test`)
        let res_id = WorkflowResolver::resolve(
            "job_001",
            &temp_dir.to_string_lossy(),
            None,
            Some("flow_db_test"),
            None,
            &db,
        ).await;
        assert!(res_id.is_ok());
        let resolved = res_id.unwrap();
        assert_eq!(resolved.data["name"], "My Database Flow");
        assert!(resolved.path.exists());

        // Resolve by Name (case-insensitive) via workflow_path (simulating `automa run "my database flow"`)
        let res_name = WorkflowResolver::resolve(
            "job_002",
            &temp_dir.to_string_lossy(),
            None,
            Some("my database flow"),
            None,
            &db,
        ).await;
        assert!(res_name.is_ok());
        assert_eq!(res_name.unwrap().data["name"], "My Database Flow");

        let _ = tokio::fs::remove_dir_all(&temp_dir).await;
    }

    #[tokio::test]
    async fn test_resolve_inline_json_string_via_path() {
        let db = Arc::new(Mutex::new(AutomaDb::new_in_memory().unwrap()));
        let temp_dir = std::env::temp_dir().join(uuid::Uuid::new_v4().to_string());
        let _ = tokio::fs::create_dir_all(&temp_dir).await;

        let inline_json = r#"{"name":"Inline Str Flow","nodes":[]}"#;
        let res = WorkflowResolver::resolve(
            "job_inline",
            &temp_dir.to_string_lossy(),
            None,
            Some(inline_json),
            None,
            &db,
        ).await;

        assert!(res.is_ok());
        let resolved = res.unwrap();
        assert_eq!(resolved.data["name"], "Inline Str Flow");
        assert!(resolved.path.exists());

        let _ = tokio::fs::remove_dir_all(&temp_dir).await;
    }

    #[tokio::test]
    async fn test_resolve_vault_file_via_id() {
        let db = Arc::new(Mutex::new(AutomaDb::new_in_memory().unwrap()));
        let temp_dir = std::env::temp_dir().join(uuid::Uuid::new_v4().to_string());
        let vault_dir = temp_dir.join("workflows");
        let _ = tokio::fs::create_dir_all(&vault_dir).await;

        // Place a workflow file inside vault_dir
        let vault_file = vault_dir.join("daily_checkin.workflow.json");
        let file_content = r#"{"name":"Daily Checkin Flow","nodes":[]}"#;
        tokio::fs::write(&vault_file, file_content).await.unwrap();

        // Resolve by ID "daily_checkin"
        let res = WorkflowResolver::resolve(
            "job_vault",
            &temp_dir.to_string_lossy(),
            None,
            Some("daily_checkin"),
            None,
            &db,
        ).await;

        assert!(res.is_ok());
        let resolved = res.unwrap();
        assert_eq!(resolved.data["name"], "Daily Checkin Flow");

        let _ = tokio::fs::remove_dir_all(&temp_dir).await;
    }
}

