use tower_http::cors::{Any, CorsLayer};
use axum::{routing::get, routing::post, routing::delete, response::IntoResponse, Router};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;
use crate::api::handlers::history::{
    get_history, get_logs, delete_history_item, clear_history
};
use crate::api::handlers::sse::sse;
use crate::api::handlers::health::health;
use crate::api::handlers::jobs::{
    submit_job, get_job_status, get_active_jobs, kill_job, pause_job, resume_job,
    worker_sse, job_log, job_finish
};
use crate::api::handlers::system::{
    install_browser, open_studio, kill_browsers, get_metrics, get_system_info,
    get_cloud_status, trigger_cloud_sync, cloud_login, cloud_logout
};
use crate::api::handlers::lint::lint_workflow;
use crate::api::handlers::storage::{
    get_variables, add_variable, delete_variable,
    get_credentials, add_credential, delete_credential,
    get_tables, add_table, delete_table,
    get_table_rows, add_table_row,
    get_workflows, get_workflow_by_id,
    create_storage_workflow, update_storage_workflow, delete_storage_workflow,
    import_storage_workflow, restore_storage_backup, export_storage_backup,
    get_workflow, save_workflow
};
use crate::api::handlers::browsers::{
    get_browsers, create_browser, update_browser, delete_browser, get_browser_detail, start_browser, stop_browser, import_csv, sideload_extension, auto_detect_browsers, set_default_browser
};
use crate::api::handlers::settings::{get_settings, update_settings, patch_settings};
use crate::api::handlers::ws::ws_handler;

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::api::handlers::jobs::submit_job,
        crate::api::handlers::jobs::get_job_status,
        crate::api::handlers::jobs::get_active_jobs,
        crate::api::handlers::jobs::kill_job,
        crate::api::handlers::jobs::pause_job,
        crate::api::handlers::jobs::resume_job,
        crate::api::handlers::jobs::worker_sse,
        crate::api::handlers::jobs::job_log,
        crate::api::handlers::jobs::job_finish,
        crate::api::handlers::history::get_history,
        crate::api::handlers::history::get_logs,
        crate::api::handlers::history::delete_history_item,
        crate::api::handlers::history::clear_history,
        crate::api::handlers::health::health,
        crate::api::handlers::sse::sse,
        crate::api::handlers::system::install_browser,
        crate::api::handlers::system::kill_browsers,
        crate::api::handlers::system::open_studio,
        crate::api::handlers::system::get_metrics,
        crate::api::handlers::system::get_system_info,
        crate::api::handlers::system::get_cloud_status,
        crate::api::handlers::system::trigger_cloud_sync,
        crate::api::handlers::system::cloud_login,
        crate::api::handlers::system::cloud_logout,
        crate::api::handlers::lint::lint_workflow,
        crate::api::handlers::storage::get_variables,
        crate::api::handlers::storage::add_variable,
        crate::api::handlers::storage::delete_variable,
        crate::api::handlers::storage::get_credentials,
        crate::api::handlers::storage::add_credential,
        crate::api::handlers::storage::delete_credential,
        crate::api::handlers::storage::get_tables,
        crate::api::handlers::storage::add_table,
        crate::api::handlers::storage::delete_table,
        crate::api::handlers::storage::get_table_rows,
        crate::api::handlers::storage::add_table_row,
        crate::api::handlers::storage::get_workflows,
        crate::api::handlers::storage::get_workflow_by_id,
        crate::api::handlers::storage::create_storage_workflow,
        crate::api::handlers::storage::update_storage_workflow,
        crate::api::handlers::storage::delete_storage_workflow,
        crate::api::handlers::storage::import_storage_workflow,
        crate::api::handlers::storage::restore_storage_backup,
        crate::api::handlers::storage::export_storage_backup,
        crate::api::handlers::storage::get_workflow,
        crate::api::handlers::storage::save_workflow,
        crate::api::handlers::browsers::get_browsers,
        crate::api::handlers::browsers::get_browser_detail,
        crate::api::handlers::browsers::create_browser,
        crate::api::handlers::browsers::update_browser,
        crate::api::handlers::browsers::delete_browser,
        crate::api::handlers::browsers::start_browser,
        crate::api::handlers::browsers::stop_browser,
        crate::api::handlers::settings::get_settings,
        crate::api::handlers::settings::update_settings,
        crate::api::handlers::settings::patch_settings,
        crate::api::handlers::browsers::import_csv,
        crate::api::handlers::browsers::sideload_extension,
        crate::api::handlers::browsers::auto_detect_browsers,
        crate::api::handlers::browsers::set_default_browser,
        crate::api::handlers::ws::ws_handler
    ),
    info(
        title = "Tuquet CLI & Core Bridge API",
        version = "1.0.0",
        description = "Lightweight native process launcher, WebSocket event hub, and SQLite storage bridge for the Tuquet Ecosystem.",
        contact(
            name = "Tuquet Ecosystem",
            url = "https://github.com/tuquet/tuquet-cli"
        ),
        license(
            name = "MIT"
        )
    ),
    servers(
        (url = "http://127.0.0.1:8765", description = "Local Daemon Bridge Server")
    ),
    tags(
        (name = "Health", description = "System health check endpoints"),
        (name = "Jobs", description = "Workflow job dispatching and execution telemetry endpoints"),
        (name = "Browsers", description = "Chromium instance lifecycle and profile isolation endpoints"),
        (name = "History", description = "Execution audit history and log queries"),
        (name = "Storage", description = "Local SQLite variables, tables, and workflow storage endpoints"),
        (name = "System", description = "System metrics and developer studio launcher"),
        (name = "Lint", description = "Workflow static analysis endpoints"),
        (name = "Settings", description = "Core runtime configuration endpoints")
    )
)]
pub struct ApiDoc;

async fn root_handler() -> impl IntoResponse {
    axum::Json(serde_json::json!({
        "service": "tuquet-cli",
        "version": env!("CARGO_PKG_VERSION"),
        "role": "native-bridge-launcher",
        "status": "running",
        "docs": "/swagger-ui/",
        "api": "/api/v1"
    }))
}

pub fn create_router(state: crate::AppState) -> Router {
    let v1_router = Router::new()
        .route("/health", get(health))
        .route("/events", get(sse))
        .route("/system/browser-binaries", post(install_browser))
        .route("/system/studio/session", post(open_studio))
        .route("/system/metrics", get(get_metrics))
        .route("/system/info", get(get_system_info))
        .route("/system/cloud/status", get(get_cloud_status))
        .route("/system/cloud/sync", post(trigger_cloud_sync))
        .route("/system/cloud/login", post(cloud_login))
        .route("/system/cloud/logout", post(cloud_logout))
        .route("/system/settings", get(get_settings).put(update_settings).patch(patch_settings))
        .route("/lint", post(lint_workflow))
        .route("/jobs", post(submit_job).get(get_active_jobs))
        .route("/jobs/{job_id}", delete(kill_job))
        .route("/jobs/{job_id}/status", get(get_job_status).patch(job_finish))
        .route("/jobs/{job_id}/pause", post(pause_job))
        .route("/jobs/{job_id}/resume", post(resume_job))
        .route("/jobs/{job_id}/logs", post(job_log))
        .route("/internal/worker/events", get(worker_sse))
        .route("/history", get(get_history).delete(clear_history))
        .route("/history/{job_id}/logs", get(get_logs))
        .route("/history/{job_id}", delete(delete_history_item))
        .route("/storage/variables", get(get_variables).post(add_variable))
        .route("/storage/variables/{id}", delete(delete_variable))
        .route("/storage/credentials", get(get_credentials).post(add_credential))
        .route("/storage/credentials/{id}", delete(delete_credential))
        .route("/storage/tables", get(get_tables).post(add_table))
        .route("/storage/tables/{id}", delete(delete_table))
        .route("/storage/tables/{id}/rows", get(get_table_rows).post(add_table_row))
        .route("/storage/backup/restore", post(restore_storage_backup))
        .route("/storage/backup/export", get(export_storage_backup))
        .route("/storage/workflows", get(get_workflows).post(create_storage_workflow))
        .route("/storage/workflows/import", post(import_storage_workflow))
        .route("/storage/workflows/{id}", get(get_workflow_by_id).put(update_storage_workflow).delete(delete_storage_workflow))
        .route("/browsers", get(get_browsers).post(create_browser))
        .route("/browsers/auto-detect", post(auto_detect_browsers))
        .route("/browsers/sessions", delete(kill_browsers))
        .route("/browsers/{id}", get(get_browser_detail).put(update_browser).delete(delete_browser))
        .route("/browsers/{id}/session", post(start_browser).delete(stop_browser))
        .route("/browsers/{id}/set-default", post(set_default_browser))
        .route("/browsers/{id}/extensions", post(sideload_extension))
        .route("/browsers/import-csv", post(import_csv))
        .route("/storage/workflow", get(get_workflow).put(save_workflow))
        .route("/ws", get(ws_handler));

    Router::new()
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .route("/", get(root_handler))
        .nest("/api/v1", v1_router.clone())
        .nest("/api", v1_router)
        .with_state(state)
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any)
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::sync::{broadcast, Mutex};
    use crate::infrastructure::db::AutomaDb;
    use crate::AppState;

    fn create_test_state() -> AppState {
        let (tx, _) = broadcast::channel(10);
        let (worker_tx, _) = broadcast::channel(10);
        let db = AutomaDb::new_in_memory().unwrap();
        let config = Arc::new(crate::config::AppConfig::load());
        AppState {
            db: Arc::new(Mutex::new(db)),
            config,
            tx,
            worker_tx,
            active_jobs: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
        }
    }

    #[tokio::test]
    async fn test_root_service_info() {
        let state = create_test_state();
        let app = create_router(state);

        use tower::ServiceExt;
        use axum::http::{Request, StatusCode};

        let res_root = app.clone().oneshot(Request::builder().uri("/").body(axum::body::Body::empty()).unwrap()).await.unwrap();
        assert_eq!(res_root.status(), StatusCode::OK);
    }
}
