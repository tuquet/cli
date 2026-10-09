pub mod crud;
pub mod sse;
pub mod types;

pub use crud::{
    get_active_jobs, get_job_status, job_finish, job_log, kill_job, pause_job, resume_job,
    submit_job,
    __path_get_active_jobs, __path_get_job_status, __path_job_finish, __path_job_log,
    __path_kill_job, __path_pause_job, __path_resume_job, __path_submit_job,
};
pub use sse::{worker_sse, __path_worker_sse};
pub use types::{
    ActiveJobResponse, GetActiveJobsQuery, JobControlResponse, JobEvent, JobLogPayload,
    JobStatusResponse, SubmitJobOptions, SubmitJobPayload, SubmitJobResponse, WorkerSseQuery,
};

pub use crate::core::browser::worker_coordinator::{
    connected_browsers, ensure_browser_worker, get_browser_launcher_lock,
    resolve_cli_runner_extension_path,
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;
    use crate::infrastructure::db::AutomaDb;
    use crate::AppState;
    use axum::{
        body::Body,
        extract::{Json, Path, State},
        http::{Request, StatusCode},
        routing::{patch, post},
        Router,
    };
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use tower::ServiceExt;

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
        tokio::fs::write("test.workflow.json", r#"{"name":"Test","nodes":[]}"#)
            .await
            .unwrap();

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

        let res = job_log(
            State(state),
            Path(crate::core::models::id::JobId::new("job_test_1")),
            Json(log_payload),
        )
        .await
        .unwrap();
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

        let res = kill_job(
            State(state),
            Path(crate::core::models::id::JobId::new("non_existent_job_123")),
        )
        .await;
        assert!(res.is_err());
    }
}
