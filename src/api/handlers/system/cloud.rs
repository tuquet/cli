use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Json,
};

use std::sync::LazyLock;
use std::sync::Mutex;
use std::collections::HashMap;
use std::time::{Duration, Instant};

static PENDING_AUTH_NONCES: LazyLock<Mutex<HashMap<String, Instant>>> = LazyLock::new(|| {
    Mutex::new(HashMap::new())
});

use super::types::{
    CloudLoginRequest, CloudLoginResponse, CloudLogoutResponse, CloudStatusResponse,
    CloudSyncResponse, DesktopAuthCallbackParams, DesktopAuthCallbackResponse,
    DesktopAuthStartResponse,
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

    let (enrolled, device_id, device_name, user_email, tenant_id, tenant_slug, tenant_name, tenant_role) = match creds {
        Some(c) => (
            true,
            Some(c.device_id),
            Some(c.name),
            c.email,
            c.tenant_id,
            c.tenant_slug,
            c.tenant_name,
            c.tenant_role,
        ),
        None => (false, None, None, None, None, None, None, None),
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
            user_email,
            tenant_id,
            tenant_slug,
            tenant_name,
            tenant_role,
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
        .or_else(|| state.config.cloud_url.clone());
    
    let is_otp = payload.otp.unwrap_or(false);

    match crate::commands::cloud::login(
        cloud_url,
        payload.email,
        payload.password,
        is_otp,
        payload.code,
        payload.token,
        payload.tenant,
        payload.name,
        payload.api_key,
        crate::ui::OutputFormat::Json,
    ).await {
        Ok(()) => {
            let creds = crate::infrastructure::cloud_reporter::CloudReporter::load_credentials(&state.config.data_dir).await;
            if let Some(c) = creds {
                (
                    StatusCode::OK,
                    Json(CloudLoginResponse {
                        success: true,
                        enrolled: true,
                        device_id: c.device_id,
                        tenant_id: c.tenant_id,
                        tenant_slug: c.tenant_slug,
                        tenant_name: c.tenant_name,
                        tenant_role: c.tenant_role,
                        user_email: c.email,
                        name: c.name,
                        message: "Workstation authenticated and paired successfully with Specter Cloud".to_string(),
                    }),
                ).into_response()
            } else {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(CloudLoginResponse {
                        success: false,
                        enrolled: false,
                        device_id: "".to_string(),
                        tenant_id: None,
                        tenant_slug: None,
                        tenant_name: None,
                        tenant_role: None,
                        user_email: None,
                        name: "".to_string(),
                        message: "Enrollment succeeded but failed to reload credentials".to_string(),
                    }),
                ).into_response()
            }
        }
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(CloudLoginResponse {
                success: false,
                enrolled: false,
                device_id: "".to_string(),
                tenant_id: None,
                tenant_slug: None,
                tenant_name: None,
                tenant_role: None,
                user_email: None,
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
        Ok(_) => {
            let _ = state.tx.send(serde_json::json!({
                "type": "auth_state_changed",
                "isAuthenticated": false,
                "message": "Logged out from Specter Cloud"
            }).to_string());

            (
                StatusCode::OK,
                Json(CloudLogoutResponse {
                    success: true,
                    message: "Successfully logged out and disconnected from Specter Cloud".to_string(),
                }),
            ).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(CloudLogoutResponse {
                success: false,
                message: format!("Failed to remove credentials: {}", e),
            }),
        ).into_response(),
    }
}

#[utoipa::path(
    tag = "System",
    post,
    path = "/api/v1/system/cloud/auth/start",
    operation_id = "start_desktop_auth",
    summary = "Initiate Desktop SSO authorization flow via Tuquet Admin",
    description = "Generates authorization URL for Tuquet Admin portal with local loopback configuration.",
    responses(
        (status = 200, description = "Desktop auth URL generated successfully", body = DesktopAuthStartResponse)
    )
)]
pub async fn start_desktop_auth(
    State(_state): State<crate::AppState>,
) -> impl IntoResponse {
    let nonce = uuid::Uuid::new_v4().to_string();
    let hostname = std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "Local-Workstation".to_string());
    let port = 8765;
    let auth_url = format!(
        "https://admin.tuquet.com/auth/desktop?client_id=tuquet-desktop&state={}&port={}&machine_name={}",
        nonce, port, hostname
    );

    // Record nonce in memory with TTL cleanup (10 minutes)
    if let Ok(mut map) = PENDING_AUTH_NONCES.lock() {
        let now = Instant::now();
        map.retain(|_, created_at| now.duration_since(*created_at) < Duration::from_secs(600));
        map.insert(nonce.clone(), now);
    }

    (
        StatusCode::OK,
        Json(DesktopAuthStartResponse {
            auth_url,
            state: nonce,
            port,
        }),
    ).into_response()
}

async fn process_desktop_auth_payload(
    state: &crate::AppState,
    params: &DesktopAuthCallbackParams,
) -> Result<(Option<String>, Option<String>), (StatusCode, String)> {
    // 1. Validate CSRF state nonce if provided
    let is_valid_state = if let Some(ref st) = params.state {
        if let Ok(mut map) = PENDING_AUTH_NONCES.lock() {
            let now = Instant::now();
            map.retain(|_, created_at| now.duration_since(*created_at) < Duration::from_secs(600));
            map.remove(st).is_some()
        } else {
            true
        }
    } else {
        true
    };

    if !is_valid_state {
        return Err((StatusCode::BAD_REQUEST, "CSRF state nonce is invalid or expired".to_string()));
    }

    // 2. Validate token and persist session
    if let Some(ref token) = params.token {
        let tenant_id = params.tenant_id.clone();
        let data_dir = state.config.data_dir.clone();
        let cloud_url = state.config.cloud_url.clone()
            .unwrap_or_else(|| "https://dswhacsoaxgpfnkaxnhz.supabase.co".to_string());

        let client = crate::infrastructure::cloud::CloudApiClient::build_http_client();
        let anon_key = crate::infrastructure::cloud::DEFAULT_SUPABASE_ANON_KEY.to_string();

        match crate::infrastructure::cloud::SupabaseAuthClient::validate_token(&client, &cloud_url, &anon_key, token).await {
            Ok(user) => {
                let mut creds = crate::infrastructure::cloud::CredentialStore::load(&data_dir).await.unwrap_or_default();
                creds.user_id = Some(user.id.clone());
                creds.email = user.email.clone();
                creds.access_token = Some(token.clone());
                if let Some(ref rt) = params.refresh_token {
                    creds.refresh_token = Some(rt.clone());
                }
                creds.cloud_url = Some(cloud_url);
                creds.api_key = Some(anon_key);
                if let Some(ref t) = tenant_id {
                    creds.tenant_id = Some(t.clone());
                }

                let _ = crate::infrastructure::cloud::CredentialStore::save(&data_dir, &creds).await;

                // Broadcast SSE event
                let _ = state.tx.send(serde_json::json!({
                    "type": "auth_state_changed",
                    "isAuthenticated": true,
                    "userEmail": user.email,
                    "tenantId": tenant_id,
                    "message": "Authenticated with Tuquet Admin successfully"
                }).to_string());

                Ok((user.email, tenant_id))
            }
            Err(e) => Err((StatusCode::UNAUTHORIZED, format!("Token validation failed: {}", e))),
        }
    } else {
        Err((StatusCode::BAD_REQUEST, "Missing authentication token".to_string()))
    }
}

#[utoipa::path(
    tag = "System",
    get,
    path = "/api/v1/auth/callback",
    operation_id = "desktop_auth_callback",
    summary = "Handle loopback authorization callback from Tuquet Admin (GET redirect)",
    description = "Receives delegated session token from Tuquet Admin consent screen via URL redirect, saves to local keystore, and renders success page.",
    params(
        DesktopAuthCallbackParams
    ),
    responses(
        (status = 200, description = "Authorization callback received and processed")
    )
)]
pub async fn desktop_auth_callback(
    State(state): State<crate::AppState>,
    axum::extract::Query(params): axum::extract::Query<DesktopAuthCallbackParams>,
) -> impl IntoResponse {
    match process_desktop_auth_payload(&state, &params).await {
        Ok(_) => {
            let html = r#"<!DOCTYPE html>
<html>
<head>
  <meta charset="utf-8">
  <title>Specter Desk Connected</title>
  <style>
    body { font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif; display: flex; align-items: center; justify-content: center; height: 100vh; margin: 0; background: #f8fafc; }
    .card { text-align: center; background: white; padding: 2.5rem; border-radius: 1rem; box-shadow: 0 10px 15px -3px rgba(0, 0, 0, 0.1); max-width: 420px; }
    h2 { color: #0284c7; margin-top: 0; margin-bottom: 0.5rem; }
    p { color: #64748b; margin-bottom: 1.5rem; }
    .hint { color: #94a3b8; font-size: 0.875rem; }
  </style>
</head>
<body>
  <div class="card">
    <h2>Specter Desk Connected!</h2>
    <p>Your workstation is now securely paired with Tuquet Admin.</p>
    <div class="hint">You can safely close this browser tab.</div>
  </div>
  <script>setTimeout(() => window.close(), 1500);</script>
</body>
</html>"#;
            (
                StatusCode::OK,
                [("content-type", "text/html; charset=utf-8")],
                html.to_string(),
            ).into_response()
        }
        Err((code, msg)) => {
            let html = format!(
                r#"<!DOCTYPE html><html><body style="font-family: sans-serif; text-align: center; padding: 3rem;">
                <h2 style="color: #ef4444;">Xác thực không thành công ({})</h2>
                <p>{}</p>
                <p>Vui lòng mở lại Specter Desk và thử lại.</p></body></html>"#,
                code, msg
            );
            (
                code,
                [("content-type", "text/html; charset=utf-8")],
                html,
            ).into_response()
        }
    }
}

#[utoipa::path(
    tag = "System",
    post,
    path = "/api/v1/auth/callback",
    operation_id = "desktop_auth_callback_post",
    summary = "Handle loopback authorization callback from Tuquet Admin (POST JSON body)",
    description = "Securely receives delegated session token in JSON body to prevent token exposure in URLs and browser history.",
    request_body = DesktopAuthCallbackParams,
    responses(
        (status = 200, description = "Authorization callback received and processed successfully", body = DesktopAuthCallbackResponse),
        (status = 400, description = "Invalid or expired state nonce", body = DesktopAuthCallbackResponse),
        (status = 401, description = "Token validation failed", body = DesktopAuthCallbackResponse)
    )
)]
pub async fn desktop_auth_callback_post(
    State(state): State<crate::AppState>,
    Json(params): Json<DesktopAuthCallbackParams>,
) -> impl IntoResponse {
    match process_desktop_auth_payload(&state, &params).await {
        Ok((user_email, tenant_id)) => (
            StatusCode::OK,
            Json(DesktopAuthCallbackResponse {
                success: true,
                message: "Workstation paired successfully with Tuquet Admin".to_string(),
                user_email,
                tenant_id,
            }),
        ).into_response(),
        Err((code, msg)) => (
            code,
            Json(DesktopAuthCallbackResponse {
                success: false,
                message: msg,
                user_email: None,
                tenant_id: None,
            }),
        ).into_response(),
    }
}
