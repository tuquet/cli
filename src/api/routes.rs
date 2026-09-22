use tower_http::cors::{Any, CorsLayer};
use axum::{routing::get, routing::post, routing::delete, Router};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;
use crate::api::handlers::secrets::encrypt_secret;
use crate::api::handlers::history::{
    get_history, get_logs, delete_history_item, clear_history
};
use crate::api::handlers::sse::sse;
use crate::api::handlers::health::health;
use crate::api::handlers::jobs::{
    submit_job, get_job_status, get_active_jobs, kill_job,
    worker_sse, job_log, job_finish
};
use crate::api::handlers::system::{
    install_browser, open_studio, kill_browsers, get_metrics
};
use crate::api::handlers::lint::lint_workflow;
use crate::api::handlers::storage::{
    get_variables, add_variable, delete_variable,
    get_credentials, add_credential, delete_credential,
    get_tables, add_table, delete_table,
    get_table_rows, add_table_row,
    get_workflows, get_workflow_by_id,
    create_storage_workflow, update_storage_workflow, delete_storage_workflow,
    import_storage_workflow, restore_storage_backup, export_storage_backup
};
use crate::api::handlers::browsers::{
    get_browsers, create_browser, update_browser, delete_browser, get_browser_detail, start_browser, stop_browser, get_cookies, post_cookies, import_csv, sideload_extension, auto_detect_browsers
};
use crate::api::handlers::campaigns::{
    get_matrix_status, execute_campaign, abort_campaign,
    get_storage_campaigns, get_storage_campaign,
    create_storage_campaign, update_storage_campaign, delete_storage_campaign,
    import_storage_campaign
};
use crate::api::handlers::vault::{get_workflow, save_workflow};
use crate::api::handlers::settings::{get_settings, update_settings, patch_settings};
use crate::api::handlers::ws::ws_handler;

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::api::handlers::jobs::submit_job,
        crate::api::handlers::jobs::get_job_status,
        crate::api::handlers::jobs::get_active_jobs,
        crate::api::handlers::jobs::kill_job,
        crate::api::handlers::jobs::worker_sse,
        crate::api::handlers::jobs::job_log,
        crate::api::handlers::jobs::job_finish,
        crate::api::handlers::history::get_history,
        crate::api::handlers::history::get_logs,
        crate::api::handlers::history::delete_history_item,
        crate::api::handlers::history::clear_history,
        crate::api::handlers::health::health,
        crate::api::handlers::secrets::encrypt_secret,
        crate::api::handlers::sse::sse,
        crate::api::handlers::system::install_browser,
        crate::api::handlers::system::kill_browsers,
        crate::api::handlers::system::open_studio,
        crate::api::handlers::system::get_metrics,
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
        crate::api::handlers::campaigns::get_storage_campaigns,
        crate::api::handlers::campaigns::get_storage_campaign,
        crate::api::handlers::campaigns::create_storage_campaign,
        crate::api::handlers::campaigns::update_storage_campaign,
        crate::api::handlers::campaigns::delete_storage_campaign,
        crate::api::handlers::campaigns::import_storage_campaign,
        crate::api::handlers::browsers::get_browsers,
        crate::api::handlers::browsers::get_browser_detail,
        crate::api::handlers::browsers::create_browser,
        crate::api::handlers::browsers::update_browser,
        crate::api::handlers::browsers::delete_browser,
        crate::api::handlers::browsers::start_browser,
        crate::api::handlers::browsers::stop_browser,
        crate::api::handlers::browsers::get_cookies,
        crate::api::handlers::browsers::post_cookies,
        crate::api::handlers::settings::get_settings,
        crate::api::handlers::settings::update_settings,
        crate::api::handlers::settings::patch_settings,
        crate::api::handlers::campaigns::get_matrix_status,
        crate::api::handlers::campaigns::execute_campaign,
        crate::api::handlers::campaigns::abort_campaign,
        crate::api::handlers::browsers::import_csv,
        crate::api::handlers::browsers::sideload_extension,
        crate::api::handlers::browsers::auto_detect_browsers,
        crate::api::handlers::vault::get_workflow,
        crate::api::handlers::vault::save_workflow,
        crate::api::handlers::ws::ws_handler
    ),
    info(
        title = "Automa Core API",
        version = "1.0.0",
        description = "High-performance Rust backend daemon providing REST, SSE, and storage APIs for the Automa automation ecosystem.",
        contact(
            name = "Automa Ecosystem Team",
            url = "https://github.com/tuquet/tuquet-automa"
        ),
        license(
            name = "MIT"
        )
    ),
    components(
        schemas(
            crate::core::error::ApiErrorResponse,
            crate::api::handlers::jobs::SubmitJobPayload,
            crate::api::handlers::jobs::SubmitJobOptions,
            crate::api::handlers::jobs::SubmitJobResponse,
            crate::api::handlers::jobs::JobStatusResponse,
            crate::api::handlers::jobs::ActiveJobResponse,
            crate::infrastructure::db::JobHistoryItem,
            crate::infrastructure::db::JobInfo,
            crate::infrastructure::db::JobDetails,
            crate::infrastructure::db::LogItem,
            crate::api::handlers::health::HealthResponse,
            crate::api::handlers::secrets::EncryptSecretRequest,
            crate::api::handlers::secrets::EncryptSecretResponse,
            crate::api::handlers::system::SystemResponse,
            crate::api::handlers::system::SystemMetricsResponse,
            crate::api::handlers::lint::LintRequest,
            crate::api::handlers::lint::LintIssue,
            crate::api::handlers::lint::LintResponse,
            crate::api::handlers::storage::StorageVariable,
            crate::api::handlers::storage::StorageCredential,
            crate::api::handlers::storage::StorageTable,
            crate::api::handlers::storage::TableRow,
            crate::api::handlers::storage::AddTableRowPayload,
            crate::api::handlers::storage::AddTableRowResponse,
            crate::api::handlers::storage::RestoreBackupRequest,
            crate::api::handlers::storage::RestoreBackupResponse,
            crate::api::handlers::storage::ExportBackupResponse,
            crate::infrastructure::db::Browser,
            crate::api::handlers::browsers::CreateBrowserRequest,
            crate::api::handlers::browsers::UpdateBrowserRequest,
            crate::api::handlers::browsers::BrowserResponse,
            crate::api::handlers::browsers::ImportCsvPayload,
            crate::api::handlers::browsers::ImportCsvResponse,
            crate::api::handlers::browsers::SideloadExtensionPayload,
            crate::api::handlers::browsers::SideloadExtensionResponse,
            crate::api::handlers::cookie_manager::Cookie,
            crate::api::handlers::campaigns::MatrixSlotInfo,
            crate::api::handlers::campaigns::MatrixStatusResponse,
            crate::api::handlers::campaigns::ExecuteCampaignRequest,
            crate::api::handlers::campaigns::ExecuteCampaignResponse,
            crate::core::models::settings::AppSettings,
            crate::core::models::settings::GridSettings,
            crate::core::models::settings::GridMatrix,
            crate::core::models::settings::DisplaySettings,
            crate::core::models::settings::GridBehavior,
            crate::core::models::settings::BrowserSettings,
            crate::core::models::settings::RunnerSettings,
            crate::core::models::settings::UpdateAppSettingsRequest,
            crate::core::models::settings::UpdateGridSettingsRequest,
            crate::core::models::settings::UpdateGridMatrixRequest,
            crate::core::models::settings::UpdateDisplaySettingsRequest,
            crate::core::models::settings::UpdateGridBehaviorRequest,
            crate::core::models::settings::UpdateBrowserSettingsRequest,
            crate::core::models::settings::UpdateRunnerSettingsRequest,
            crate::api::handlers::jobs::JobLogPayload,
            crate::api::handlers::history::HistoryActionResponse,
            crate::api::handlers::vault::SaveWorkflowPayload,
            crate::api::handlers::vault::SaveWorkflowResponse
        )
    ),
    tags(
        (name = "Jobs", description = "Workflow job execution, lifecycle tracking, real-time logging, and worker control signals."),
        (name = "Storage", description = "Workspace file exploration, workflow definitions, global variables, credentials, and data tables."),
        (name = "Browsers", description = "Isolated browser profile management, process lifecycles, cookie stores, and extension sideloading."),
        (name = "Campaigns", description = "Multi-instance grid matrix orchestration and batch workflow campaigns."),
        (name = "System", description = "Host system diagnostics, CPU/RAM telemetry, browser binary installers, and Studio web editor."),
        (name = "History", description = "Job execution history, step audit trails, and log persistence."),
        (name = "Settings", description = "Application preferences, grid layout configuration, browser defaults, and concurrency limits."),
        (name = "Secrets", description = "Cryptographic security and AES-256 authenticated secret encryption."),
        (name = "Lint", description = "Static analysis and structural validation for Automa workflow AST graphs."),
        (name = "Events", description = "Global Server-Sent Events stream for real-time task notifications.")
    )
)]
pub struct ApiDoc;

use axum::response::IntoResponse;

async fn root_handler() -> impl IntoResponse {
    axum::Json(serde_json::json!({
        "service": "automa-core",
        "version": env!("CARGO_PKG_VERSION"),
        "status": "running",
        "docs": "/swagger-ui/",
        "api": "/api/v1"
    }))
}

pub fn create_router(state: crate::AppState) -> Router {
    let v1_router = Router::new()
        .route("/health", get(health))
        .route("/events", get(sse))
        .route("/secrets/encrypt", post(encrypt_secret))
        .route("/system/browser-binaries", post(install_browser))
        .route("/system/studio/session", post(open_studio))
        .route("/system/metrics", get(get_metrics))
        .route("/system/settings", get(get_settings).put(update_settings).patch(patch_settings))
        .route("/lint", post(lint_workflow))
        .route("/jobs", post(submit_job).get(get_active_jobs))
        .route("/jobs/{job_id}", delete(kill_job))
        .route("/jobs/{job_id}/status", get(get_job_status).patch(job_finish))
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
        .route("/storage/campaigns", get(get_storage_campaigns).post(create_storage_campaign))
        .route("/storage/campaigns/import", post(import_storage_campaign))
        .route("/storage/campaigns/{id}", get(get_storage_campaign).put(update_storage_campaign).delete(delete_storage_campaign))
        .route("/browsers", get(get_browsers).post(create_browser))
        .route("/browsers/auto-detect", post(auto_detect_browsers))
        .route("/browsers/sessions", delete(kill_browsers))
        .route("/browsers/{id}", get(get_browser_detail).put(update_browser).delete(delete_browser))
        .route("/browsers/{id}/session", post(start_browser).delete(stop_browser))
        .route("/browsers/{id}/cookies", get(get_cookies).post(post_cookies))
        .route("/browsers/{id}/extensions", post(sideload_extension))
        .route("/browsers/import-csv", post(import_csv))
        .route("/campaigns/execute", post(execute_campaign))
        .route("/campaigns/{id}", delete(abort_campaign))
        .route("/campaigns/{id}/matrix-status", get(get_matrix_status))
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
