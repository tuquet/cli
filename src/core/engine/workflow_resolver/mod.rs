pub mod resolver;
pub mod sources;
pub mod types;
pub mod validator;

pub use self::resolver::*;
pub use self::sources::*;
pub use self::types::*;
pub use self::validator::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::db::AutomationDb;
    use std::sync::Arc;
    use tokio::sync::Mutex;

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

        let res =
            WorkflowResolver::resolve_inline_data("job_123", &temp_dir.to_string_lossy(), &data)
                .await;

        assert!(res.is_ok());
        let resolved = res.unwrap();
        assert_eq!(resolved.data["name"], "Inline Flow");
        assert!(resolved.path.exists());

        let _ = tokio::fs::remove_dir_all(&temp_dir).await;
    }

    #[tokio::test]
    async fn debug_inline_validation() {
        let db = Arc::new(Mutex::new(AutomationDb::new_in_memory().unwrap()));
        let inline_json = r#"{"name":"Inline Str Flow","nodes":[]}"#;
        let res = WorkflowResolver::resolve("j", "/tmp", None, Some(inline_json), None, &db).await;
        match res {
            Ok(r) => println!(
                "DEBUG OK keys={:?}",
                r.data
                    .as_object()
                    .map(|o| o.keys().cloned().collect::<Vec<_>>())
            ),
            Err(e) => println!("DEBUG ERR: {}", e),
        }
    }

    #[tokio::test]
    async fn test_resolve_missing_all_params_fails() {
        let db = Arc::new(Mutex::new(AutomationDb::new_in_memory().unwrap()));
        let res = WorkflowResolver::resolve("job_empty", "/tmp", None, None, None, &db).await;

        assert!(res.is_err());
        match res.unwrap_err() {
            WorkflowResolveError::BadRequest(msg) => {
                assert!(msg.contains("Either workflowId, workflowData, or workflowPath"));
            }
            _ => panic!("Expected BadRequest error"),
        }
    }

    #[tokio::test]
    async fn test_resolve_inline_json_string_via_path() {
        let db = Arc::new(Mutex::new(AutomationDb::new_in_memory().unwrap()));
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
        )
        .await;

        assert!(res.is_ok());
        let resolved = res.unwrap();
        assert_eq!(resolved.data["name"], "Inline Str Flow");
        assert!(resolved.path.exists());

        let _ = tokio::fs::remove_dir_all(&temp_dir).await;
    }

    #[tokio::test]
    async fn test_resolve_path_traversal_in_id_fails() {
        let db = Arc::new(Mutex::new(AutomationDb::new_in_memory().unwrap()));
        let res = WorkflowResolver::resolve(
            "job_traversal",
            "/tmp",
            None,
            Some("../../etc/passwd"),
            None,
            &db,
        )
        .await;

        assert!(res.is_err());
        match res.unwrap_err() {
            WorkflowResolveError::BadRequest(msg) => {
                assert!(msg.contains("path traversal"));
            }
            other => panic!("Expected BadRequest for path traversal, got {:?}", other),
        }
    }
}
