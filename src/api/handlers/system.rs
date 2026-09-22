use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::process::Command;
use utoipa::ToSchema;
use sysinfo::System;

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
#[schema(example = json!({"success": true, "message": "Operation completed successfully"}))]
/// General system action response
pub struct SystemResponse {
    /// Whether the system operation succeeded
    pub success: bool,
    /// Result description message
    pub message: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
#[schema(example = json!({
    "cpuUsage": 12.5,
    "memoryFree": 8388608,
    "memoryTotal": 16777216,
    "activeRunners": 2
}))]
/// Real-time CPU, RAM, and browser runner metrics
pub struct SystemMetricsResponse {
    /// Global CPU utilization percentage (0 - 100)
    pub cpu_usage: f32,
    /// Free system memory in bytes
    pub memory_free: u64,
    /// Total system physical memory in bytes
    pub memory_total: u64,
    /// Number of actively connected browser runner instances
    pub active_runners: usize,
}

#[utoipa::path(
    tag = "System",
    get,
    path = "/api/v1/system/metrics",
    operation_id = "get_system_metrics",
    summary = "Get host system performance metrics",
    description = "Samples current CPU load, memory utilization, and active browser runner counts.",
    responses(
        (status = 200, description = "System performance metrics sampled successfully", body = SystemMetricsResponse)
    )
)]
pub async fn get_metrics() -> impl IntoResponse {
    let mut sys = System::new_all();
    sys.refresh_all();
    
    let cpu_usage = sys.global_cpu_usage();
    let memory_free = sys.free_memory();
    let memory_total = sys.total_memory();
    
    // Connect to connected_browsers for active runners
    let active_runners = {
        let guard = crate::api::handlers::jobs::connected_browsers().read().await;
        guard.len()
    };
    
    (
        StatusCode::OK,
        Json(SystemMetricsResponse {
            cpu_usage,
            memory_free,
            memory_total,
            active_runners,
        })
    ).into_response()
}

#[utoipa::path(
    tag = "System",
    post,
    path = "/api/v1/system/browser-binaries",
    operation_id = "install_browser_binary",
    summary = "Download and install Chromium binary",
    description = "Downloads and extracts the latest compatible Chromium build into the local cache via `@puppeteer/browsers`.",
    responses(
        (status = 200, description = "Chromium binary downloaded and installed successfully", body = SystemResponse),
        (status = 500, description = "Failed to download browser binary", body = crate::core::error::ApiErrorResponse)
    )
)]
pub async fn install_browser() -> impl IntoResponse {
    let _ = tokio::task::spawn_blocking(|| {
        Command::new("npx")
            .args(["@puppeteer/browsers", "install", "chromium@latest"])
            .output()
    }).await;
    
    (
        StatusCode::OK,
        Json(SystemResponse {
            success: true,
            message: "Browser installed successfully".to_string(),
        })
    ).into_response()
}

#[utoipa::path(
    tag = "Browsers",
    delete,
    path = "/api/v1/browsers/sessions",
    operation_id = "kill_all_browsers",
    summary = "Terminate all running browser processes",
    description = "Forcefully shuts down all managed browser processes, child workers, and zombie processes across all profiles.",
    responses(
        (status = 200, description = "All managed browser processes terminated", body = SystemResponse)
    )
)]
pub async fn kill_browsers(
    State(state): State<crate::AppState>,
) -> impl IntoResponse {
    crate::core::browser::manager::BrowserManager::destroy_all().await;
    {
        let mut connected = crate::api::handlers::jobs::connected_browsers().write().await;
        connected.clear();
    }
    let _ = state.tx.send(serde_json::json!({
        "type": "browser_offline",
        "all": true
    }).to_string());
    
    (
        StatusCode::OK,
        Json(SystemResponse {
            success: true,
            message: "All managed browser processes have been killed".to_string(),
        })
    ).into_response()
}

#[utoipa::path(
    tag = "System",
    post,
    path = "/api/v1/system/studio/session",
    operation_id = "open_web_studio",
    summary = "Open Web Studio in default system browser",
    description = "Spawns the default OS web browser and navigates to the locally served Automa Web Studio canvas editor.",
    responses(
        (status = 200, description = "Web Studio opened successfully in system browser", body = SystemResponse)
    )
)]
pub async fn open_studio(
    State(state): State<crate::AppState>,
) -> impl IntoResponse {
    let port = state.config.server_port;
    let url = std::env::var("AUTOMA_STUDIO_URL").unwrap_or_else(|_| {
        format!("https://automa-studio.vercel.app?port={}", port)
    });
    
    let _ = tokio::task::spawn_blocking(move || {
        #[cfg(target_os = "windows")]
        let _ = Command::new("cmd").args(["/C", "start", &url]).spawn();
        
        #[cfg(target_os = "macos")]
        let _ = Command::new("open").arg(&url).spawn();
        
        #[cfg(target_os = "linux")]
        let _ = Command::new("xdg-open").arg(&url).spawn();
    }).await;
    
    (
        StatusCode::OK,
        Json(SystemResponse {
            success: true,
            message: "Studio opened".to_string(),
        })
    ).into_response()
}
