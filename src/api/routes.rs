use crate::api::handlers::browsers::{
    auto_detect_browsers, create_browser, delete_browser, get_browser_detail, get_browsers,
    import_csv, set_default_browser, sideload_extension, start_browser, stop_browser,
    update_browser,
};
use crate::api::handlers::health::health;
use crate::api::handlers::history::{clear_history, delete_history_item, get_history, get_logs};
use crate::api::handlers::jobs::{
    get_active_jobs, get_job_status, job_finish, job_log, kill_job, pause_job, resume_job,
    submit_job, worker_sse,
};
use crate::api::handlers::lint::lint_workflow;
use crate::api::handlers::secrets::{
    delete_secret, get_secrets, list_credentials, pull_secrets, push_secrets, rotate_secrets,
    set_secret,
};
use crate::api::handlers::settings::{get_settings, patch_settings, update_settings};
use crate::api::handlers::sse::sse;
use crate::api::handlers::system::{
    cloud_login, cloud_logout, desktop_auth_callback, desktop_auth_callback_post, get_cloud_status,
    get_metrics, get_system_info, get_tenants, kill_browsers, open_studio, start_desktop_auth,
    switch_tenant, trigger_cloud_sync,
};
use crate::api::handlers::ws::ws_handler;
use axum::{Router, response::IntoResponse, routing::delete, routing::get, routing::post};
use tower_http::cors::{Any, CorsLayer};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

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
        crate::api::handlers::system::kill_browsers,
        crate::api::handlers::system::open_studio,
        crate::api::handlers::system::get_metrics,
        crate::api::handlers::system::get_system_info,
        crate::api::handlers::system::get_cloud_status,
        crate::api::handlers::system::trigger_cloud_sync,
        crate::api::handlers::system::cloud_login,
        crate::api::handlers::system::cloud_logout,
        crate::api::handlers::system::get_tenants,
        crate::api::handlers::system::switch_tenant,
        crate::api::handlers::system::start_desktop_auth,
        crate::api::handlers::system::desktop_auth_callback,
        crate::api::handlers::system::desktop_auth_callback_post,
        crate::api::handlers::lint::lint_workflow,
        crate::api::handlers::secrets::get_secrets,
        crate::api::handlers::secrets::list_credentials,
        crate::api::handlers::secrets::set_secret,
        crate::api::handlers::secrets::delete_secret,
        crate::api::handlers::secrets::push_secrets,
        crate::api::handlers::secrets::pull_secrets,
        crate::api::handlers::secrets::rotate_secrets,
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
        title = "CLI & Core Gateway API",
        version = "1.0.0",
        description = "Embedded Backend for Frontend (BFF) Gateway, WebSocket/SSE event hub, and SQLite storage gateway for the Ecosystem.",
        contact(
            name = "Ecosystem",
            url = "https://github.com/tuquet/cli"
        ),
        license(
            name = "MIT"
        )
    ),
    servers(
        (url = "http://127.0.0.1:8765", description = "Local Daemon Gateway Server")
    ),
    components(
        schemas(
            crate::core::error::ApiErrorResponse,
            crate::api::handlers::health::HealthResponse,
            crate::api::handlers::system::TenantListResponse,
            crate::api::handlers::system::SwitchTenantRequest,
            crate::api::handlers::system::SwitchTenantResponse,
            crate::infrastructure::cloud::TenantInfo,
            crate::api::handlers::system::CloudStatusResponse,
            crate::api::handlers::system::CloudSyncResponse,
            crate::api::handlers::system::CloudLoginRequest,
            crate::api::handlers::system::CloudLoginResponse,
            crate::api::handlers::system::CloudLogoutResponse,
            crate::api::handlers::system::SystemMetricsResponse,
            crate::api::handlers::system::RunnerIdentityResponse,
            crate::api::handlers::system::SystemResponse,
            crate::api::handlers::browsers::BrowserResponse,
            crate::api::handlers::browsers::CreateBrowserRequest,
            crate::api::handlers::browsers::CreateBrowserResponse,
            crate::api::handlers::browsers::UpdateBrowserRequest,
            crate::api::handlers::browsers::UpdateBrowserResponse,
            crate::api::handlers::browsers::DeleteBrowserResponse,
            crate::api::handlers::browsers::StartBrowserSessionRequest,
            crate::api::handlers::browsers::StartBrowserSessionResponse,
            crate::api::handlers::browsers::StopBrowserResponse,
            crate::api::handlers::browsers::SetDefaultBrowserResponse,
            crate::api::handlers::browsers::ImportCsvPayload,
            crate::api::handlers::browsers::ImportCsvResponse,
            crate::api::handlers::browsers::SideloadExtensionPayload,
            crate::api::handlers::browsers::SideloadExtensionResponse,
            crate::api::handlers::jobs::SubmitJobPayload,
            crate::api::handlers::jobs::SubmitJobResponse,
            crate::api::handlers::jobs::ActiveJobResponse,
            crate::api::handlers::jobs::JobStatusResponse,
            crate::api::handlers::jobs::JobControlResponse,
            crate::api::handlers::jobs::JobLogPayload,
            crate::api::handlers::history::HistoryActionResponse,
            crate::api::handlers::lint::LintWorkflowRequest,
            crate::api::handlers::lint::LintWorkflowResponse,
            crate::api::handlers::lint::LintIssue,
            crate::api::handlers::secrets::SecretRow,
        crate::api::handlers::secrets::SecretStoreStatus,
        crate::api::handlers::secrets::CredentialList,
        crate::api::handlers::secrets::SetSecretRequest,
        crate::api::handlers::secrets::SecretOpResult,
            crate::core::models::settings::AppSettings,
            crate::core::models::settings::UpdateAppSettingsRequest
        )
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
        "service": "cli",
        "version": env!("CARGO_PKG_VERSION"),
        "role": "embedded-bff-gateway",
        "status": "running",
        "docs": "/swagger-ui/",
        "api": "/api/v1"
    }))
}

pub fn create_router(state: crate::AppState) -> Router {
    let v1_router = Router::new()
        .route("/health", get(health))
        .route("/events", get(sse))
        .route("/system/studio/session", post(open_studio))
        .route("/system/metrics", get(get_metrics))
        .route("/system/info", get(get_system_info))
        .route("/system/cloud/status", get(get_cloud_status))
        .route("/system/cloud/sync", post(trigger_cloud_sync))
        .route("/system/cloud/login", post(cloud_login))
        .route("/system/cloud/logout", post(cloud_logout))
        .route("/system/cloud/auth/start", post(start_desktop_auth))
        .route(
            "/auth/callback",
            get(desktop_auth_callback).post(desktop_auth_callback_post),
        )
        .route("/system/tenants", get(get_tenants))
        .route("/system/tenants/switch", post(switch_tenant))
        .route(
            "/system/settings",
            get(get_settings).put(update_settings).patch(patch_settings),
        )
        // Secret Store — paths mirror the `secrets` schema / table names (SPEC-core-route-foundation §5.1)
        .route("/secrets", get(get_secrets))
        .route(
            "/secrets/credentials",
            get(list_credentials).post(set_secret),
        )
        .route("/secrets/credentials/{name}", delete(delete_secret))
        .route("/secrets/credentials/push", post(push_secrets))
        .route("/secrets/credentials/pull", post(pull_secrets))
        .route("/secrets/tenant-keys/rotate", post(rotate_secrets))
        .route("/lint", post(lint_workflow))
        .route("/jobs", post(submit_job).get(get_active_jobs))
        .route("/jobs/{job_id}", delete(kill_job))
        .route(
            "/jobs/{job_id}/status",
            get(get_job_status).patch(job_finish),
        )
        .route("/jobs/{job_id}/pause", post(pause_job))
        .route("/jobs/{job_id}/resume", post(resume_job))
        .route("/jobs/{job_id}/logs", post(job_log))
        .route("/internal/worker/events", get(worker_sse))
        .route("/history", get(get_history).delete(clear_history))
        .route("/history/{job_id}/logs", get(get_logs))
        .route("/history/{job_id}", delete(delete_history_item))
        .route("/browsers", get(get_browsers).post(create_browser))
        .route("/browsers/auto-detect", post(auto_detect_browsers))
        .route("/browsers/sessions", delete(kill_browsers))
        .route(
            "/browsers/{id}",
            get(get_browser_detail)
                .put(update_browser)
                .delete(delete_browser),
        )
        .route(
            "/browsers/{id}/session",
            post(start_browser).delete(stop_browser),
        )
        .route("/browsers/{id}/set-default", post(set_default_browser))
        .route("/browsers/{id}/extensions", post(sideload_extension))
        .route("/browsers/import-csv", post(import_csv))
        .route("/ws", get(ws_handler));

    let cors = CorsLayer::new()
        .allow_origin(tower_http::cors::AllowOrigin::predicate(
            |origin: &axum::http::HeaderValue, _parts: &axum::http::request::Parts| {
                if let Ok(s) = origin.to_str() {
                    let s_trimmed = s.trim_end_matches('/');
                    let host_with_port = s_trimmed
                        .strip_prefix("https://")
                        .or_else(|| s_trimmed.strip_prefix("http://"))
                        .unwrap_or(s_trimmed);
                    let host = host_with_port.split(':').next().unwrap_or(host_with_port);

                    s_trimmed.starts_with("chrome-extension://")
                        || s_trimmed.starts_with("moz-extension://")
                        || s_trimmed.starts_with("tauri://")
                        || host == "localhost"
                        || host == "127.0.0.1"
                        || host == "tauri.localhost"
                        || host == "asset.localhost"
                        || host == "tuquet.tq.com"
                        || host == "tuquet.com"
                        || host.ends_with(".tq.com")
                        || s_trimmed == "https://automation-studio.vercel.app"
                        || s_trimmed == "https://tuquet.github.io"
                } else {
                    false
                }
            },
        ))
        .allow_methods(Any)
        .allow_headers(Any)
        .allow_private_network(true);

    Router::new()
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .route("/", get(root_handler))
        .nest("/api/v1", v1_router.clone())
        .nest("/api", v1_router)
        .with_state(state)
        .layer(cors)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AppState;
    use crate::infrastructure::db::AutomationDb;
    use std::sync::Arc;
    use tokio::sync::{Mutex, broadcast};

    fn create_test_state() -> AppState {
        let (tx, _) = broadcast::channel(10);
        let (worker_tx, _) = broadcast::channel(10);
        let db = AutomationDb::new_in_memory().unwrap();
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

        use axum::http::{Request, StatusCode};
        use tower::ServiceExt;

        let res_root = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res_root.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_cors_origins() {
        let state = create_test_state();
        let app = create_router(state);

        use axum::http::{Request, StatusCode};
        use tower::ServiceExt;

        let valid_origins = [
            "tauri://localhost",
            "http://tauri.localhost",
            "https://tauri.localhost",
            "http://asset.localhost",
            "https://tuquet.tq.com",
            "http://tuquet.tq.com",
            "https://app.tq.com",
            "https://admin.tq.com",
            "http://admin.tq.com",
            "http://localhost:5173",
            "http://127.0.0.1:8765",
            "chrome-extension://abcdefghijklmnop",
        ];

        for origin in valid_origins {
            let req = Request::builder()
                .method("OPTIONS")
                .uri("/api/v1/health")
                .header("Origin", origin)
                .header("Access-Control-Request-Method", "GET")
                .body(axum::body::Body::empty())
                .unwrap();

            let res = app.clone().oneshot(req).await.unwrap();
            assert_eq!(
                res.status(),
                StatusCode::OK,
                "Failed for origin: {}",
                origin
            );
            let allow_origin = res.headers().get("access-control-allow-origin");
            assert!(
                allow_origin.is_some(),
                "CORS header missing for origin: {}",
                origin
            );
        }
    }

    #[tokio::test]
    async fn test_get_system_tenants() {
        let state = create_test_state();
        let app = create_router(state);

        use axum::http::{Request, StatusCode};
        use tower::ServiceExt;

        let req = Request::builder()
            .method("GET")
            .uri("/api/v1/system/tenants")
            .body(axum::body::Body::empty())
            .unwrap();

        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }

    #[test]
    fn test_openapi_spec_generation() {
        let openapi = ApiDoc::openapi();
        let json_result = openapi.to_pretty_json();
        assert!(
            json_result.is_ok(),
            "OpenAPI spec serialization failed: {:?}",
            json_result.err()
        );
        let json_str = json_result.unwrap();
        assert!(json_str.contains("CLI & Core Gateway API"));
        assert!(json_str.contains("/api/v1/system/tenants"));
        assert!(json_str.contains("/api/v1/system/cloud/auth/start"));
        assert!(json_str.contains("/api/v1/auth/callback"));
        assert!(json_str.contains("/api/v1/browsers"));
    }
}
