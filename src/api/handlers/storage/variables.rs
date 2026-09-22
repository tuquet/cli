use axum::{
    extract::{Path, Json, Query, State},
    http::StatusCode,
};
use crate::AppState;
use crate::core::error::AutomaError;
use crate::core::models::storage::StorageVariable;
use serde::Deserialize;
use utoipa::IntoParams;

#[derive(Deserialize, IntoParams)]
pub struct GetVariablesQuery {
    /// Maximum number of variables to return
    #[param(example = 50)]
    pub limit: Option<usize>,
    /// Number of items to skip for pagination (default 0)
    #[param(example = 0)]
    pub offset: Option<usize>,
    /// Optional search query to filter variables by name, key or ID
    pub search: Option<String>,
}

#[utoipa::path(
    tag = "Storage",
    get,
    path = "/api/v1/storage/variables",
    operation_id = "get_storage_variables",
    summary = "List all storage variables",
    description = "Retrieves all global and local variables persisted in Automa SQLite storage.",
    params(GetVariablesQuery),
    responses(
        (status = 200, description = "List of variables", body = Vec<StorageVariable>),
        (status = 500, description = "Database read error", body = crate::core::error::ApiErrorResponse)
    )
)]
pub async fn get_variables(
    State(state): State<AppState>,
    Query(query): Query<GetVariablesQuery>,
) -> Result<Json<Vec<StorageVariable>>, AutomaError> {
    let db = state.db.lock().await;
    let vars = db.storage().get_variables(query.limit, query.offset, query.search.as_deref())?;
    Ok(Json(vars))
}

#[utoipa::path(
    tag = "Storage",
    post,
    path = "/api/v1/storage/variables",
    operation_id = "add_storage_variable",
    summary = "Add or update a storage variable",
    description = "Creates a new variable or updates an existing variable by ID/key in storage.",
    request_body = StorageVariable,
    responses(
        (status = 200, description = "Variable saved successfully", body = StorageVariable),
        (status = 400, description = "Invalid variable payload", body = crate::core::error::ApiErrorResponse),
        (status = 500, description = "Database write error", body = crate::core::error::ApiErrorResponse)
    )
)]
pub async fn add_variable(
    State(state): State<AppState>,
    Json(payload): Json<StorageVariable>,
) -> Result<Json<StorageVariable>, AutomaError> {
    let db = state.db.lock().await;
    let res = db.storage().save_variable(&payload)?;
    Ok(Json(res))
}

#[utoipa::path(
    tag = "Storage",
    delete,
    path = "/api/v1/storage/variables/{id}",
    operation_id = "delete_storage_variable",
    summary = "Delete a storage variable",
    description = "Permanently removes a variable from storage by ID or key.",
    params(
        ("id" = String, Path, description = "Variable ID or unique key")
    ),
    responses(
        (status = 200, description = "Variable deleted successfully"),
        (status = 404, description = "Variable not found", body = crate::core::error::ApiErrorResponse),
        (status = 500, description = "Database deletion error", body = crate::core::error::ApiErrorResponse)
    )
)]
pub async fn delete_variable(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, AutomaError> {
    let db = state.db.lock().await;
    db.storage().delete_variable(&id)?;
    Ok(StatusCode::OK)
}
