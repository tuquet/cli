use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::AppState;
use crate::core::browser::worker_coordinator::{connected_browsers, ensure_browser_worker};
use crate::core::engine::workflow_resolver::{WorkflowResolveError, WorkflowResolver};
use crate::core::models::id::{BrowserId, JobId};

/// Error types occurring during job coordination and lifecycle management
#[derive(Debug, thiserror::Error)]
pub enum JobCoordinatorError {
    #[error("Workflow resolution error: {0}")]
    WorkflowError(#[from] WorkflowResolveError),

    #[error("Browser worker unavailable: {0}")]
    BrowserUnavailable(String),

    #[error("Job not found: {0}")]
    NotFound(String),

    #[error("Internal coordinator error: {0}")]
    Internal(String),
}

/// Options configuring job submission
#[derive(Debug, Clone)]
pub struct JobExecutionOptions {
    pub browser_id: BrowserId,
    pub headless: Option<bool>,
    pub default_browser: Option<String>,
    pub variables: Option<serde_json::Value>,
    pub debug: Option<bool>,
    pub close_browser_on_finish: Option<bool>,
}

/// Coordinator orchestrating workflow job lifecycle, worker communication, and DB state
pub struct JobCoordinator;

impl JobCoordinator {
    /// Resolves workflow source, initializes cancellation token, launches browser worker, and dispatches job
    pub async fn submit(
        state: &AppState,
        workflow_id: Option<&str>,
        workflow_path: Option<&str>,
        workflow_data: Option<&serde_json::Value>,
        options: Option<crate::api::handlers::jobs::SubmitJobOptions>,
    ) -> Result<JobId, JobCoordinatorError> {
        let job_id = JobId::new(Uuid::new_v4().to_string());

        let resolved = WorkflowResolver::resolve(
            job_id.as_str(),
            &state.config.data_dir,
            workflow_id,
            workflow_path,
            workflow_data,
            &state.db,
        )
        .await
        .map_err(JobCoordinatorError::WorkflowError)?;

        let cancel_token = CancellationToken::new();

        // Load AppSettings from DB as default/fallback
        let app_settings = {
            let db = state.db.lock().await;
            db.settings().get_settings().unwrap_or_default()
        };

        // Store cancellation token for the active job
        state
            .active_jobs
            .write()
            .await
            .insert(job_id.to_string(), cancel_token.clone());

        // Extract browser options
        let browser_id = options
            .as_ref()
            .and_then(|o| o.browser_id.clone())
            .map(BrowserId::new)
            .unwrap_or_else(|| BrowserId::new("daemon_worker"));

        let headless_opt = options.as_ref().and_then(|o| o.headless);
        let default_browser_opt = options.as_ref().and_then(|o| o.default_browser.clone());

        ensure_browser_worker(browser_id.as_str(), &app_settings, headless_opt, default_browser_opt).await;

        let options_value = match &options {
            Some(opts) => serde_json::to_value(opts).unwrap_or(serde_json::Value::Null),
            None => serde_json::Value::Null,
        };

        let payload_str = serde_json::json!({
            "jobId": job_id.as_str(),
            "workflowPath": resolved.path.to_string_lossy().to_string(),
            "workflowData": resolved.data,
            "options": options_value,
            "browserId": browser_id.as_str()
        })
        .to_string();

        let is_connected = {
            let browsers = connected_browsers().read().await;
            browsers.contains(browser_id.as_str())
        };

        if !is_connected {
            // Clean up registered token if browser failed to connect
            state.active_jobs.write().await.remove(job_id.as_str());
            return Err(JobCoordinatorError::BrowserUnavailable(
                "Browser worker failed to connect after 30 seconds. This might happen if a browser process is already locking the browser.".to_string(),
            ));
        }

        if let Err(e) = state.worker_tx.send(payload_str) {
            tracing::warn!("Broadcasted job to worker channel with no active listeners: {}", e);
        }

        // Persist initial job record to SQLite database
        {
            let workflow_name = resolved
                .data
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("Workflow");
            let db = state.db.lock().await;
            let _ = db.jobs().create_job(
                job_id.as_str(),
                workflow_name,
                &resolved.data,
                &options_value,
                "running",
            );
        }

        Ok(job_id)
    }

    /// Aborts a running job by triggering its cancellation token and dispatching stop signal to worker
    pub async fn kill(state: &AppState, job_id: &JobId) -> Result<(), JobCoordinatorError> {
        let token_opt = {
            let active = state.active_jobs.read().await;
            active.get(job_id.as_str()).cloned()
        };

        if let Some(token) = token_opt {
            token.cancel();
            let event = crate::api::handlers::jobs::JobEvent {
                job_id: job_id.to_string(),
                event_type: "stop-workflow".to_string(),
            };
            if let Ok(stop_msg) = serde_json::to_string(&event) {
                let _ = state.worker_tx.send(stop_msg);
            }
            Ok(())
        } else {
            Err(JobCoordinatorError::NotFound(format!(
                "Job '{}' not found or not active",
                job_id
            )))
        }
    }

    /// Marks a job as completed in DB, cleans up cancellation token, and broadcasts completion event
    pub async fn finish(state: &AppState, job_id: &JobId) -> Result<(), JobCoordinatorError> {
        let mut active = state.active_jobs.write().await;
        if let Some(token) = active.remove(job_id.as_str()) {
            token.cancel();

            // Mark job as completed in SQLite DB
            {
                let db = state.db.lock().await;
                let _ = db
                    .jobs()
                    .finish_job(job_id.as_str(), "completed", &serde_json::Value::Null, 0);
            }

            let event = crate::api::handlers::jobs::JobEvent {
                job_id: job_id.to_string(),
                event_type: "workflow_finished".to_string(),
            };
            if let Ok(msg) = serde_json::to_string(&event) {
                let _ = state.tx.send(msg);
            }

            Ok(())
        } else {
            Err(JobCoordinatorError::NotFound(format!(
                "Job '{}' not found or already finished",
                job_id
            )))
        }
    }

    /// Appends execution logs into DB and broadcasts across SSE channel
    pub async fn record_log(
        state: &AppState,
        job_id: &JobId,
        payload: &crate::api::handlers::jobs::JobLogPayload,
    ) -> Result<(), JobCoordinatorError> {
        let data_val = if let Some(logs) = &payload.logs {
            serde_json::to_value(logs).unwrap_or(serde_json::Value::Null)
        } else {
            serde_json::json!(&payload.message)
        };

        // Save log to SQLite DB
        {
            let db = state.db.lock().await;
            if let Some(logs) = &payload.logs {
                for item in logs {
                    let log_type = item
                        .get("type")
                        .and_then(|t| t.as_str())
                        .unwrap_or(&payload.r#type);
                    let msg_str = serde_json::to_string(item).unwrap_or_else(|_| "{}".to_string());
                    let _ = db.jobs().insert_log(job_id.as_str(), log_type, &msg_str);
                }
            } else {
                let log_msg = payload.message.as_deref().unwrap_or(&payload.r#type);
                let _ = db.jobs().insert_log(job_id.as_str(), &payload.r#type, log_msg);
            }
        }

        let msg = serde_json::json!({
            "jobId": job_id.as_str(),
            "type": payload.r#type,
            "data": data_val
        })
        .to_string();
        tracing::debug!("[Job {}] Log: {}", job_id, msg);
        let _ = state.tx.send(msg);

        Ok(())
    }

    /// Queries the runtime status of a job
    pub async fn get_status(state: &AppState, job_id: &JobId) -> String {
        let is_active = state.active_jobs.read().await.contains_key(job_id.as_str());
        if is_active {
            "running".to_string()
        } else {
            "completed".to_string()
        }
    }
}
