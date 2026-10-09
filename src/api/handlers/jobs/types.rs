use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

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

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
#[schema(example = json!({"success": true, "message": "Job paused successfully", "jobId": "job_123", "status": "paused"}))]
/// Response after controlling job execution lifecycle (pause, resume)
pub struct JobControlResponse {
    /// Operation success status
    pub success: bool,
    /// Informational message
    pub message: String,
    /// Target job ID
    pub job_id: String,
    /// New lifecycle status
    pub status: String,
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

#[derive(Deserialize)]
pub struct WorkerSseQuery {
    #[serde(rename = "browserId")]
    pub browser_id: Option<String>,
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
