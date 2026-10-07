use axum::{
    extract::{Json, Query, State},
};
use crate::AppState;
use crate::core::error::AutomaError;
use crate::core::models::storage::{StorageTable, StorageVariable};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// Request payload to restore a full Automa backup JSON into SQLite
pub struct RestoreBackupRequest {
    /// Full Automa backup JSON payload
    #[schema(value_type = Object)]
    pub backup: serde_json::Value,
    /// Optional password to decrypt protected workflows
    pub password: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// Summary response returned after successfully restoring an Automa backup
pub struct RestoreBackupResponse {
    /// True if restore succeeded
    pub success: bool,
    /// Human-readable summary message
    pub message: String,
    /// Number of workflows imported
    pub workflows_count: usize,
    /// Number of storage variables imported
    pub variables_count: usize,
    /// Number of storage tables imported
    pub tables_count: usize,
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct ExportBackupQuery {
    /// Optional password to encrypt workflows using AES-256-CBC + HMAC
    pub password: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// Full Automa backup export payload
pub struct ExportBackupResponse {
    /// True if workflows are encrypted with AES-256-CBC
    pub is_protected: bool,
    /// Array of workflow objects or encrypted HMAC-SHA256 string
    #[schema(value_type = Value)]
    pub workflows: serde_json::Value,
    /// Stored variables
    pub storage_variables: Vec<StorageVariable>,
    /// Stored tables with nested items
    pub storage_tables: Vec<StorageTable>,
    /// Export timestamp in milliseconds
    pub exported_at: String,
}

#[utoipa::path(
    tag = "Storage",
    post,
    path = "/api/v1/storage/backup/restore",
    operation_id = "restore_storage_backup",
    summary = "Restore full Automa backup into SQLite",
    description = "Restores workflows, storage variables, and storage tables from an Automa backup JSON. Supports protected backups encrypted with AES-256-CBC and HMAC-SHA256.",
    request_body = RestoreBackupRequest,
    responses(
        (status = 200, description = "Backup restored successfully", body = RestoreBackupResponse),
        (status = 400, description = "Invalid backup format or decryption error", body = crate::core::error::ApiErrorResponse),
        (status = 500, description = "Database write error", body = crate::core::error::ApiErrorResponse)
    )
)]
pub async fn restore_storage_backup(
    State(state): State<AppState>,
    Json(payload): Json<RestoreBackupRequest>,
) -> Result<Json<RestoreBackupResponse>, AutomaError> {
    let backup_val = &payload.backup;
    let mut total_workflows = 0;
    let mut total_variables = 0;
    let mut total_tables = 0;

    let db = state.db.lock().await;

    let insert_workflow = |db: &crate::infrastructure::db::AutomaDb, wf: &serde_json::Value| -> Result<bool, AutomaError> {
        if !wf.is_object() {
            return Ok(false);
        }
        let id = wf.get("id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("wf_{}", uuid::Uuid::new_v4().simple()));

        let name = wf.get("name")
            .and_then(|v| v.as_str())
            .unwrap_or(&id)
            .to_string();

        let description = wf.get("description")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let version = wf.get("version")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let icon = wf.get("icon")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let data_str = serde_json::to_string(wf)
            .map_err(|e| AutomaError::BadRequest(format!("Failed to serialize workflow: {}", e)))?;

        db.workflows().create_workflow(
            &id,
            &name,
            description.as_deref(),
            &data_str,
            version.as_deref(),
            icon.as_deref(),
        ).map_err(|e| AutomaError::DatabaseError(e.to_string()))?;

        if let Some(included) = wf.get("includedWorkflows").and_then(|v| v.as_object()) {
            for (sub_id, sub_wf) in included {
                if sub_wf.is_object() {
                    let sub_name = sub_wf.get("name").and_then(|v| v.as_str()).unwrap_or(sub_id);
                    let sub_desc = sub_wf.get("description").and_then(|v| v.as_str());
                    let sub_ver = sub_wf.get("version").and_then(|v| v.as_str());
                    let sub_icon = sub_wf.get("icon").and_then(|v| v.as_str());
                    if let Ok(sub_data_str) = serde_json::to_string(sub_wf) {
                        let _ = db.workflows().create_workflow(
                            sub_id,
                            sub_name,
                            sub_desc,
                            &sub_data_str,
                            sub_ver,
                            sub_icon,
                        );
                    }
                }
            }
        }
        Ok(true)
    };

    if let Some(arr) = backup_val.as_array() {
        for item in arr {
            if insert_workflow(&db, item)? {
                total_workflows += 1;
            }
        }
    } else if let Some(map) = backup_val.as_object() {
        if map.contains_key("workflows") || map.contains_key("storageVariables") || map.contains_key("storageTables") {
            if let Some(wf_field) = map.get("workflows") {
                let is_protected = map.get("isProtected")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                let parsed_wf_list: Vec<serde_json::Value> = if is_protected {
                    let pass = payload.password.as_deref().unwrap_or("");
                    if pass.is_empty() {
                        return Err(AutomaError::BadRequest("Password is required to decrypt protected backup".into()));
                    }
                    let enc_str = wf_field.as_str().ok_or_else(|| {
                        AutomaError::BadRequest("Protected workflows must be an encrypted string".into())
                    })?;
                    let decrypted_str = crate::core::crypto::decrypt_secret(enc_str, pass)
                        .map_err(|e| AutomaError::BadRequest(format!("Failed to decrypt workflows: {}", e)))?;
                    serde_json::from_str(&decrypted_str)
                        .map_err(|e| AutomaError::BadRequest(format!("Invalid decrypted workflow JSON: {}", e)))?
                } else if let Some(s) = wf_field.as_str() {
                    serde_json::from_str(s).unwrap_or_default()
                } else if let Some(arr) = wf_field.as_array() {
                    arr.clone()
                } else {
                    Vec::new()
                };

                for wf in &parsed_wf_list {
                    if insert_workflow(&db, wf)? {
                        total_workflows += 1;
                    }
                }
            }

            if let Some(var_field) = map.get("storageVariables") {
                let parsed_var_list: Vec<serde_json::Value> = if let Some(s) = var_field.as_str() {
                    serde_json::from_str(s).unwrap_or_default()
                } else if let Some(arr) = var_field.as_array() {
                    arr.clone()
                } else {
                    Vec::new()
                };

                for v in parsed_var_list {
                    if let Some(v_obj) = v.as_object() {
                        let name = v_obj.get("name")
                            .or_else(|| v_obj.get("key"))
                            .and_then(|k| k.as_str())
                            .unwrap_or_default();
                        if !name.is_empty() {
                            let val = v_obj.get("value").cloned().unwrap_or(serde_json::Value::Null);
                            let sv = StorageVariable {
                                id: None,
                                name: Some(name.to_string()),
                                key: Some(name.to_string()),
                                value: Some(val),
                            };
                            if db.storage().save_variable(&sv).is_ok() {
                                total_variables += 1;
                            }
                        }
                    }
                }
            }

            if let Some(tbl_field) = map.get("storageTables") {
                let parsed_tbl_list: Vec<serde_json::Value> = if let Some(s) = tbl_field.as_str() {
                    serde_json::from_str(s).unwrap_or_default()
                } else if let Some(arr) = tbl_field.as_array() {
                    arr.clone()
                } else {
                    Vec::new()
                };

                for t in parsed_tbl_list {
                    if let Some(t_obj) = t.as_object() {
                        let name = t_obj.get("name").and_then(|n| n.as_str()).unwrap_or_default();
                        if !name.is_empty() {
                            let id = t_obj.get("id")
                                .and_then(|i| i.as_str())
                                .map(|s| s.to_string())
                                .unwrap_or_else(|| format!("tbl_{}", uuid::Uuid::new_v4().simple()));
                            let columns = t_obj.get("columns").cloned();
                            let cols_idx = t_obj.get("columnsIndex").cloned();
                            let now = std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_millis() as i64;

                            let st = StorageTable {
                                id: Some(id.clone()),
                                name: Some(name.to_string()),
                                columns,
                                items: None,
                                columns_index: cols_idx,
                                created_at: Some(now),
                                modified_at: Some(now),
                            };

                            if db.tables().save_table(&st).is_ok() {
                                total_tables += 1;
                                let rows_arr = t_obj.get("rows")
                                    .or_else(|| t_obj.get("items"))
                                    .and_then(|r| r.as_array());

                                if let Some(rows) = rows_arr {
                                    for row in rows {
                                        let row_id = format!("row_{}", uuid::Uuid::new_v4().simple());
                                        let row_str = serde_json::to_string(row).unwrap_or_default();
                                        let _ = db.tables().add_table_row(&row_id, &id, &row_str);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        } else if insert_workflow(&db, backup_val)? {
            total_workflows += 1;
        }
    } else {
        return Err(AutomaError::BadRequest("Backup payload must be a JSON object or array".into()));
    }

    let _ = state.tx.send(serde_json::json!({
        "type": "storage_changed",
        "action": "restore_backup",
        "workflowsCount": total_workflows,
        "variablesCount": total_variables,
        "tablesCount": total_tables,
    }).to_string());

    Ok(Json(RestoreBackupResponse {
        success: true,
        message: format!("Successfully restored {} workflow(s), {} variable(s), {} table(s)", total_workflows, total_variables, total_tables),
        workflows_count: total_workflows,
        variables_count: total_variables,
        tables_count: total_tables,
    }))
}

#[utoipa::path(
    tag = "Storage",
    get,
    path = "/api/v1/storage/backup/export",
    operation_id = "export_storage_backup",
    summary = "Export full Automa backup JSON",
    description = "Exports all workflows, storage variables, and storage tables from SQLite into a standardized Automa backup JSON format.",
    params(
        ExportBackupQuery
    ),
    responses(
        (status = 200, description = "Full backup exported successfully", body = ExportBackupResponse),
        (status = 500, description = "Database read error", body = crate::core::error::ApiErrorResponse)
    )
)]
pub async fn export_storage_backup(
    State(state): State<AppState>,
    Query(query): Query<ExportBackupQuery>,
) -> Result<Json<ExportBackupResponse>, AutomaError> {
    let db = state.db.lock().await;

    let raw_workflows = db.workflows().get_workflows(None, None, None)
        .map_err(|e| AutomaError::DatabaseError(e.to_string()))?;

    let mut workflows_array = Vec::new();
    for w in raw_workflows {
        let mut wf_obj: serde_json::Value = serde_json::from_str(&w.data).unwrap_or(serde_json::json!({
            "id": w.id,
            "name": w.name,
            "description": w.description,
            "version": w.version,
        }));
        if let Some(map) = wf_obj.as_object_mut() {
            map.insert("id".to_string(), serde_json::Value::String(w.id));
            map.insert("name".to_string(), serde_json::Value::String(w.name));
            if let Some(desc) = w.description {
                map.insert("description".to_string(), serde_json::Value::String(desc));
            }
            map.insert("version".to_string(), serde_json::Value::String(w.version));
        }
        workflows_array.push(wf_obj);
    }

    let (is_protected, workflows_val) = if let Some(pass) = query.password.as_deref().filter(|p| !p.trim().is_empty()) {
        let serialized = serde_json::to_string(&workflows_array)
            .map_err(|e| AutomaError::Internal(format!("Failed to serialize workflows: {}", e)))?;
        let encrypted = crate::core::crypto::encrypt_secret(&serialized, pass)
            .map_err(|e| AutomaError::Internal(format!("Failed to encrypt workflows: {}", e)))?;
        (true, serde_json::Value::String(encrypted))
    } else {
        (false, serde_json::Value::Array(workflows_array))
    };

    let variables = db.storage().get_variables(None, None, None)
        .map_err(|e| AutomaError::DatabaseError(e.to_string()))?;

    let mut tables = db.tables().get_tables(None, None, None)
        .map_err(|e| AutomaError::DatabaseError(e.to_string()))?;

    for t in &mut tables {
        if let Some(ref tid) = t.id {
            let rows = db.tables().get_table_rows(tid, None, None, None).unwrap_or_default();
            let row_items: Vec<serde_json::Value> = rows.into_iter().map(|r| r.data).collect();
            t.items = Some(serde_json::Value::Array(row_items));
        }
    }

    let exported_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .to_string();

    Ok(Json(ExportBackupResponse {
        is_protected,
        workflows: workflows_val,
        storage_variables: variables,
        storage_tables: tables,
        exported_at,
    }))
}
