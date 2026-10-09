use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use sysinfo::System;

use super::types::{MachineInfo, RunnerIdentityResponse, SystemMetricsResponse};

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
    let mut sys = System::new();
    sys.refresh_cpu_all();
    sys.refresh_memory();
    
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
    get,
    path = "/api/v1/system/info",
    operation_id = "get_system_info",
    summary = "Get runner machine identity and system capabilities",
    description = "Returns host machine identity (hostname, OS, architecture), runner service protocol, and runtime capabilities for Runner orchestration.",
    responses(
        (status = 200, description = "Runner machine identity information", body = RunnerIdentityResponse)
    )
)]
pub async fn get_system_info(
    State(state): State<crate::AppState>,
) -> impl IntoResponse {
    let mut sys = System::new();
    sys.refresh_memory();

    let hostname = std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "unknown".to_string());
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
            service: "specter-cli".to_string(),
            protocol: "specter.automa.v1".to_string(),
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
