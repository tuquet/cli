use axum::{
    extract::{Path, Json, Query, State},
    http::StatusCode,
};
use crate::AppState;
use crate::core::error::AutomaError;
use crate::core::models::storage::{StorageTable, TableRow};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

#[derive(Deserialize, IntoParams)]
pub struct GetTablesQuery {
    /// Maximum number of tables to return
    #[param(example = 50)]
    pub limit: Option<usize>,
    /// Number of items to skip for pagination (default 0)
    #[param(example = 0)]
    pub offset: Option<usize>,
    /// Optional search query to filter tables by name or ID
    pub search: Option<String>,
}

#[utoipa::path(
    tag = "Storage",
    get,
    path = "/api/v1/storage/tables",
    operation_id = "get_storage_tables",
    summary = "List all storage tables",
    description = "Retrieves schemas for all user data tables stored in SQLite.",
    params(GetTablesQuery),
    responses(
        (status = 200, description = "List of data tables", body = Vec<StorageTable>),
        (status = 500, description = "Database read error", body = crate::core::error::ApiErrorResponse)
    )
)]
pub async fn get_tables(
    State(state): State<AppState>,
    Query(query): Query<GetTablesQuery>,
) -> Result<Json<Vec<StorageTable>>, AutomaError> {
    let db = state.db.lock().await;
    let tables = db.tables().get_tables(query.limit, query.offset, query.search.as_deref())
        .map_err(|e| AutomaError::DatabaseError(e.to_string()))?;
    Ok(Json(tables))
}

#[utoipa::path(
    tag = "Storage",
    post,
    path = "/api/v1/storage/tables",
    operation_id = "add_storage_table",
    summary = "Create or update a storage table schema",
    description = "Defines a new data table schema or modifies column definitions for an existing table.",
    request_body = StorageTable,
    responses(
        (status = 200, description = "Table schema created or updated", body = StorageTable),
        (status = 400, description = "Invalid table schema", body = crate::core::error::ApiErrorResponse),
        (status = 500, description = "Database write error", body = crate::core::error::ApiErrorResponse)
    )
)]
pub async fn add_table(
    State(state): State<AppState>,
    Json(mut payload): Json<StorageTable>,
) -> Result<Json<StorageTable>, AutomaError> {
    if payload.id.is_none() {
        payload.id = Some(format!("table_{}", crate::infrastructure::fs_storage::FsStorageManager::generate_id()));
    }
    let db = state.db.lock().await;
    db.tables().save_table(&payload)
        .map_err(|e| AutomaError::DatabaseError(e.to_string()))?;
    Ok(Json(payload))
}

#[utoipa::path(
    tag = "Storage",
    delete,
    path = "/api/v1/storage/tables/{id}",
    operation_id = "delete_storage_table",
    summary = "Delete a storage table",
    description = "Drops the specified data table and purges all of its associated rows from SQLite.",
    params(
        ("id" = String, Path, description = "Unique table identifier")
    ),
    responses(
        (status = 200, description = "Table and rows deleted successfully"),
        (status = 404, description = "Table not found", body = crate::core::error::ApiErrorResponse),
        (status = 500, description = "Database deletion error", body = crate::core::error::ApiErrorResponse)
    )
)]
pub async fn delete_table(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, AutomaError> {
    let db = state.db.lock().await;
    db.tables().delete_table(&id)
        .map_err(|e| AutomaError::DatabaseError(e.to_string()))?;
    Ok(StatusCode::OK)
}

#[derive(Deserialize, IntoParams)]
pub struct GetTableRowsQuery {
    /// Maximum number of rows to return (default 50)
    #[param(example = 50)]
    pub limit: Option<usize>,
    /// Number of items to skip for pagination (default 0)
    #[param(example = 0)]
    pub offset: Option<usize>,
    /// Optional search query to filter rows by data content
    pub search: Option<String>,
}

pub type PaginationQuery = GetTableRowsQuery;

#[utoipa::path(
    tag = "Storage",
    get,
    path = "/api/v1/storage/tables/{id}/rows",
    operation_id = "get_storage_table_rows",
    summary = "Get rows from a storage table",
    description = "Queries paginated row documents belonging to the specified data table.",
    params(
        ("id" = String, Path, description = "Unique table identifier"),
        GetTableRowsQuery
    ),
    responses(
        (status = 200, description = "Array of table row objects", body = Vec<TableRow>),
        (status = 404, description = "Table not found", body = crate::core::error::ApiErrorResponse),
        (status = 500, description = "Database query error", body = crate::core::error::ApiErrorResponse)
    )
)]
pub async fn get_table_rows(
    State(state): State<AppState>,
    Path(table_id): Path<String>,
    Query(query): Query<GetTableRowsQuery>,
) -> Result<Json<Vec<TableRow>>, AutomaError> {
    let limit = query.limit;
    let offset = query.offset;
    let search = query.search.as_deref();

    let db = state.db.lock().await;
    let rows = db.tables().get_table_rows(&table_id, limit, offset, search)
        .map_err(|e| AutomaError::DatabaseError(e.to_string()))?;
    Ok(Json(rows))
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
#[schema(example = json!({"id": "row_123", "table_id": "tbl_456"}))]
/// Response returned after appending a row to a storage table
pub struct AddTableRowResponse {
    /// Newly assigned row identifier
    pub id: String,
    /// Parent table identifier
    pub table_id: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
#[schema(example = json!({"name": "John Doe", "age": 30}))]
/// Key-value payload for inserting a new table row
pub struct AddTableRowPayload {
    /// Dynamic column values for the row
    #[serde(flatten)]
    #[schema(value_type = Object)]
    pub data: std::collections::HashMap<String, serde_json::Value>,
}

#[utoipa::path(
    tag = "Storage",
    post,
    path = "/api/v1/storage/tables/{id}/rows",
    operation_id = "add_storage_table_row",
    summary = "Insert a row into a storage table",
    description = "Appends a new document row into the specified data table.",
    params(
        ("id" = String, Path, description = "Unique table identifier")
    ),
    request_body = AddTableRowPayload,
    responses(
        (status = 200, description = "Row inserted successfully", body = AddTableRowResponse),
        (status = 400, description = "Malformed row data", body = crate::core::error::ApiErrorResponse),
        (status = 404, description = "Table not found", body = crate::core::error::ApiErrorResponse),
        (status = 500, description = "Database insertion error", body = crate::core::error::ApiErrorResponse)
    )
)]
pub async fn add_table_row(
    State(state): State<AppState>,
    Path(table_id): Path<String>,
    Json(payload): Json<AddTableRowPayload>,
) -> Result<Json<AddTableRowResponse>, AutomaError> {
    let row_id = crate::infrastructure::fs_storage::FsStorageManager::generate_id();
    let data_str = serde_json::to_string(&payload.data)
        .map_err(|e| AutomaError::JsonError(e))?;
    let db = state.db.lock().await;
    db.tables().add_table_row(&row_id, &table_id, &data_str)
        .map_err(|e| AutomaError::DatabaseError(e.to_string()))?;

    Ok(Json(AddTableRowResponse { id: row_id, table_id }))
}
