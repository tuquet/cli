use axum::{
    extract::{Path, Json, Query, State},
    http::StatusCode,
};
use crate::AppState;
use crate::core::error::AutomaError;
use crate::core::models::storage::StorageCredential;
use serde::Deserialize;
use utoipa::IntoParams;

#[derive(Deserialize, IntoParams)]
pub struct GetCredentialsQuery {
    /// Maximum number of credentials to return
    #[param(example = 50)]
    pub limit: Option<usize>,
    /// Number of items to skip for pagination (default 0)
    #[param(example = 0)]
    pub offset: Option<usize>,
    /// Optional search query to filter credentials by name, key or ID
    pub search: Option<String>,
}

#[utoipa::path(
    tag = "Storage",
    get,
    path = "/api/v1/storage/credentials",
    operation_id = "get_storage_credentials",
    summary = "List all stored credentials",
    description = "Retrieves all saved authentication credentials from storage.",
    params(GetCredentialsQuery),
    responses(
        (status = 200, description = "List of credentials", body = Vec<StorageCredential>),
        (status = 500, description = "Database read error", body = crate::core::error::ApiErrorResponse)
    )
)]
pub async fn get_credentials(
    State(state): State<AppState>,
    Query(query): Query<GetCredentialsQuery>,
) -> Result<Json<Vec<StorageCredential>>, AutomaError> {
    let db = state.db.lock().await;
    let creds = db.storage().get_credentials(query.limit, query.offset, query.search.as_deref())?;
    Ok(Json(creds))
}

#[utoipa::path(
    tag = "Storage",
    post,
    path = "/api/v1/storage/credentials",
    operation_id = "add_storage_credential",
    summary = "Add or update a stored credential",
    description = "Persists a new credential or updates an existing credential in storage.",
    request_body = StorageCredential,
    responses(
        (status = 200, description = "Credential saved successfully", body = StorageCredential),
        (status = 400, description = "Invalid credential payload", body = crate::core::error::ApiErrorResponse),
        (status = 500, description = "Database write error", body = crate::core::error::ApiErrorResponse)
    )
)]
pub async fn add_credential(
    State(state): State<AppState>,
    Json(payload): Json<StorageCredential>,
) -> Result<Json<StorageCredential>, AutomaError> {
    let db = state.db.lock().await;
    let res = db.storage().save_credential(&payload)?;
    Ok(Json(res))
}

#[utoipa::path(
    tag = "Storage",
    delete,
    path = "/api/v1/storage/credentials/{id}",
    operation_id = "delete_storage_credential",
    summary = "Delete a stored credential",
    description = "Permanently removes a credential from storage by ID or key.",
    params(
        ("id" = String, Path, description = "Credential ID or unique key")
    ),
    responses(
        (status = 200, description = "Credential deleted successfully"),
        (status = 404, description = "Credential not found", body = crate::core::error::ApiErrorResponse),
        (status = 500, description = "Database deletion error", body = crate::core::error::ApiErrorResponse)
    )
)]
pub async fn delete_credential(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, AutomaError> {
    let db = state.db.lock().await;
    db.storage().delete_credential(&id)?;
    Ok(StatusCode::OK)
}
