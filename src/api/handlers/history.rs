use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use crate::infrastructure::db::{JobHistoryItem, JobDetails};
use crate::core::error::AutomaError;
use crate::core::models::id::JobId;

#[derive(Deserialize, IntoParams)]
pub struct HistoryQuery {
    /// Maximum number of history entries to return (default 50)
    #[param(example = 50)]
    pub limit: Option<usize>,
    /// Number of items to skip for pagination (default 0)
    #[param(example = 0)]
    pub offset: Option<usize>,
    /// Optional search query to filter history by workflow name or job ID
    pub search: Option<String>,
    /// Optional status filter (e.g. "completed", "failed", "running")
    pub status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[schema(example = json!({"success": true, "message": "Job history deleted"}))]
/// Response returned after performing a history modification or purge action
pub struct HistoryActionResponse {
    /// Whether the history action succeeded
    pub success: bool,
    /// Detailed status message
    pub message: String,
}

#[utoipa::path(
    tag = "History",
    get,
    path = "/api/v1/history",
    operation_id = "get_job_history",
    summary = "Get paginated job execution history",
    description = "Queries previous workflow execution runs and audit logs persisted in the SQLite database.",
    params(HistoryQuery),
    responses(
        (status = 200, description = "List of past job execution summaries", body = Vec<JobHistoryItem>),
        (status = 500, description = "Database or thread pool failure", body = crate::core::error::ApiErrorResponse)
    )
)]
pub async fn get_history(
    State(state): State<crate::AppState>,
    Query(query): Query<HistoryQuery>,
) -> Result<Json<Vec<JobHistoryItem>>, AutomaError> {
    let db = state.db.lock().await;
    let history_res = db.jobs().get_history(query.limit, query.offset, query.search.as_deref(), query.status.as_deref())
        .map_err(|e| AutomaError::DatabaseError(e.to_string()))?;

    Ok(Json(history_res))
}

#[utoipa::path(
    tag = "History",
    get,
    path = "/api/v1/history/{job_id}/logs",
    operation_id = "get_job_execution_logs",
    summary = "Get execution details and logs for a job",
    description = "Retrieves full execution audit trail, timing breakdown, and detailed step logs for a specific job.",
    params(
        ("job_id" = String, Path, description = "Unique job identifier")
    ),
    responses(
        (status = 200, description = "Detailed job execution record and logs", body = JobDetails),
        (status = 404, description = "Job not found", body = crate::core::error::ApiErrorResponse),
        (status = 500, description = "Database query failure", body = crate::core::error::ApiErrorResponse)
    )
)]
pub async fn get_logs(
    State(state): State<crate::AppState>,
    Path(job_id): Path<JobId>,
) -> Result<Json<JobDetails>, AutomaError> {
    let db = state.db.lock().await;
    let logs_res = db.jobs().get_job_details(job_id.as_str())
        .map_err(|e| AutomaError::DatabaseError(e.to_string()))?;

    Ok(Json(logs_res))
}

#[utoipa::path(
    tag = "History",
    delete,
    path = "/api/v1/history/{job_id}",
    operation_id = "delete_job_history_item",
    summary = "Delete a single job history item",
    description = "Removes a specific job run and its associated logs from the SQLite history store.",
    params(
        ("job_id" = String, Path, description = "Unique job identifier to delete")
    ),
    responses(
        (status = 200, description = "Job history entry deleted successfully", body = HistoryActionResponse),
        (status = 404, description = "Job not found", body = crate::core::error::ApiErrorResponse),
        (status = 500, description = "Database deletion failure", body = crate::core::error::ApiErrorResponse)
    )
)]
pub async fn delete_history_item(
    State(state): State<crate::AppState>,
    Path(job_id): Path<JobId>,
) -> Result<Json<HistoryActionResponse>, AutomaError> {
    let db = state.db.lock().await;
    db.jobs().delete_job(job_id.as_str())
        .map_err(|e| AutomaError::DatabaseError(e.to_string()))?;
    
    Ok(Json(HistoryActionResponse {
        success: true,
        message: "Job history deleted".to_string(),
    }))
}

#[utoipa::path(
    tag = "History",
    delete,
    path = "/api/v1/history",
    operation_id = "clear_all_job_history",
    summary = "Clear entire job execution history",
    description = "Permanently truncates the job execution history and purge all logs from the database.",
    responses(
        (status = 200, description = "All job history cleared successfully", body = HistoryActionResponse),
        (status = 500, description = "Database purge failure", body = crate::core::error::ApiErrorResponse)
    )
)]
pub async fn clear_history(
    State(state): State<crate::AppState>,
) -> Result<Json<HistoryActionResponse>, AutomaError> {
    let db = state.db.lock().await;
    let success = db.jobs().clear_all_jobs();
    if !success {
        return Err(AutomaError::DatabaseError("Failed to clear jobs from database".to_string()));
    }
    
    Ok(Json(HistoryActionResponse {
        success: true,
        message: "History cleared".to_string(),
    }))
}
