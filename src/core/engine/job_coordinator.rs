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

        ensure_browser_worker(
            browser_id.as_str(),
            &app_settings,
            headless_opt,
            default_browser_opt,
        )
        .await;

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
            tracing::warn!(
                "Broadcasted job to worker channel with no active listeners: {}",
                e
            );
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

        // Native Zero-Extension Execution via CDP
        let session_opt = {
            let sessions = browser::browser_sessions().read().await;
            sessions.get(browser_id.as_str()).cloned()
        };

        if let Some(session) = session_opt {
            let state_clone = state.clone();
            let job_id_clone = job_id.clone();
            let browser_id_clone = browser_id.clone();
            let cancel_token_clone = cancel_token.clone();
            let workflow_data_val = resolved.data.clone();
            let close_browser = options
                .as_ref()
                .and_then(|o| o.close_browser_on_finish)
                .unwrap_or(true);
            let vars_opt = options.as_ref().and_then(|o| o.variables.clone());

            tokio::spawn(async move {
                Self::run_native_job(
                    state_clone,
                    job_id_clone,
                    browser_id_clone,
                    session,
                    workflow_data_val,
                    vars_opt,
                    cancel_token_clone,
                    close_browser,
                )
                .await;
            });
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
                let _ =
                    db.jobs()
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
                let _ = db
                    .jobs()
                    .insert_log(job_id.as_str(), &payload.r#type, log_msg);
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
        let db_status = {
            let db = state.db.lock().await;
            db.jobs().get_job_status(job_id.as_str()).ok().flatten()
        };
        match db_status {
            Some(st) => st,
            None => {
                if is_active {
                    "running".to_string()
                } else {
                    "completed".to_string()
                }
            }
        }
    }

    /// Executes a workflow natively via Rust CDP DAG engine with zero extensions
    pub async fn run_native_job(
        state: AppState,
        job_id: JobId,
        browser_id: BrowserId,
        session: browser::BrowserSession,
        workflow_data: serde_json::Value,
        variables: Option<serde_json::Value>,
        cancel_token: CancellationToken,
        close_browser_on_finish: bool,
    ) {
        let start_time = std::time::Instant::now();
        tracing::info!(
            "[run_native_job] Starting native CDP execution for job {}",
            job_id
        );

        let graph = match worker::drivers::cdp::WorkflowGraph::from_json(&workflow_data) {
            Ok(g) => g,
            Err(e) => {
                tracing::error!("Failed to parse workflow graph for job {}: {}", job_id, e);
                let _ = Self::finish_with_status(
                    &state,
                    &job_id,
                    "failed",
                    serde_json::json!({ "error": e }),
                    0,
                )
                .await;
                return;
            }
        };

        let cdp_session =
            match worker::drivers::cdp::CdpSession::connect(session.debugging_port).await {
                Ok(c) => c,
                Err(e) => {
                    tracing::error!(
                        "Failed to connect CDP session on port {}: {}",
                        session.debugging_port,
                        e
                    );
                    let _ = Self::finish_with_status(
                        &state,
                        &job_id,
                        "failed",
                        serde_json::json!({ "error": e }),
                        0,
                    )
                    .await;
                    return;
                }
            };

        let mut initial_state = worker::drivers::cdp::ExecutionState::new();
        if let Some(serde_json::Value::Object(map)) = variables {
            for (k, v) in map {
                initial_state.set_variable(&k, v);
            }
        }

        let executor = worker::drivers::cdp::WorkflowDagExecutor::new(
            graph,
            std::sync::Arc::new(cdp_session),
        );
        match executor.execute(Some(initial_state), cancel_token).await {
            Ok(final_state) => {
                let duration_ms = start_time.elapsed().as_millis() as i64;
                for step in &final_state.step_logs {
                    let log_payload = crate::api::handlers::jobs::JobLogPayload {
                        r#type: step.status.clone(),
                        message: Some(format!(
                            "{}: {}",
                            step.name,
                            step.message.as_deref().unwrap_or("ok")
                        )),
                        logs: None,
                    };
                    let _ = Self::record_log(&state, &job_id, &log_payload).await;
                }
                let results = serde_json::json!({
                    "variables": final_state.variables,
                    "table": final_state.table,
                    "stepLogs": final_state.step_logs,
                });
                let _ =
                    Self::finish_with_status(&state, &job_id, "completed", results, duration_ms)
                        .await;
            }
            Err(e) => {
                let duration_ms = start_time.elapsed().as_millis() as i64;
                tracing::error!("Workflow execution failed for job {}: {}", job_id, e);
                let _ = Self::finish_with_status(
                    &state,
                    &job_id,
                    "failed",
                    serde_json::json!({ "error": e }),
                    duration_ms,
                )
                .await;
            }
        }

        if close_browser_on_finish {
            let mut launchers = browser::browser_launchers().write().await;
            if let Some(mut launcher) = launchers.remove(browser_id.as_str()) {
                let _ = launcher.close().await;
            }
            let mut sessions = browser::browser_sessions().write().await;
            sessions.remove(browser_id.as_str());
            let mut reg = browser::browser_registry().write().await;
            reg.remove(browser_id.as_str());
            let mut connected = browser::connected_browsers().write().await;
            connected.remove(browser_id.as_str());
        }
    }

    pub async fn finish_with_status(
        state: &AppState,
        job_id: &JobId,
        status: &str,
        results: serde_json::Value,
        duration: i64,
    ) -> Result<(), JobCoordinatorError> {
        let mut active = state.active_jobs.write().await;
        active.remove(job_id.as_str());

        {
            let db = state.db.lock().await;
            let _ = db
                .jobs()
                .finish_job(job_id.as_str(), status, &results, duration);
        }

        let event = crate::api::handlers::jobs::JobEvent {
            job_id: job_id.to_string(),
            event_type: if status == "completed" {
                "workflow_finished".to_string()
            } else {
                "job_failed".to_string()
            },
        };
        if let Ok(msg) = serde_json::to_string(&event) {
            let _ = state.tx.send(msg);
        }

        Ok(())
    }
}
