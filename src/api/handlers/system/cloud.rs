use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Json,
};

use super::types::{
    CloudLoginRequest, CloudLoginResponse, CloudLogoutResponse, CloudStatusResponse,
    CloudSyncResponse,
};

#[utoipa::path(
    tag = "System",
    get,
    path = "/api/v1/system/cloud/status",
    operation_id = "get_cloud_status",
    summary = "Get Specter Cloud reporting and device enrollment status",
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
    summary = "Trigger immediate inventory and heartbeat sync to Specter Cloud",
    description = "Forces an immediate snapshot of local SQLite browser profiles and system telemetry to be sent to Specter Cloud central hub.",
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

#[utoipa::path(
    tag = "System",
    post,
    path = "/api/v1/system/cloud/login",
    operation_id = "cloud_login",
    summary = "Enroll and authenticate this runner with Specter Cloud",
    description = "Authenticates this workstation runner with Specter Cloud using an enrollment token or JWT and stores credentials locally.",
    request_body = CloudLoginRequest,
    responses(
        (status = 200, description = "Successfully enrolled workstation", body = CloudLoginResponse),
        (status = 400, description = "Enrollment failed or invalid parameters", body = CloudLoginResponse)
    )
)]
pub async fn cloud_login(
    State(state): State<crate::AppState>,
    Json(payload): Json<CloudLoginRequest>,
) -> impl IntoResponse {
    let cloud_url = payload.cloud_url
        .or_else(|| state.config.cloud_url.clone())
        .unwrap_or_else(|| "http://127.0.0.1:54321".to_string());
    
    let token = payload.token.as_deref().or(state.config.cloud_enrollment_token.as_deref());
    let custom_name = payload.name.as_deref();

    match crate::infrastructure::cloud_reporter::CloudReporter::login(
        &cloud_url,
        token,
        custom_name,
        &state.config.data_dir,
    ).await {
        Ok(creds) => (
            StatusCode::OK,
            Json(CloudLoginResponse {
                success: true,
                enrolled: true,
                device_id: creds.device_id,
                tenant_id: creds.tenant_id,
                name: creds.name,
                message: "Workstation enrolled successfully with Specter Cloud".to_string(),
            }),
        ).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(CloudLoginResponse {
                success: false,
                enrolled: false,
                device_id: "".to_string(),
                tenant_id: None,
                name: "".to_string(),
                message: format!("Enrollment failed: {}", e),
            }),
        ).into_response(),
    }
}

#[utoipa::path(
    tag = "System",
    post,
    path = "/api/v1/system/cloud/logout",
    operation_id = "cloud_logout",
    summary = "Disconnect and unenroll this runner from Specter Cloud",
    description = "Removes locally stored device credentials, disconnecting this workstation from Specter Cloud.",
    responses(
        (status = 200, description = "Successfully logged out", body = CloudLogoutResponse)
    )
)]
pub async fn cloud_logout(
    State(state): State<crate::AppState>,
) -> impl IntoResponse {
    match crate::infrastructure::cloud_reporter::CloudReporter::logout(&state.config.data_dir).await {
        Ok(_) => (
            StatusCode::OK,
            Json(CloudLogoutResponse {
                success: true,
                message: "Successfully logged out and disconnected from Specter Cloud".to_string(),
            }),
        ).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(CloudLogoutResponse {
                success: false,
                message: format!("Failed to remove credentials: {}", e),
            }),
        ).into_response(),
    }
}
