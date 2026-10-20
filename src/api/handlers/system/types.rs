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
    /// Authenticated operator email
    pub user_email: Option<String>,
    /// Active tenant workspace ID
    pub tenant_id: Option<String>,
    /// Active tenant workspace vanity slug
    pub tenant_slug: Option<String>,
    /// Active tenant workspace display name
    pub tenant_name: Option<String>,
    /// Operator role within active workspace
    pub tenant_role: Option<String>,
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
/// Request payload to enroll and authenticate runner with Tuquet Cloud
pub struct CloudLoginRequest {
    /// Tuquet Cloud Base URL (e.g. "https://dswhacsoaxgpfnkaxnhz.supabase.co")
    pub cloud_url: Option<String>,
    /// Organization / Tenant enrollment token or user JWT
    pub token: Option<String>,
    /// Optional friendly device name
    pub name: Option<String>,
    /// Operator email for Supabase GoTrue authentication
    pub email: Option<String>,
    /// Operator password
    pub password: Option<String>,
    /// Request One-Time Passcode (OTP)
    pub otp: Option<bool>,
    /// Pre-supplied 6-digit OTP code
    pub code: Option<String>,
    /// Pre-select target workspace slug or ID
    pub tenant: Option<String>,
    /// Optional custom Supabase API key
    pub api_key: Option<String>,
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
    /// Active tenant slug
    pub tenant_slug: Option<String>,
    /// Active tenant name
    pub tenant_name: Option<String>,
    /// Operator role in active tenant
    pub tenant_role: Option<String>,
    /// Operator email
    pub user_email: Option<String>,
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

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
#[schema(example = json!({
    "authUrl": "https://admin.tuquet.com/auth/desktop?state=xyz&port=8765&machine_name=DESKTOP-1",
    "state": "xyz",
    "port": 8765
}))]
/// Response descriptor containing Desktop SSO authorization URL
pub struct DesktopAuthStartResponse {
    /// Generated authorization URL on Tuquet Admin
    pub auth_url: String,
    /// CSRF state token
    pub state: String,
    /// Local loopback port
    pub port: u16,
}

#[derive(Serialize, Deserialize, utoipa::IntoParams, ToSchema)]
#[serde(rename_all = "camelCase")]
/// Parameters received by loopback auth callback (via Query or JSON body)
pub struct DesktopAuthCallbackParams {
    /// Delegation access token or JWT
    pub token: Option<String>,
    /// Long-lived refresh token for silent session renewal
    pub refresh_token: Option<String>,
    /// CSRF state token
    pub state: Option<String>,
    /// Selected Tenant / Workspace ID
    pub tenant_id: Option<String>,
    /// Machine hostname
    pub machine_name: Option<String>,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// Response returned after processing loopback auth callback
pub struct DesktopAuthCallbackResponse {
    /// Whether pairing succeeded
    pub success: bool,
    /// Confirmation message
    pub message: String,
    /// Authenticated user email
    pub user_email: Option<String>,
    /// Active tenant identifier
    pub tenant_id: Option<String>,
}
