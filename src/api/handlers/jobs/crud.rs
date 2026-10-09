use axum::{
    extract::{Json, Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};

use crate::AppState;
use crate::core::engine::job_coordinator::{JobCoordinator, JobCoordinatorError};
use crate::core::engine::workflow_resolver::WorkflowResolveError;
use crate::core::error::AutomaError;
use crate::core::models::id::JobId;

use super::types::{
    ActiveJobResponse, GetActiveJobsQuery, JobControlResponse, JobEvent, JobLogPayload,
    JobStatusResponse, SubmitJobPayload, SubmitJobResponse,
};

#[utoipa::path(
    tag = "Jobs",
    post,
    path = "/api/v1/jobs",
    operation_id = "submit_job",
    summary = "Submit a workflow execution job",
    description = "Submits a new automation workflow job to be executed by a browser worker instance. Supports workflowId, inline workflowData, or filesystem workflowPath. Automatically ensures the required browser instance is launched and connected.",
    request_body = SubmitJobPayload,
    responses(
        (status = 200, description = "Job submitted and accepted for execution", body = SubmitJobResponse),
        (status = 400, description = "Invalid request payload or unreadable workflow file", body = crate::core::error::ApiErrorResponse),
        (status = 429, description = "Maximum concurrent jobs reached", body = SubmitJobResponse),
        (status = 503, description = "Browser worker unavailable or connection timeout", body = SubmitJobResponse)
    )
)]
pub async fn submit_job(
    State(state): State<AppState>,
    Json(payload): Json<SubmitJobPayload>,
) -> impl IntoResponse {
    match JobCoordinator::submit(
        &state,
        payload.workflow_id.as_deref(),
        payload.workflow_path.as_deref(),
        payload.workflow_data.as_ref(),
        payload.options,
    )
    .await
    {
        Ok(job_id) => (
            StatusCode::OK,
            axum::Json(SubmitJobResponse {
                job_id: job_id.to_string(),
                status: "queued".to_string(),
                message: None,
            }),
        )
            .into_response(),
        Err(JobCoordinatorError::WorkflowError(WorkflowResolveError::BadRequest(msg))) => (
            StatusCode::BAD_REQUEST,
            axum::Json(SubmitJobResponse {
                job_id: "".to_string(),
                status: "error".to_string(),
                message: Some(msg),
            }),
        )
            .into_response(),
        Err(JobCoordinatorError::WorkflowError(WorkflowResolveError::NotFound(msg))) => (
            StatusCode::NOT_FOUND,
            axum::Json(SubmitJobResponse {
                job_id: "".to_string(),
                status: "error".to_string(),
                message: Some(msg),
            }),
        )
            .into_response(),
        Err(JobCoordinatorError::WorkflowError(WorkflowResolveError::Internal(msg))) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            axum::Json(SubmitJobResponse {
                job_id: "".to_string(),
                status: "error".to_string(),
                message: Some(msg),
            }),
        )
            .into_response(),
        Err(JobCoordinatorError::BrowserUnavailable(msg)) => (
            StatusCode::SERVICE_UNAVAILABLE,
            axum::Json(SubmitJobResponse {
                job_id: "".to_string(),
                status: "error".to_string(),
                message: Some(msg),
            }),
        )
            .into_response(),
        Err(JobCoordinatorError::NotFound(msg)) => (
            StatusCode::NOT_FOUND,
            axum::Json(SubmitJobResponse {
                job_id: "".to_string(),
                status: "error".to_string(),
                message: Some(msg),
            }),
        )
            .into_response(),
        Err(JobCoordinatorError::Internal(msg)) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            axum::Json(SubmitJobResponse {
                job_id: "".to_string(),
                status: "error".to_string(),
                message: Some(msg),
            }),
        )
            .into_response(),
    }
}

#[utoipa::path(
    tag = "Jobs",
    post,
    path = "/api/v1/jobs/{job_id}/logs",
    operation_id = "append_job_log",
    summary = "Append execution log entry for a job",
    description = "Receives runtime log entries and block execution telemetry from the browser worker, persisting them to the database and broadcasting via SSE.",
    params(
        ("job_id" = String, Path, description = "Unique job identifier")
    ),
    request_body = JobLogPayload,
    responses(
        (status = 200, description = "Log entry received and persisted successfully"),
        (status = 400, description = "Invalid payload or job not found", body = crate::core::error::ApiErrorResponse)
    )
)]
pub async fn job_log(
    State(state): State<AppState>,
    Path(job_id): Path<JobId>,
    Json(payload): Json<JobLogPayload>,
) -> Result<StatusCode, AutomaError> {
    JobCoordinator::record_log(&state, &job_id, &payload)
        .await
        .map_err(|e| AutomaError::Internal(e.to_string()))?;
    Ok(StatusCode::OK)
}

#[utoipa::path(
    tag = "Jobs",
    patch,
    path = "/api/v1/jobs/{job_id}/status",
    operation_id = "finish_job",
    summary = "Mark a job as completed",
    description = "Signals the completion of a workflow execution job, records the final status in SQLite, releases active tokens, and notifies subscribers.",
    params(
        ("job_id" = String, Path, description = "Unique job identifier")
    ),
    responses(
        (status = 200, description = "Job marked as completed successfully"),
        (status = 404, description = "Job not found or already finished", body = crate::core::error::ApiErrorResponse)
    )
)]
pub async fn job_finish(
    State(state): State<AppState>,
    Path(job_id): Path<JobId>,
) -> Result<StatusCode, AutomaError> {
    JobCoordinator::finish(&state, &job_id)
        .await
        .map_err(|e| match e {
            JobCoordinatorError::NotFound(_) => {
                AutomaError::NotFound(format!("Job '{job_id}' not found or already finished"))
            }
            other => AutomaError::Internal(other.to_string()),
        })?;
    Ok(StatusCode::OK)
}

#[utoipa::path(
    tag = "Jobs",
    get,
    path = "/api/v1/jobs/{job_id}/status",
    operation_id = "get_job_status",
    summary = "Get current status of a job",
    description = "Queries the runtime lifecycle status of a specific job by its unique identifier.",
    params(
        ("job_id" = String, Path, description = "Unique job identifier")
    ),
    responses(
        (status = 200, description = "Current job execution status", body = JobStatusResponse),
        (status = 404, description = "Job not found", body = crate::core::error::ApiErrorResponse)
    )
)]
pub async fn get_job_status(
    State(state): State<AppState>,
    Path(job_id): Path<JobId>,
) -> Result<Json<JobStatusResponse>, AutomaError> {
    let status = JobCoordinator::get_status(&state, &job_id).await;
    Ok(Json(JobStatusResponse { status }))
}

#[utoipa::path(
    tag = "Jobs",
    get,
    path = "/api/v1/jobs",
    operation_id = "get_active_jobs",
    summary = "List all active running jobs",
    description = "Returns an array of identifiers for all workflow execution jobs currently running in the daemon.",
    params(GetActiveJobsQuery),
    responses(
        (status = 200, description = "List of active running jobs", body = Vec<ActiveJobResponse>)
    )
)]
pub async fn get_active_jobs(
    State(state): State<AppState>,
    Query(query): Query<GetActiveJobsQuery>,
) -> Result<Json<Vec<ActiveJobResponse>>, AutomaError> {
    let active = state.active_jobs.read().await;
    let mut jobs: Vec<ActiveJobResponse> = active
        .keys()
        .filter(|id| {
            if let Some(ref search) = query.search {
                id.to_lowercase().contains(&search.to_lowercase())
            } else {
                true
            }
        })
        .map(|id| ActiveJobResponse {
            job_id: id.clone(),
        })
        .collect();

    jobs.sort_by(|a, b| a.job_id.cmp(&b.job_id));

    let offset = query.offset.unwrap_or(0);
    let limit = query.limit.unwrap_or(usize::MAX);
    let paginated = jobs.into_iter().skip(offset).take(limit).collect();

    Ok(Json(paginated))
}

#[utoipa::path(
    tag = "Jobs",
    delete,
    path = "/api/v1/jobs/{job_id}",
    operation_id = "kill_job",
    summary = "Terminate a running job",
    description = "Cancels execution token for a job, releases the associated browser process, records terminated status, and clears state.",
    params(
        ("job_id" = String, Path, description = "Unique job identifier")
    ),
    responses(
        (status = 200, description = "Job terminated successfully"),
        (status = 404, description = "Job not found or not active", body = crate::core::error::ApiErrorResponse)
    )
)]
pub async fn kill_job(
    State(state): State<AppState>,
    Path(job_id): Path<JobId>,
) -> Result<StatusCode, AutomaError> {
    JobCoordinator::kill(&state, &job_id)
        .await
        .map_err(|e| match e {
            JobCoordinatorError::NotFound(_) => {
                AutomaError::NotFound(format!("Job '{job_id}' not found or not active"))
            }
            other => AutomaError::Internal(other.to_string()),
        })?;
    Ok(StatusCode::OK)
}

#[utoipa::path(
    tag = "Jobs",
    post,
    path = "/api/v1/jobs/{job_id}/pause",
    operation_id = "pause_job",
    summary = "Pause a running job",
    description = "Dispatches a pause-workflow signal to the worker, updates job status to 'paused', and broadcasts event via SSE.",
    params(
        ("job_id" = String, Path, description = "Unique job identifier")
    ),
    responses(
        (status = 200, description = "Job paused successfully", body = JobControlResponse),
        (status = 404, description = "Job not found or not active", body = crate::core::error::ApiErrorResponse)
    )
)]
pub async fn pause_job(
    State(state): State<AppState>,
    Path(job_id): Path<JobId>,
) -> Result<Json<JobControlResponse>, AutomaError> {
    let is_active = state.active_jobs.read().await.contains_key(job_id.as_str());
    if !is_active {
        return Err(AutomaError::NotFound(format!(
            "Job '{job_id}' not found or not active"
        )));
    }

    // 1. Update status in SQLite DB
    {
        let db = state.db.lock().await;
        let _ = db.jobs().update_job_status(job_id.as_str(), "paused");
    }

    // 2. Dispatch pause event to browser worker
    let event = JobEvent {
        job_id: job_id.to_string(),
        event_type: "pause-workflow".to_string(),
    };
    if let Ok(msg) = serde_json::to_string(&event) {
        let _ = state.worker_tx.send(msg);
    }

    // 3. Broadcast to global SSE
    let _ = state.tx.send(
        serde_json::json!({
            "type": "job_status_changed",
            "jobId": job_id.as_str(),
            "status": "paused"
        })
        .to_string(),
    );

    Ok(Json(JobControlResponse {
        success: true,
        message: format!("Job '{}' paused successfully", job_id),
        job_id: job_id.to_string(),
        status: "paused".to_string(),
    }))
}

#[utoipa::path(
    tag = "Jobs",
    post,
    path = "/api/v1/jobs/{job_id}/resume",
    operation_id = "resume_job",
    summary = "Resume a paused job",
    description = "Dispatches a resume-workflow signal to the worker, updates job status to 'running', and broadcasts event via SSE.",
    params(
        ("job_id" = String, Path, description = "Unique job identifier")
    ),
    responses(
        (status = 200, description = "Job resumed successfully", body = JobControlResponse),
        (status = 404, description = "Job not found or not active", body = crate::core::error::ApiErrorResponse)
    )
)]
pub async fn resume_job(
    State(state): State<AppState>,
    Path(job_id): Path<JobId>,
) -> Result<Json<JobControlResponse>, AutomaError> {
    let is_active = state.active_jobs.read().await.contains_key(job_id.as_str());
    if !is_active {
        return Err(AutomaError::NotFound(format!(
            "Job '{job_id}' not found or not active"
        )));
    }

    // 1. Update status in SQLite DB
    {
        let db = state.db.lock().await;
        let _ = db.jobs().update_job_status(job_id.as_str(), "running");
    }

    // 2. Dispatch resume event to browser worker
    let event = JobEvent {
        job_id: job_id.to_string(),
        event_type: "resume-workflow".to_string(),
    };
    if let Ok(msg) = serde_json::to_string(&event) {
        let _ = state.worker_tx.send(msg);
    }

    // 3. Broadcast to global SSE
    let _ = state.tx.send(
        serde_json::json!({
            "type": "job_status_changed",
            "jobId": job_id.as_str(),
            "status": "running"
        })
        .to_string(),
    );

    Ok(Json(JobControlResponse {
        success: true,
        message: format!("Job '{}' resumed successfully", job_id),
        job_id: job_id.to_string(),
        status: "running".to_string(),
    }))
}
