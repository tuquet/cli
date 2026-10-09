pub mod types;
pub mod validator;
pub mod sources;
pub mod resolver;

pub use self::types::*;
pub use self::validator::*;
pub use self::sources::*;
pub use self::resolver::*;

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use crate::infrastructure::db::AutomaDb;

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

    #[tokio::test]
    async fn test_resolve_path_traversal_in_id_fails() {
        let db = Arc::new(Mutex::new(AutomaDb::new_in_memory().unwrap()));
        let res = WorkflowResolver::resolve(
            "job_traversal",
            "/tmp",
            None,
            Some("../../etc/passwd"),
            None,
            &db,
        ).await;

        assert!(res.is_err());
        match res.unwrap_err() {
            WorkflowResolveError::BadRequest(msg) => {
                assert!(msg.contains("path traversal"));
            }
            other => panic!("Expected BadRequest for path traversal, got {:?}", other),
        }
    }
}
