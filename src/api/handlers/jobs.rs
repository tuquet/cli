use std::sync::Arc;
use axum::{
    extract::{Path, State, Json, Query},
    http::StatusCode,
    response::IntoResponse,
};
use utoipa::{IntoParams, ToSchema};

pub use crate::core::browser::worker_coordinator::{
    connected_browsers, ensure_browser_worker, get_browser_launcher_lock,
    resolve_cli_runner_extension_path,
};
use crate::core::engine::job_coordinator::{JobCoordinator, JobCoordinatorError};
use crate::core::engine::workflow_resolver::WorkflowResolveError;
use crate::core::models::id::JobId;

use serde::{Deserialize, Serialize};
use crate::AppState;
use crate::core::error::AutomaError;

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
#[schema(example = json!({"jobId": "job_123abc", "status": "running", "message": null}))]
/// Response returned after successfully submitting a workflow job
pub struct SubmitJobResponse {
    /// Unique identifier of the created execution job
    pub job_id: String,
    /// Initial lifecycle status (e.g. "queued", "running")
    pub status: String,
    /// Optional status or error message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
#[schema(example = json!({"status": "running"}))]
/// Current execution status of a background job
pub struct JobStatusResponse {
    /// Status name ("running", "completed", "failed", "cancelled")
    pub status: String,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
#[schema(example = json!({"jobId": "job_123abc"}))]
/// Active running job descriptor
pub struct ActiveJobResponse {
    /// Unique job identifier
    pub job_id: String,
}

#[derive(Serialize, Deserialize, ToSchema, Clone)]
#[serde(rename_all = "camelCase")]
/// Advanced runtime execution options for a workflow job
pub struct SubmitJobOptions {
    /// Dynamic variables to inject into the workflow execution scope
    #[schema(value_type = Option<Object>, example = json!({"keyword": "automation"}))]
    pub variables: Option<serde_json::Value>,
    /// Target browser instance ID (defaults to "daemon_worker")
    #[schema(example = "daemon_worker")]
    pub browser_id: Option<String>,
    /// Browser executable type ("chromium", "chrome", "edge", "firefox")
    #[schema(example = "chromium")]
    pub default_browser: Option<String>,
    /// Run browser in headless mode (no visible window)
    #[schema(example = true)]
    pub headless: Option<bool>,
    /// Enable verbose debugging and DevTools inspection
    #[schema(example = false)]
    pub debug: Option<bool>,
    /// Automatically close browser session when workflow execution finishes
    #[schema(example = true)]
    pub close_browser_on_finish: Option<bool>,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
#[schema(example = json!({
  "workflowId": "google_search",
  "options": {
    "variables": {
      "keyword": "automa automation"
    },
    "browserId": "daemon_worker",
    "headless": true,
    "debug": false,
    "closeBrowserOnFinish": true
  }
}))]
/// Request payload for submitting a new workflow execution job
pub struct SubmitJobPayload {
    /// Unique identifier of the workflow to execute
    #[serde(default, alias = "workflowId", alias = "workflow_id", alias = "id")]
    pub workflow_id: Option<String>,

    /// Absolute or relative filesystem path to the `.workflow.json` file (deprecated, prefer workflowId)
    #[serde(default, alias = "targetPath", alias = "workflow_path")]
    pub workflow_path: Option<String>,
    
    /// Raw inline workflow JSON payload (alternative to workflowPath or workflowId)
    #[serde(default, alias = "workflowData", alias = "workflow")]
    #[schema(value_type = Option<Object>)]
    pub workflow_data: Option<serde_json::Value>,
    
    /// Optional execution options and browser configuration
    pub options: Option<SubmitJobOptions>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
/// Internal telemetry event dispatched across SSE channels
pub struct JobEvent {
    /// Associated job identifier
    #[serde(rename = "jobId")]
    pub job_id: String,
    /// Event type descriptor (e.g. "workflow_finished", "log")
    #[serde(rename = "type")]
    pub event_type: String,
}

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
    ).await {
        Ok(job_id) => (
            StatusCode::OK,
            axum::Json(SubmitJobResponse {
                job_id: job_id.to_string(),
                status: "queued".to_string(),
                message: None,
            }),
        ).into_response(),
        Err(JobCoordinatorError::WorkflowError(WorkflowResolveError::BadRequest(msg))) => (
            StatusCode::BAD_REQUEST,
            axum::Json(SubmitJobResponse {
                job_id: "".to_string(),
                status: "error".to_string(),
                message: Some(msg),
            }),
        ).into_response(),
        Err(JobCoordinatorError::WorkflowError(WorkflowResolveError::NotFound(msg))) => (
            StatusCode::NOT_FOUND,
            axum::Json(SubmitJobResponse {
                job_id: "".to_string(),
                status: "error".to_string(),
                message: Some(msg),
            }),
        ).into_response(),
        Err(JobCoordinatorError::WorkflowError(WorkflowResolveError::Internal(msg))) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            axum::Json(SubmitJobResponse {
                job_id: "".to_string(),
                status: "error".to_string(),
                message: Some(msg),
            }),
        ).into_response(),
        Err(JobCoordinatorError::BrowserUnavailable(msg)) => (
            StatusCode::SERVICE_UNAVAILABLE,
            axum::Json(SubmitJobResponse {
                job_id: "".to_string(),
                status: "error".to_string(),
                message: Some(msg),
            }),
        ).into_response(),
        Err(JobCoordinatorError::NotFound(msg)) => (
            StatusCode::NOT_FOUND,
            axum::Json(SubmitJobResponse {
                job_id: "".to_string(),
                status: "error".to_string(),
                message: Some(msg),
            }),
        ).into_response(),
        Err(JobCoordinatorError::Internal(msg)) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            axum::Json(SubmitJobResponse {
                job_id: "".to_string(),
                status: "error".to_string(),
                message: Some(msg),
            }),
        ).into_response(),
    }
}

use axum::response::sse::{Event, Sse};
use futures::stream::Stream;
use std::convert::Infallible;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

#[derive(Deserialize)]
pub struct WorkerSseQuery {
    #[serde(rename = "browserId")]
    pub browser_id: Option<String>,
}

#[utoipa::path(
    tag = "Jobs",
    get,
    path = "/api/v1/internal/worker/events",
    operation_id = "worker_sse",
    summary = "Subscribe to worker event stream (SSE)",
    description = "Establishes a long-lived Server-Sent Events (SSE) connection used by browser workers to receive workflow job dispatches and cancellation commands.",
    params(
        ("browserId" = Option<String>, Query, description = "Target browser instance identifier (defaults to 'daemon_worker')")
    ),
    responses(
        (status = 200, description = "SSE Stream for Worker jobs", content_type = "text/event-stream")
    )
)]
pub async fn worker_sse(
    State(state): State<AppState>,
    axum::extract::Query(query): axum::extract::Query<WorkerSseQuery>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = state.worker_tx.subscribe();
    let expected_browser = query.browser_id.unwrap_or_else(|| "daemon_worker".to_string());

    // Register the connected browser
    {
        let mut browsers = connected_browsers().write().await;
        browsers.insert(expected_browser.clone());
    }

    // Create a guard to unregister on drop
    struct BrowserGuard {
        browser_id: String,
    }
    impl Drop for BrowserGuard {
        fn drop(&mut self) {
            let browser_id = self.browser_id.clone();
            if let Ok(handle) = tokio::runtime::Handle::try_current() {
                handle.spawn(async move {
                    let mut browsers = connected_browsers().write().await;
                    browsers.remove(&browser_id);
                });
            }
        }
    }
    let _guard = Arc::new(BrowserGuard { browser_id: expected_browser.clone() });

    let stream = BroadcastStream::new(rx).filter_map(move |msg| {
        let _guard_clone = _guard.clone();
        let expected_browser = expected_browser.clone();
        let data = msg.ok()?;
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&data) {
            if let Some(msg_browser) = json.get("browserId").and_then(|v| v.as_str()) {
                if msg_browser != expected_browser {
                    return None;
                }
            }
        }
        Some(Ok(Event::default().data(data)))
    });

    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(std::time::Duration::from_secs(2))
    )
}

#[derive(Deserialize, ToSchema)]
#[schema(example = json!({"type": "info", "message": "Executing node 1", "logs": null}))]
/// Payload containing execution logs and telemetry emitted by a workflow step
pub struct JobLogPayload {
    /// Log severity level ("info", "warn", "error", "debug")
    #[schema(example = "info")]
    pub r#type: String,
    /// Human-readable log message
    #[schema(example = "Execution started")]
    pub message: Option<String>,
    /// Array of structured log entries or node execution telemetry
    #[schema(value_type = Option<Vec<Object>>, example = json!([{"timestamp": 123456789, "details": "Node 1 finished"}]))]
    pub logs: Option<Vec<serde_json::Value>>,
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

#[derive(Deserialize, IntoParams)]
pub struct GetActiveJobsQuery {
    /// Maximum number of active jobs to return
    #[param(example = 50)]
    pub limit: Option<usize>,
    /// Number of items to skip for pagination (default 0)
    #[param(example = 0)]
    pub offset: Option<usize>,
    /// Optional search query to filter active jobs by job ID
    pub search: Option<String>,
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
    let mut jobs: Vec<ActiveJobResponse> = active.keys()
        .filter(|id| {
            if let Some(ref search) = query.search {
                id.to_lowercase().contains(&search.to_lowercase())
            } else {
                true
            }
        })
        .map(|id| ActiveJobResponse {
            job_id: id.clone()
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


#[cfg(test)]
mod tests {
    use super::*;
    use crate::AppState;
    use crate::config::AppConfig;
    use crate::infrastructure::db::AutomaDb;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
        routing::{post, patch},
        Router,
    };
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use tower::ServiceExt;
    use std::collections::HashMap;

    async fn create_test_app() -> Router {
        let db = Arc::new(Mutex::new(AutomaDb::new_in_memory().unwrap()));
        let (tx, _) = tokio::sync::broadcast::channel(100);
        let (worker_tx, _) = tokio::sync::broadcast::channel(100);
        let config = Arc::new(AppConfig::load());
        
        let state = AppState {
            db,
            config,
            tx,
            worker_tx,
            active_jobs: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        };

        Router::new()
            .route("/api/jobs", post(submit_job))
            .route("/api/jobs/{job_id}/status", patch(job_finish))
            .with_state(state)
    }

    #[tokio::test]
    async fn test_submit_job_valid() {
        let app = create_test_app().await;

        let payload = serde_json::json!({
            "workflowPath": "test.workflow.json",
            "options": {
                "browserId": "test_browser"
            }
        });

        // Mock a connected browser so it doesn't try to launch one
        {
            let mut browsers = connected_browsers().write().await;
            browsers.insert("test_browser".to_string());
        }
        
        // Write a dummy workflow file to pass the file read check
        tokio::fs::write("test.workflow.json", r#"{"name":"Test","nodes":[]}"#).await.unwrap();

        let req = Request::builder()
            .method("POST")
            .uri("/api/jobs")
            .header("Content-Type", "application/json")
            .body(Body::from(payload.to_string()))
            .unwrap();

        let response = app.oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        
        let _ = tokio::fs::remove_file("test.workflow.json").await;
    }

    #[tokio::test]
    async fn test_patch_job_status_invalid_id() {
        let app = create_test_app().await;

        let req = Request::builder()
            .method("PATCH")
            .uri("/api/jobs/invalid_id_123/status")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_job_log_handler_saves_to_db() {
        let db = Arc::new(Mutex::new(AutomaDb::new_in_memory().unwrap()));
        let (tx, mut rx) = tokio::sync::broadcast::channel(100);
        let (worker_tx, _) = tokio::sync::broadcast::channel(100);
        let config = Arc::new(AppConfig::load());
        
        let state = AppState {
            db: db.clone(),
            config,
            tx,
            worker_tx,
            active_jobs: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        };

        let log_payload = JobLogPayload {
            r#type: "info".to_string(),
            message: Some("Step 1 finished".to_string()),
            logs: None,
        };

        let res = job_log(State(state), Path(JobId::new("job_test_1")), Json(log_payload)).await.unwrap();
        assert_eq!(res, StatusCode::OK);

        // Verify broadcast event was sent
        let broadcasted = rx.recv().await.unwrap();
        assert!(broadcasted.contains("job_test_1"));
        assert!(broadcasted.contains("Step 1 finished"));
    }

    #[tokio::test]
    async fn test_kill_job_not_found() {
        let db = Arc::new(Mutex::new(AutomaDb::new_in_memory().unwrap()));
        let (tx, _) = tokio::sync::broadcast::channel(100);
        let (worker_tx, _) = tokio::sync::broadcast::channel(100);
        let config = Arc::new(AppConfig::load());
        
        let state = AppState {
            db,
            config,
            tx,
            worker_tx,
            active_jobs: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        };

        let res = kill_job(State(state), Path(JobId::new("non_existent_job_123"))).await;
        assert!(res.is_err());
    }
}
