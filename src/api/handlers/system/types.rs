use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

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
    /// Protocol contract version for Runner
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

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// Cloud telemetry and device enrollment status
pub struct CloudStatusResponse {
    /// Whether cloud telemetry reporting is configured
    pub enabled: bool,
    /// Connected Specter Cloud URL
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

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// Request payload to enroll and authenticate runner with Specter Cloud
pub struct CloudLoginRequest {
    /// Specter Cloud Base URL (e.g. "http://127.0.0.1:54321")
    pub cloud_url: Option<String>,
    /// Organization / Tenant enrollment token or user JWT
    pub token: Option<String>,
    /// Optional friendly device name
    pub name: Option<String>,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// Response returned after attempting cloud enrollment
pub struct CloudLoginResponse {
    /// Whether enrollment succeeded
    pub success: bool,
    /// Whether machine is enrolled
    pub enrolled: bool,
    /// Registered device ID
    pub device_id: String,
    /// Tenant or organization ID
    pub tenant_id: Option<String>,
    /// Device name
    pub name: String,
    /// Status or error message
    pub message: String,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// Response returned after logging out from cloud
pub struct CloudLogoutResponse {
    /// Whether logout succeeded
    pub success: bool,
    /// Confirmation message
    pub message: String,
}
