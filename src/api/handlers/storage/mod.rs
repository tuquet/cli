pub use crate::core::models::storage::{StorageCredential, StorageTable, StorageVariable, TableRow};

pub mod variables;
pub mod credentials;
pub mod tables;
pub mod workflows;
pub mod backup;

pub use variables::*;
pub use credentials::*;
pub use tables::*;
pub use workflows::*;
pub use backup::*;

#[cfg(test)]
mod tests {
    use super::*;
    use axum::extract::{Path, Json, Query, State};
    use axum::http::StatusCode;
    use crate::AppState;
    use crate::config::AppConfig;
    use crate::infrastructure::db::AutomaDb;
    use std::sync::Arc;
    use tokio::sync::{broadcast, Mutex, RwLock};

    fn create_test_state() -> AppState {
        let (tx, _) = broadcast::channel(100);
        let (worker_tx, _) = broadcast::channel(100);
        let in_memory_db = AutomaDb::new_in_memory().expect("Failed to create in-memory test db");
        let config = Arc::new(AppConfig::load());

        AppState {
            tx,
            worker_tx,
            config,
            db: Arc::new(Mutex::new(in_memory_db)),
            active_jobs: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }

    #[tokio::test]
    async fn test_storage_variables_api_flow() {
        let state = create_test_state();

        // 1. Add variable
        let var_payload = StorageVariable {
            id: Some("var_test_1".to_string()),
            name: Some("API_KEY".to_string()),
            key: Some("api_key".to_string()),
            value: Some(serde_json::json!("sk_live_123456")),
        };
        let added = add_variable(State(state.clone()), Json(var_payload)).await.unwrap();
        assert_eq!(added.name.as_deref(), Some("API_KEY"));

        // 2. Query variables
        let vars = get_variables(State(state.clone()), Query(GetVariablesQuery { limit: None, offset: None, search: None })).await.unwrap();
        assert_eq!(vars.len(), 1);
        assert_eq!(vars[0].name.as_deref(), Some("API_KEY"));

        // 3. Delete variable
        let del_status = delete_variable(State(state.clone()), Path("var_test_1".to_string())).await.unwrap();
        assert_eq!(del_status, StatusCode::OK);

        let vars_after = get_variables(State(state), Query(GetVariablesQuery { limit: None, offset: None, search: None })).await.unwrap();
        assert_eq!(vars_after.len(), 0);
    }

    #[tokio::test]
    async fn test_storage_credentials_api_flow() {
        let state = create_test_state();

        // 1. Add credential
        let cred_payload = StorageCredential {
            id: Some("cred_test_1".to_string()),
            name: Some("AWS Token".to_string()),
            key: Some("aws_token".to_string()),
            value: Some("super_secret_aws_token_value".to_string()),
        };
        let added = add_credential(State(state.clone()), Json(cred_payload)).await.unwrap();
        assert_eq!(added.name.as_deref(), Some("AWS Token"));

        // 2. Query credentials
        let creds = get_credentials(State(state.clone()), Query(GetCredentialsQuery { limit: None, offset: None, search: None })).await.unwrap();
        assert_eq!(creds.len(), 1);
        assert_eq!(creds[0].name.as_deref(), Some("AWS Token"));

        // 3. Delete credential
        let del_status = delete_credential(State(state.clone()), Path("cred_test_1".to_string())).await.unwrap();
        assert_eq!(del_status, StatusCode::OK);

        let creds_after = get_credentials(State(state), Query(GetCredentialsQuery { limit: None, offset: None, search: None })).await.unwrap();
        assert_eq!(creds_after.len(), 0);
    }

    #[tokio::test]
    async fn test_storage_tables_and_rows_api_flow() {
        let state = create_test_state();

        // 1. Add table
        let table_payload = StorageTable {
            id: Some("tbl_users".to_string()),
            name: Some("Users".to_string()),
            columns: Some(serde_json::json!([
                {"id": "col_1", "name": "email", "type": "string"}
            ])),
            items: None,
            columns_index: None,
            created_at: None,
            modified_at: None,
        };
        let created_tbl = add_table(State(state.clone()), Json(table_payload)).await.unwrap();
        assert_eq!(created_tbl.name.as_deref(), Some("Users"));

        // 2. Get tables
        let tables = get_tables(State(state.clone()), Query(GetTablesQuery { limit: None, offset: None, search: None })).await.unwrap();
        assert_eq!(tables.len(), 1);
        assert_eq!(tables[0].name.as_deref(), Some("Users"));

        // 3. Add table row
        let mut row_map = std::collections::HashMap::new();
        row_map.insert("email".to_string(), serde_json::json!("user@example.com"));
        let row_payload = AddTableRowPayload { data: row_map };
        let inserted_row = add_table_row(State(state.clone()), Path("tbl_users".to_string()), Json(row_payload)).await.unwrap();
        assert_eq!(inserted_row.table_id, "tbl_users");
        assert!(!inserted_row.id.is_empty());

        // 4. Get table rows with pagination
        let query = GetTableRowsQuery { limit: Some(10), offset: Some(0), search: None };
        let rows = get_table_rows(State(state.clone()), Path("tbl_users".to_string()), Query(query)).await.unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].table_id, "tbl_users");

        // 5. Delete table
        let del_status = delete_table(State(state.clone()), Path("tbl_users".to_string())).await.unwrap();
        assert_eq!(del_status, StatusCode::OK);

        let tables_after = get_tables(State(state), Query(GetTablesQuery { limit: None, offset: None, search: None })).await.unwrap();
        assert_eq!(tables_after.len(), 0);
    }

    #[tokio::test]
    async fn test_storage_workflows_api_flow() {
        let state = create_test_state();

        // 1. Create workflow
        let create_payload = CreateWorkflowStorageRequest {
            id: Some("wf_test_1".to_string()),
            name: "Test Search".to_string(),
            description: Some("Search tests".to_string()),
            data: serde_json::json!({
                "nodes": [{"id": "n1", "type": "trigger", "label": "trigger"}],
                "edges": []
            }),
            version: Some("1.0.0".to_string()),
            icon: Some("search".to_string()),
        };
        let created = create_storage_workflow(State(state.clone()), Json(create_payload)).await.unwrap();
        assert_eq!(created.id, "wf_test_1");
        assert_eq!(created.name, "Test Search");

        // 2. Get workflows
        let list = get_workflows(State(state.clone()), Query(GetWorkflowsQuery { limit: None, offset: None, search: None })).await.unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, "wf_test_1");

        // 3. Get workflow by ID
        let detail = get_workflow_by_id(State(state.clone()), Path("wf_test_1".to_string())).await.unwrap();
        assert_eq!(detail.name, "Test Search");

        // 4. Update workflow
        let update_payload = UpdateWorkflowStorageRequest {
            name: Some("Updated Test Search".to_string()),
            description: None,
            data: None,
            version: Some("1.0.1".to_string()),
            icon: None,
        };
        let updated = update_storage_workflow(State(state.clone()), Path("wf_test_1".to_string()), Json(update_payload)).await.unwrap();
        assert_eq!(updated.name, "Updated Test Search");
        assert_eq!(updated.version, "1.0.1");

        // 5. Import workflow
        let import_payload = ImportWorkflowStorageRequest {
            id: Some("wf_imported_1".to_string()),
            workflow: serde_json::json!({
                "name": "Imported Google Flow",
                "nodes": [{"id": "n2", "type": "click-element", "label": "click"}]
            }),
        };
        let imported = import_storage_workflow(State(state.clone()), Json(import_payload)).await.unwrap();
        assert_eq!(imported.id, "wf_imported_1");
        assert_eq!(imported.name, "Imported Google Flow");

        // 6. Delete workflow
        let del_res = delete_storage_workflow(State(state.clone()), Path("wf_test_1".to_string())).await.unwrap();
        assert!(del_res.0.success);

        let remaining = get_workflows(State(state), Query(GetWorkflowsQuery { limit: None, offset: None, search: None })).await.unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].id, "wf_imported_1");
    }
}
