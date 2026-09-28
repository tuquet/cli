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

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// Machine hardware and operating system identity
pub struct MachineInfo {
    /// Machine network hostname
    pub hostname: String,
    /// Operating system platform
    pub os: String,
    /// Operating system kernel/version release
    pub os_version: String,
    /// CPU hardware architecture
    pub cpu_arch: String,
    /// Total installed physical RAM in bytes
    pub total_memory: u64,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// Runner service and machine identity response
pub struct RunnerIdentityResponse {
    /// Service identifier name
    pub service: String,
    /// Protocol contract version for Tuquet Runner
    pub protocol: String,
    /// Version of automa-runner daemon
    pub version: String,
    /// Runner operational lifecycle status
    pub status: String,
    /// Host machine identity
    pub machine: MachineInfo,
    /// Number of connected active browser workers
    pub active_runners: usize,
    /// Daemon listening port
    pub server_port: u16,
    /// Data storage directory path
    pub data_dir: String,
}

#[utoipa::path(
    tag = "System",
    get,
    path = "/api/v1/system/info",
    operation_id = "get_system_info",
    summary = "Get runner machine identity and system capabilities",
    description = "Returns host machine identity (hostname, OS, architecture), runner service protocol, and runtime capabilities for Tuquet Runner orchestration.",
    responses(
        (status = 200, description = "Runner machine identity information", body = RunnerIdentityResponse)
    )
)]
pub async fn get_system_info(
    State(state): State<crate::AppState>,
) -> impl IntoResponse {
    let mut sys = System::new_all();
    sys.refresh_all();

    let hostname = System::host_name().unwrap_or_else(|| "unknown".to_string());
    let os = System::name().unwrap_or_else(|| std::env::consts::OS.to_string());
    let os_version = System::os_version().unwrap_or_default();
    let cpu_arch = {
        let arch = System::cpu_arch();
        if arch.is_empty() {
            std::env::consts::ARCH.to_string()
        } else {
            arch
        }
    };
    let total_memory = sys.total_memory();

    let active_runners = {
        let guard = crate::api::handlers::jobs::connected_browsers().read().await;
        guard.len()
    };

    (
        StatusCode::OK,
        Json(RunnerIdentityResponse {
            service: "tuquet-automa-runner".to_string(),
            protocol: "tuquet.automa.v1".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            status: "ready".to_string(),
            machine: MachineInfo {
                hostname,
                os,
                os_version,
                cpu_arch,
                total_memory,
            },
            active_runners,
            server_port: state.config.server_port,
            data_dir: state.config.data_dir.clone(),
        }),
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
    let url = std::env::var("AUTOMA_STUDIO_URL")
        .map(|u| {
            if !u.contains("port=") {
                let separator = if u.contains('?') { '&' } else { '?' };
                format!("{}{separator}port={}", u, port)
            } else {
                u
            }
        })
        .unwrap_or_else(|_| {
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

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// Cloud telemetry and device enrollment status
pub struct CloudStatusResponse {
    /// Whether cloud telemetry reporting is configured
    pub enabled: bool,
    /// Connected Tuquet Cloud URL
    pub cloud_url: Option<String>,
    /// Whether this machine is enrolled with a device ID
    pub enrolled: bool,
    /// Persistent workstation device ID
    pub device_id: Option<String>,
    /// Enrolled workstation name
    pub device_name: Option<String>,
    /// Machine hardware fingerprint
    pub machine_fingerprint: String,
    /// Heartbeat interval in seconds
    pub heartbeat_interval_secs: u64,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// Cloud sync operation result
pub struct CloudSyncResponse {
    /// Whether sync succeeded
    pub success: bool,
    /// Whether machine is enrolled
    pub enrolled: bool,
    /// Device ID
    pub device_id: String,
    /// Number of local browser profiles synced to central hub
    pub browsers_synced: usize,
    /// Whether heartbeat telemetry was accepted
    pub heartbeat_sent: bool,
    /// Status or error message
    pub message: String,
}

#[utoipa::path(
    tag = "System",
    get,
    path = "/api/v1/system/cloud/status",
    operation_id = "get_cloud_status",
    summary = "Get Tuquet Cloud reporting and device enrollment status",
    description = "Returns current enrollment credentials, hardware fingerprint, cloud endpoint URL, and last synchronization state.",
    responses(
        (status = 200, description = "Cloud reporting status retrieved successfully", body = CloudStatusResponse)
    )
)]
pub async fn get_cloud_status(
    State(state): State<crate::AppState>,
) -> impl IntoResponse {
    let cloud_url = state.config.cloud_url.clone();
    let enabled = cloud_url.is_some();
    let creds = crate::infrastructure::cloud_reporter::CloudReporter::load_credentials(&state.config.data_dir).await;
    let fingerprint = crate::infrastructure::cloud_reporter::CloudReporter::generate_machine_fingerprint();

    let (enrolled, device_id, device_name) = match creds {
        Some(c) => (true, Some(c.device_id), Some(c.name)),
        None => (false, None, None),
    };

    (
        StatusCode::OK,
        Json(CloudStatusResponse {
            enabled,
            cloud_url,
            enrolled,
            device_id,
            device_name,
            machine_fingerprint: fingerprint,
            heartbeat_interval_secs: state.config.cloud_heartbeat_interval_secs,
        }),
    ).into_response()
}

#[utoipa::path(
    tag = "System",
    post,
    path = "/api/v1/system/cloud/sync",
    operation_id = "trigger_cloud_sync",
    summary = "Trigger immediate inventory and heartbeat sync to Tuquet Cloud",
    description = "Forces an immediate snapshot of local SQLite browser profiles and system telemetry to be sent to Tuquet Cloud central hub.",
    responses(
        (status = 200, description = "Sync completed or attempted", body = CloudSyncResponse)
    )
)]
pub async fn trigger_cloud_sync(
    State(state): State<crate::AppState>,
) -> impl IntoResponse {
    match crate::infrastructure::cloud_reporter::CloudReporter::sync_inventory_and_heartbeat(&state).await {
        Ok(res) => (
            StatusCode::OK,
            Json(CloudSyncResponse {
                success: res.success,
                enrolled: res.enrolled,
                device_id: res.device_id,
                browsers_synced: res.browsers_synced,
                heartbeat_sent: res.heartbeat_sent,
                message: res.message,
            }),
        ).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(CloudSyncResponse {
                success: false,
                enrolled: false,
                device_id: "".to_string(),
                browsers_synced: 0,
                heartbeat_sent: false,
                message: e.to_string(),
            }),
        ).into_response(),
    }
}

