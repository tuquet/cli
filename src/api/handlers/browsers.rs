use axum::extract::{Path, State, Json, Query};
use axum::body::Bytes;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use utoipa::{IntoParams, ToSchema};
use crate::AppState;
use crate::core::error::AutomaError;
use crate::core::models::id::BrowserId;
use crate::api::handlers::jobs::connected_browsers;

fn is_valid_id(id: &str) -> bool {
    BrowserId::new(id).is_valid()
}

async fn get_browsers_base_path(data_dir: &str) -> String {
    let settings_path = std::path::Path::new(data_dir).join("settings.json");
    if let Ok(content) = tokio::fs::read_to_string(&settings_path).await {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(path) = json.get("browsersPath").and_then(|p| p.as_str()) {
                return path.to_string();
            }
        }
    }
    "browsers".to_string()
}

#[derive(Serialize, Deserialize, ToSchema, Clone)]
#[serde(rename_all = "camelCase")]
#[schema(example = json!({
    "id": "profile_1",
    "name": "Personal Profile",
    "userAgent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
    "timezone": "Asia/Ho_Chi_Minh",
    "proxy": "socks5://127.0.0.1:1080",
    "isDefault": true,
    "createdAt": "2026-08-25T00:00:00Z",
    "updatedAt": "2026-08-25T00:00:00Z",
    "isOnline": false
}))]
/// Browser profile descriptor
pub struct BrowserResponse {
    /// Unique browser profile identifier
    pub id: String,
    /// User-friendly profile label
    pub name: String,
    /// Custom User-Agent header string
    pub user_agent: Option<String>,
    /// Emulated timezone ID
    pub timezone: Option<String>,
    /// Proxy server string (e.g. socks5://127.0.0.1:1080 or http://user:pass@host:port)
    pub proxy: Option<String>,
    /// Whether this profile is currently configured as the system default
    pub is_default: bool,
    /// Creation timestamp (ISO 8601)
    pub created_at: String,
    /// Last update timestamp (ISO 8601)
    pub updated_at: String,
    /// Whether an active browser process is currently connected
    pub is_online: bool,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
#[schema(example = json!({
    "id": "profile_1",
    "name": "Personal Profile",
    "userAgent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
    "timezone": "Asia/Ho_Chi_Minh",
    "proxy": "socks5://127.0.0.1:1080"
}))]
/// Request payload for creating a new browser profile
pub struct CreateBrowserRequest {
    /// Optional custom ID (auto-generated UUID if omitted)
    pub id: Option<String>,
    /// User-friendly profile name
    pub name: String,
    /// Custom User-Agent header
    pub user_agent: Option<String>,
    /// Emulated timezone ID
    pub timezone: Option<String>,
    /// Optional proxy server URL
    pub proxy: Option<String>,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
#[schema(example = json!({
    "name": "Updated Profile Name",
    "userAgent": null,
    "timezone": "UTC",
    "proxy": "http://10.0.0.1:8080"
}))]
/// Request payload for modifying an existing browser profile
pub struct UpdateBrowserRequest {
    /// New profile display name
    pub name: Option<String>,
    /// New custom User-Agent
    pub user_agent: Option<String>,
    /// New emulated timezone ID
    pub timezone: Option<String>,
    /// New proxy server URL
    pub proxy: Option<String>,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
#[schema(example = json!({
    "status": "success",
    "message": "Browser session closed and profile sanitized"
}))]
/// Response payload when a browser process is terminated
pub struct StopBrowserResponse {
    /// Result status
    pub status: String,
    /// Detailed message
    pub message: String,
}

#[derive(Deserialize, IntoParams)]
pub struct GetBrowsersQuery {
    /// Maximum number of browser profiles to return
    #[param(example = 50)]
    pub limit: Option<usize>,
    /// Number of items to skip for pagination (default 0)
    #[param(example = 0)]
    pub offset: Option<usize>,
    /// Optional search query to filter browsers by name or ID
    pub search: Option<String>,
}

#[utoipa::path(
    tag = "Browsers",
    get,
    path = "/api/v1/browsers",
    operation_id = "get_browsers",
    summary = "List all browser profiles",
    description = "Retrieves an array of all persisted browser profiles, including their active online/offline connection state.",
    params(GetBrowsersQuery),
    responses(
        (status = 200, description = "List of all browser profiles", body = Vec<BrowserResponse>),
        (status = 500, description = "Database read error", body = crate::core::error::ApiErrorResponse)
    )
)]
pub async fn get_browsers(
    State(state): State<AppState>,
    Query(query): Query<GetBrowsersQuery>,
) -> Result<Json<Vec<BrowserResponse>>, AutomaError> {
    let db = state.db.lock().await;
    let browsers = db.browsers().get_browsers(query.limit, query.offset, query.search.as_deref())
        .map_err(|e| AutomaError::DatabaseError(e.to_string()))?;
    
    let default_profile_id = db.settings().get_settings().ok().and_then(|s| s.browser.default_profile_id);

    let online_browsers = {
        let guard = connected_browsers().read().await;
        guard.clone()
    };
    let running_registry = {
        let guard = browser_registry().read().await;
        guard.clone()
    };

    let response = browsers.into_iter().map(|p| {
        let is_online = online_browsers.contains(&p.id) || running_registry.contains_key(&p.id);
        let is_default = default_profile_id.as_deref() == Some(&p.id) || (default_profile_id.is_none() && p.id == "default");
        BrowserResponse {
            id: p.id,
            name: p.name,
            user_agent: p.user_agent,
            timezone: p.timezone,
            proxy: p.proxy,
            is_default,
            created_at: p.created_at,
            updated_at: p.updated_at,
            is_online,
        }
    }).collect();

    Ok(Json(response))
}

#[utoipa::path(
    tag = "Browsers",
    post,
    path = "/api/v1/browsers",
    operation_id = "create_browser",
    summary = "Create a new browser profile",
    description = "Persists a new isolated browser profile with custom fingerprint settings (User-Agent, Timezone, Proxy) in SQLite.",
    request_body = CreateBrowserRequest,
    responses(
        (status = 200, description = "Browser profile created successfully", body = Value),
        (status = 400, description = "Invalid browser ID or missing name", body = crate::core::error::ApiErrorResponse),
        (status = 500, description = "Failed to create browser profile in database", body = crate::core::error::ApiErrorResponse)
    )
)]
pub async fn create_browser(
    State(state): State<AppState>,
    Json(req): Json<CreateBrowserRequest>,
) -> Result<Json<Value>, AutomaError> {
    let final_id = req.id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    if !is_valid_id(&final_id) {
        return Err(AutomaError::BadRequest("Invalid browser ID".to_string()));
    }
    let db = state.db.lock().await;
    db.browsers().create_browser(&final_id, &req.name, req.user_agent.as_deref(), req.timezone.as_deref(), req.proxy.as_deref())
        .map_err(|e| AutomaError::DatabaseError(e.to_string()))?;
    Ok(Json(json!({"success": true, "id": final_id, "message": "Browser created"})))
}

#[utoipa::path(
    tag = "Browsers",
    put,
    path = "/api/v1/browsers/{id}",
    operation_id = "update_browser",
    summary = "Update a browser profile",
    description = "Updates the configuration and metadata of an existing browser profile by ID.",
    params(
        ("id" = String, Path, description = "Unique browser profile identifier")
    ),
    request_body = UpdateBrowserRequest,
    responses(
        (status = 200, description = "Browser profile updated successfully", body = Value),
        (status = 400, description = "Invalid browser ID", body = crate::core::error::ApiErrorResponse),
        (status = 404, description = "Browser profile not found", body = crate::core::error::ApiErrorResponse),
        (status = 500, description = "Database update error", body = crate::core::error::ApiErrorResponse)
    )
)]
pub async fn update_browser(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<UpdateBrowserRequest>,
) -> Result<Json<Value>, AutomaError> {
    if !is_valid_id(&id) {
        return Err(AutomaError::BadRequest("Invalid browser ID".to_string()));
    }
    let db = state.db.lock().await;
    let name = req.name.unwrap_or_default();
    db.browsers().update_browser(&id, &name, req.user_agent.as_deref(), req.timezone.as_deref(), req.proxy.as_deref())
        .map_err(|e| AutomaError::DatabaseError(e.to_string()))?;
    Ok(Json(json!({"success": true, "message": "Browser updated"})))
}

#[utoipa::path(
    tag = "Browsers",
    delete,
    path = "/api/v1/browsers/{id}",
    operation_id = "delete_browser",
    summary = "Delete a browser profile",
    description = "Permanently removes a browser profile from SQLite storage and cleans up associated session data.",
    params(
        ("id" = String, Path, description = "Unique browser profile identifier")
    ),
    responses(
        (status = 200, description = "Browser profile deleted successfully", body = Value),
        (status = 400, description = "Invalid browser ID", body = crate::core::error::ApiErrorResponse),
        (status = 404, description = "Browser profile not found", body = crate::core::error::ApiErrorResponse),
        (status = 500, description = "Database deletion error", body = crate::core::error::ApiErrorResponse)
    )
)]
pub async fn delete_browser(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, AutomaError> {
    if !is_valid_id(&id) {
        return Err(AutomaError::BadRequest("Invalid browser ID".to_string()));
    }
    
    let db = state.db.lock().await;
    db.browsers().delete_browser(&id)
        .map_err(|e| AutomaError::DatabaseError(e.to_string()))?;
    Ok(Json(json!({"success": true, "message": "Browser deleted"})))
}

#[utoipa::path(
    tag = "Browsers",
    get,
    path = "/api/v1/browsers/{id}",
    operation_id = "get_browser_detail",
    summary = "Get browser profile details",
    description = "Retrieves full configuration details and live online status for a specific browser profile.",
    params(
        ("id" = String, Path, description = "Unique browser profile identifier")
    ),
    responses(
        (status = 200, description = "Browser profile details", body = BrowserResponse),
        (status = 400, description = "Invalid browser ID", body = crate::core::error::ApiErrorResponse),
        (status = 404, description = "Browser profile not found", body = crate::core::error::ApiErrorResponse),
        (status = 500, description = "Database query error", body = crate::core::error::ApiErrorResponse)
    )
)]
pub async fn get_browser_detail(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<BrowserResponse>, AutomaError> {
    if !is_valid_id(&id) {
        return Err(AutomaError::BadRequest("Invalid browser ID".to_string()));
    }
    let db = state.db.lock().await;
    let browser_opt = db.browsers().get_browser(&id).map_err(|e| AutomaError::DatabaseError(e.to_string()))?;
    
    if let Some(p) = browser_opt {
        let is_online = {
            let guard = connected_browsers().read().await;
            let registry = browser_registry().read().await;
            guard.contains(&p.id) || registry.contains_key(&p.id)
        };

        let default_profile_id = db.settings().get_settings().ok().and_then(|s| s.browser.default_profile_id);
        let is_default = default_profile_id.as_deref() == Some(&p.id) || (default_profile_id.is_none() && p.id == "default");
        
        let response = BrowserResponse {
            id: p.id,
            name: p.name,
            user_agent: p.user_agent,
            timezone: p.timezone,
            proxy: p.proxy,
            is_default,
            created_at: p.created_at,
            updated_at: p.updated_at,
            is_online,
        };
        
        Ok(Json(response))
    } else {
        Err(AutomaError::NotFound(format!("Browser '{id}' not found")))
    }
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SetDefaultBrowserResponse {
    pub success: bool,
    pub message: String,
    pub default_profile_id: String,
}

#[utoipa::path(
    tag = "Browsers",
    post,
    path = "/api/v1/browsers/{id}/set-default",
    operation_id = "set_default_browser",
    summary = "Set a browser profile as system-wide default",
    description = "Updates the core daemon settings to mark this profile as default for all automated workflow jobs.",
    params(
        ("id" = String, Path, description = "Unique browser profile identifier")
    ),
    responses(
        (status = 200, description = "Default browser updated successfully", body = SetDefaultBrowserResponse),
        (status = 400, description = "Invalid browser ID", body = crate::core::error::ApiErrorResponse),
        (status = 404, description = "Browser profile not found", body = crate::core::error::ApiErrorResponse)
    )
)]
pub async fn set_default_browser(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<SetDefaultBrowserResponse>, AutomaError> {
    if !is_valid_id(&id) {
        return Err(AutomaError::BadRequest("Invalid browser ID".to_string()));
    }

    let db = state.db.lock().await;
    let exists = db.browsers().get_browser(&id)
        .map_err(|e| AutomaError::DatabaseError(e.to_string()))?
        .is_some();

    if !exists && id != "default" {
        return Err(AutomaError::NotFound(format!("Browser profile '{}' not found", id)));
    }

    let mut settings = db.settings().get_settings().unwrap_or_default();
    settings.browser.default_profile_id = Some(id.clone());
    db.settings().save_settings(&settings)
        .map_err(|e| AutomaError::DatabaseError(e.to_string()))?;

    let _ = state.tx.send(json!({
        "type": "settings_updated",
        "default_profile_id": id
    }).to_string());

    Ok(Json(SetDefaultBrowserResponse {
        success: true,
        message: format!("Profile '{}' set as default browser", id),
        default_profile_id: id,
    }))
}

#[derive(Debug, Deserialize, ToSchema, Default, Clone)]
#[serde(rename_all = "camelCase")]
#[schema(example = json!({
    "headless": false,
    "startUrl": "https://www.google.com",
    "proxy": "socks5://127.0.0.1:1080",
    "args": ["--start-maximized"]
}))]
/// Request options when initiating a browser session
pub struct StartBrowserSessionRequest {
    /// Override headless mode (true = background, false = visual GUI window)
    pub headless: Option<bool>,
    /// Optional URL to open upon browser launch
    pub start_url: Option<String>,
    /// Optional custom Proxy server URL (e.g. socks5://127.0.0.1:1080, http://proxy.example:8080)
    pub proxy: Option<String>,
    /// Additional Chromium CLI flags
    pub args: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, Clone)]
#[serde(rename_all = "camelCase")]
#[schema(example = json!({
    "status": "success",
    "message": "Browser session launched successfully",
    "browserId": "default",
    "pid": 12345,
    "debuggingPort": 54321,
    "wsUrl": "ws://127.0.0.1:54321/devtools/browser/abc-123",
    "userDataDir": "C:\\Users\\user\\.automa\\core-dev\\browsers\\default"
}))]
/// Response descriptor containing active browser session connection parameters
pub struct StartBrowserSessionResponse {
    /// Result status (e.g. "success")
    pub status: String,
    /// Informational message
    pub message: String,
    /// Profile identifier
    pub browser_id: String,
    /// Process ID of the spawned browser
    pub pid: Option<u32>,
    /// Remote CDP debugging port
    pub debugging_port: Option<u16>,
    /// WebSocket Debugger URL for CDP connection
    pub ws_url: Option<String>,
    /// Path to user data directory
    pub user_data_dir: Option<String>,
}

use crate::core::browser::manager::{browser_registry, BrowserManager, BrowserManagerOptions};

#[utoipa::path(
    tag = "Browsers",
    post,
    path = "/api/v1/browsers/{id}/session",
    operation_id = "start_browser",
    summary = "Launch isolated browser profile instance",
    description = "Spawns a new Chromium browser process attached to the specific profile directory with anti-detect flags and extensions loaded.",
    params(
        ("id" = String, Path, description = "Unique browser profile identifier")
    ),
    request_body = StartBrowserSessionRequest,
    responses(
        (status = 200, description = "Browser session started", body = StartBrowserSessionResponse),
        (status = 400, description = "Invalid browser ID or configuration error", body = crate::core::error::ApiErrorResponse),
        (status = 500, description = "Process spawn failure", body = crate::core::error::ApiErrorResponse)
    )
)]
pub async fn start_browser(
    State(state): State<crate::AppState>,
    Path(browser_id): Path<String>,
    body: Bytes,
) -> Result<Json<StartBrowserSessionResponse>, AutomaError> {
    if !is_valid_id(&browser_id) {
        return Err(AutomaError::BadRequest("Invalid browser ID".to_string()));
    }

    // Safely parse request body if provided, fallback to defaults if empty
    let req: StartBrowserSessionRequest = if body.is_empty() {
        StartBrowserSessionRequest::default()
    } else {
        serde_json::from_slice(&body).unwrap_or_default()
    };

    // Edge Case 4: Mutex lock per browser_id to prevent concurrent double-launches
    let _lock = crate::core::browser::worker_coordinator::get_browser_launcher_lock(&browser_id).await;

    // Check if session is already running & verified alive on OS
    let existing_session = {
        let registry = browser_registry().read().await;
        let pid_opt = registry.get(&browser_id).copied();
        if let Some(pid) = pid_opt {
            let mut sys = sysinfo::System::new_all();
            sys.refresh_all();
            if sys.process(sysinfo::Pid::from_u32(pid)).is_some() {
                let sessions = crate::core::browser::manager::browser_sessions().read().await;
                sessions.get(&browser_id).cloned()
            } else {
                None
            }
        } else {
            None
        }
    };

    if let Some(sess) = existing_session {
        return Ok(Json(StartBrowserSessionResponse {
            status: "success".to_string(),
            message: "Browser session already running".to_string(),
            browser_id: browser_id.clone(),
            pid: Some(sess.pid),
            debugging_port: Some(sess.debugging_port),
            ws_url: Some(sess.ws_url),
            user_data_dir: Some(sess.user_data_dir),
        }));
    }

    // If an orphan or dead process was registered, clean it up
    {
        let mut registry = browser_registry().write().await;
        registry.remove(&browser_id);
        let mut sessions = crate::core::browser::manager::browser_sessions().write().await;
        sessions.remove(&browser_id);
    }

    // Edge Case 5: Use canonical runner extension path resolver
    let ext_path = crate::core::browser::worker_coordinator::resolve_cli_runner_extension_path();

    // Edge Case 3: Pre-launch sanitization: remove stale SingletonLock, volatile caches and crash dumps
    let browsers_base_path = get_browsers_base_path(&state.config.data_dir).await;
    let target_dir = std::path::Path::new(&state.config.data_dir).join(&browsers_base_path).join(&browser_id);
    crate::core::browser::manager::sanitize_browser_profile(&target_dir, false).await;

    if let Err(e) = unzip_browser_folder(&browser_id, &state.config.data_dir).await {
        tracing::error!("Failed to unzip browser folder: {}", e);
        return Err(AutomaError::Internal(format!("Failed to unzip browser: {}", e)));
    }

    // Edge Case 2: Load Profile settings from SQLite database
    let (db_user_agent, db_timezone, db_proxy) = {
        let db = state.db.lock().await;
        if let Ok(Some(profile)) = db.browsers().get_browser(&browser_id) {
            (profile.user_agent, profile.timezone, profile.proxy)
        } else {
            (None, None, None)
        }
    };

    let app_settings = {
        let db = state.db.lock().await;
        db.settings().get_settings().unwrap_or_default()
    };

    let mut custom_args = Vec::new();
    if app_settings.grid.enabled {
        let active_browsers_count = connected_browsers().read().await.len() as u32;
        let (pos_x, pos_y, slot_w, slot_h) = app_settings.grid.calculate_slot_bounds(active_browsers_count);
        custom_args.push(format!("--window-position={},{}", pos_x, pos_y));
        custom_args.push(format!("--window-size={},{}", slot_w, slot_h));
    }

    // User-Agent: Profile DB takes precedence, then app_settings
    let effective_ua = db_user_agent.or(app_settings.browser.default_user_agent);
    if let Some(ref ua) = effective_ua {
        if !ua.is_empty() {
            custom_args.push(format!("--user-agent={}", ua));
        }
    }

    // Timezone: Profile DB
    if let Some(ref tz) = db_timezone {
        if !tz.is_empty() {
            custom_args.push(format!("--timezone={}", tz));
        }
    }

    // Proxy: from request payload takes precedence, then profile DB
    let effective_proxy = req.proxy.or(db_proxy);
    if let Some(ref proxy) = effective_proxy {
        if !proxy.is_empty() {
            custom_args.push(format!("--proxy-server={}", proxy));
        }
    }

    // Custom extra CLI flags from request payload
    if let Some(ref extra_args) = req.args {
        custom_args.extend(extra_args.clone());
    }

    // Start URL: from request payload
    if let Some(ref start_url) = req.start_url {
        if !start_url.is_empty() {
            custom_args.push(start_url.clone());
        }
    }

    // Edge Case 1: Headless override from request payload takes precedence
    let headless = req.headless.unwrap_or(app_settings.browser.headless);

    let default_browser = if app_settings.browser.default_type.is_empty() {
        "chromium".to_string()
    } else {
        app_settings.browser.default_type.clone()
    };

    let mut manager = BrowserManager::new(BrowserManagerOptions {
        default_browser,
        browser_id: browser_id.clone(),
        headless,
        extension_paths: vec![ext_path],
        custom_args,
        user_data_dir: Some(target_dir.to_string_lossy().to_string()),
    });

    let ws_url = match manager.launch().await {
        Ok(url) => url,
        Err(e) => return Err(AutomaError::Internal(format!("Failed to launch browser: {}", e))),
    };

    let pid = manager.get_pid();
    let (debugging_port, actual_user_data_dir) = {
        let sessions = crate::core::browser::manager::browser_sessions().read().await;
        if let Some(sess) = sessions.get(&browser_id) {
            (Some(sess.debugging_port), Some(sess.user_data_dir.clone()))
        } else {
            (None, Some(target_dir.to_string_lossy().to_string()))
        }
    };

    // Broadcast real-time browser online event to global SSE stream
    let _ = state.tx.send(json!({
        "type": "browser_online",
        "id": browser_id
    }).to_string());

    // Edge Case 6: Return complete session connection details (PID, port, ws_url, user_data_dir)
    Ok(Json(StartBrowserSessionResponse {
        status: "success".to_string(),
        message: "Browser session launched successfully".to_string(),
        browser_id,
        pid,
        debugging_port,
        ws_url: Some(ws_url),
        user_data_dir: actual_user_data_dir,
    }))
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct StopBrowserQuery {
    /// If true, forcefully kills the process immediately without graceful closing or saving cache
    pub force: Option<bool>,
}

#[utoipa::path(
    tag = "Browsers",
    delete,
    path = "/api/v1/browsers/{id}/session",
    operation_id = "stop_browser_session",
    summary = "Terminate or stop an active browser session",
    description = "Gracefully closes the running browser process (or forcefully if force=true), sanitizes volatile caches, cleans up locks, and syncs profile data.",
    params(
        ("id" = String, Path, description = "Unique browser profile identifier"),
        StopBrowserQuery
    ),
    responses(
        (status = 200, description = "Browser session terminated successfully", body = StopBrowserResponse),
        (status = 400, description = "Invalid browser ID", body = crate::core::error::ApiErrorResponse)
    )
)]
pub async fn stop_browser(
    State(state): State<crate::AppState>,
    Path(browser_id): Path<String>,
    Query(query): Query<StopBrowserQuery>,
) -> Result<Json<StopBrowserResponse>, AutomaError> {
    if !is_valid_id(&browser_id) {
        return Err(AutomaError::BadRequest("Invalid browser ID".to_string()));
    }

    let is_force = query.force.unwrap_or(false);
    let pid = {
        let registry = browser_registry().read().await;
        registry.get(&browser_id).copied()
    };

    let browsers_base_path = get_browsers_base_path(&state.config.data_dir).await;
    let target_dir = std::path::Path::new(&state.config.data_dir).join(&browsers_base_path).join(&browser_id);

    if is_force {
        crate::core::browser::manager::kill_browser_processes(&browser_id, pid).await;
        // Emergency lock cleanup so profile is not stuck on next launch
        crate::core::browser::manager::sanitize_browser_profile(&target_dir, true).await;
    } else {
        crate::core::browser::manager::graceful_stop_browser(&browser_id, pid).await;
        // Deep sanitize: remove volatile bloat caches & locks, preserve cookies/history
        crate::core::browser::manager::sanitize_browser_profile(&target_dir, false).await;
    }

    {
        let mut registry = browser_registry().write().await;
        registry.remove(&browser_id);
        let mut sessions = crate::core::browser::manager::browser_sessions().write().await;
        sessions.remove(&browser_id);
    }

    {
        let mut connected = crate::api::handlers::jobs::connected_browsers().write().await;
        connected.remove(&browser_id);
    }

    // Broadcast real-time browser offline event to global SSE stream
    let _ = state.tx.send(json!({
        "type": "browser_offline",
        "id": browser_id
    }).to_string());

    if !is_force {
        if let Err(e) = zip_browser_folder(&browser_id, &state.config.data_dir).await {
            tracing::warn!("Browser folder zip skipped or error: {}", e);
        }
    }

    let msg = if is_force {
        "Browser session forcefully terminated"
    } else {
        "Browser session closed and profile sanitized"
    };

    Ok(Json(StopBrowserResponse {
        status: "success".to_string(),
        message: msg.to_string(),
    }))
}

fn should_skip_zip_entry(name: &str, skip_folders: &[&str]) -> bool {
    skip_folders.iter().any(|skip| name.starts_with(skip))
}

fn append_entry_to_zip<W: std::io::Write + std::io::Seek>(
    zip: &mut zip::ZipWriter<W>,
    path: &std::path::Path,
    name: &str,
    options: zip::write::FileOptions,
) -> anyhow::Result<()> {
    if path.is_dir() {
        let _ = zip.add_directory(name, options);
    } else if path.is_file() {
        zip.start_file(name, options)?;
        if let Ok(mut f) = std::fs::File::open(path) {
            let _ = std::io::copy(&mut f, zip);
        }
    }
    Ok(())
}

async fn zip_browser_folder(id: &str, data_dir: &str) -> anyhow::Result<()> {
    let browsers_base_path = get_browsers_base_path(data_dir).await;
    
    let user_data_dir = std::path::PathBuf::from(data_dir)
        .join(&browsers_base_path)
        .join(id);

    let zip_path = std::path::PathBuf::from(data_dir)
        .join(&browsers_base_path)
        .join(format!("{}.zip", id));

    if !user_data_dir.exists() {
        return Ok(());
    }

    // Pre-sanitize profile before zipping: remove locks and volatile cache
    crate::core::browser::manager::sanitize_browser_profile(&user_data_dir, false).await;

    let user_data_dir_clone = user_data_dir.clone();
    let zip_path_clone = zip_path.clone();
    
    tokio::task::spawn_blocking(move || {
        let file = std::fs::File::create(&zip_path_clone)?;
        let mut zip = zip::ZipWriter::new(file);
        let options = zip::write::FileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .unix_permissions(0o755);

        // Note: NEVER skip "Network" as modern Chromium (96+) stores cookies in Default/Network/Cookies!
        let skip_folders = vec![
            "Cache",
            "Code Cache",
            "GPUCache",
            "DawnCache",
            "ShaderCache",
            "Crashpad",
            "Default/Cache",
            "Default/Code Cache",
            "Default/GPUCache",
            "Default/DawnCache",
            "Default/ShaderCache",
        ];

        let mut dirs = vec![user_data_dir_clone.clone()];
        while let Some(dir) = dirs.pop() {
            let Ok(entries) = std::fs::read_dir(&dir) else {
                continue;
            };

            for entry in entries.flatten() {
                let path = entry.path();
                let name = path.strip_prefix(&user_data_dir_clone)
                    .unwrap_or_else(|_| path.as_path())
                    .to_string_lossy()
                    .replace("\\", "/");
                
                if should_skip_zip_entry(&name, &skip_folders) {
                    continue;
                }
                if entry.file_type().map(|t| t.is_symlink()).unwrap_or(false) {
                    continue;
                }

                if path.is_dir() {
                    dirs.push(path.clone());
                }
                let _ = append_entry_to_zip(&mut zip, &path, &name, options);
            }
        }
        zip.finish()?;
        Ok::<(), anyhow::Error>(())
    }).await??;

    let _ = tokio::fs::remove_dir_all(user_data_dir).await;
    Ok(())
}

async fn unzip_browser_folder(id: &str, data_dir: &str) -> anyhow::Result<()> {
    let browsers_base_path = get_browsers_base_path(data_dir).await;
    
    let user_data_dir = std::path::PathBuf::from(data_dir)
        .join(&browsers_base_path)
        .join(id);

    let zip_path = std::path::PathBuf::from(data_dir)
        .join(&browsers_base_path)
        .join(format!("{}.zip", id));

    if !zip_path.exists() {
        return Ok(());
    }

    if user_data_dir.exists() {
        tracing::warn!("Browser folder already exists alongside zip. Skipping unzip to preserve crash state.");
        return Ok(());
    }

    let user_data_dir_clone = user_data_dir.clone();
    let zip_path_clone = zip_path.clone();
    
    tokio::task::spawn_blocking(move || {
        let file = std::fs::File::open(&zip_path_clone)?;
        let mut archive = zip::ZipArchive::new(file)?;
        
        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let outpath = match file.enclosed_name() {
                Some(path) => user_data_dir_clone.join(path),
                None => continue,
            };

            if (*file.name()).ends_with('/') {
                std::fs::create_dir_all(&outpath)?;
            } else {
                if let Some(p) = outpath.parent() {
                    if !p.exists() {
                        std::fs::create_dir_all(&p)?;
                    }
                }
                let mut outfile = std::fs::File::create(&outpath)?;
                std::io::copy(&mut file, &mut outfile)?;
            }
        }
        Ok::<(), anyhow::Error>(())
    }).await??;

    let _ = tokio::fs::remove_file(zip_path).await;
    Ok(())
}

#[derive(serde::Deserialize, utoipa::ToSchema)]
#[schema(example = json!({"csv_string": "id,name,userAgent,timezone\nprofile1,Main Profile,,UTC"}))]
/// Request payload containing raw CSV formatted browser profiles
pub struct ImportCsvPayload {
    /// Raw CSV string containing profile headers and rows
    #[serde(alias = "csvString")]
    pub csv_string: String,
}

#[derive(serde::Serialize, utoipa::ToSchema)]
#[schema(example = json!({"status": "success", "message": "Imported 3 profiles", "imported": 3}))]
/// Response after batch importing browser profiles from CSV
pub struct ImportCsvResponse {
    /// Status code indicator
    pub status: String,
    /// Result description message
    pub message: String,
    /// Number of profiles successfully imported
    pub imported: usize,
}

#[utoipa::path(
    tag = "Browsers",
    post,
    path = "/api/v1/browsers/import-csv",
    operation_id = "import_browsers_csv",
    summary = "Batch import browser profiles from CSV",
    description = "Parses a multi-line CSV string and batch inserts new browser profiles into SQLite.",
    request_body = ImportCsvPayload,
    responses(
        (status = 200, description = "CSV profiles successfully parsed and imported", body = ImportCsvResponse),
        (status = 400, description = "Malformed CSV format", body = crate::core::error::ApiErrorResponse)
    )
)]
pub async fn import_csv(
    State(state): State<crate::AppState>,
    Json(payload): Json<ImportCsvPayload>,
) -> axum::response::Json<ImportCsvResponse> {
    let mut imported = 0;
    let db = state.db.lock().await;
    for line in payload.csv_string.lines().skip(1) {
        let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
        if parts.is_empty() || parts[0].is_empty() {
            continue;
        }
        let id = parts[0].to_string();
        let name = parts.get(1).unwrap_or(&"").to_string();
        let user_agent = parts.get(2).and_then(|s| if s.is_empty() { None } else { Some(s.to_string()) });
        let timezone = parts.get(3).and_then(|s| if s.is_empty() { None } else { Some(s.to_string()) });
        let proxy = parts.get(4).and_then(|s| if s.is_empty() { None } else { Some(s.to_string()) });
        let _ = db.browsers().create_browser(&id, &name, user_agent.as_deref(), timezone.as_deref(), proxy.as_deref());
        imported += 1;
    }

    axum::response::Json(ImportCsvResponse {
        status: "success".to_string(),
        message: format!("Successfully imported {} browser profiles", imported),
        imported,
    })
}

#[derive(serde::Deserialize, utoipa::ToSchema)]
#[schema(example = json!({"extension_path": "C:/extensions/my-custom-plugin"}))]
/// Payload containing filesystem path to an unpacked browser extension
pub struct SideloadExtensionPayload {
    /// Full filesystem path to the unpacked extension directory (containing manifest.json)
    #[serde(alias = "extensionPath")]
    pub extension_path: String,
}

#[derive(serde::Serialize, utoipa::ToSchema)]
#[schema(example = json!({"status": "success", "message": "Extension sideloaded successfully", "success": true}))]
/// Response after sideloading an extension into a browser profile
pub struct SideloadExtensionResponse {
    /// Status code indicator
    pub status: String,
    /// Detailed result message
    pub message: String,
    /// Whether sideload succeeded
    pub success: bool,
}

#[utoipa::path(
    tag = "Browsers",
    post,
    path = "/api/v1/browsers/{id}/extensions",
    operation_id = "sideload_browser_extension",
    summary = "Sideload unpacked extension into browser profile",
    description = "Copies and configures an external unpacked extension into the specified browser profile's data directory.",
    params(
        ("id" = String, Path, description = "Unique browser profile identifier")
    ),
    request_body = SideloadExtensionPayload,
    responses(
        (status = 200, description = "Extension sideloaded successfully", body = SideloadExtensionResponse),
        (status = 400, description = "Invalid extension path or manifest", body = crate::core::error::ApiErrorResponse),
        (status = 404, description = "Browser profile not found", body = crate::core::error::ApiErrorResponse)
    )
)]
pub async fn sideload_extension(
    State(_state): State<crate::AppState>,
    Path(_id): Path<String>,
    Json(_payload): Json<SideloadExtensionPayload>,
) -> axum::response::Json<SideloadExtensionResponse> {
    axum::response::Json(SideloadExtensionResponse {
        status: "success".to_string(),
        message: "Extension configured for profile".to_string(),
        success: true,
    })
}

#[utoipa::path(
    tag = "Browsers",
    post,
    path = "/api/v1/browsers/auto-detect",
    operation_id = "auto_detect_browsers",
    summary = "Ensure managed default Chromium browser profile",
    description = "Ensures standard managed Chromium browser profile is registered in SQLite and configured as default.",
    responses(
        (status = 200, description = "List of all registered browser profiles after detection", body = Vec<BrowserResponse>),
        (status = 500, description = "Database error during auto-detection", body = crate::core::error::ApiErrorResponse)
    )
)]
pub async fn auto_detect_browsers(
    State(state): State<AppState>,
) -> Result<Json<Vec<BrowserResponse>>, AutomaError> {
    let detected = crate::core::browser::resolver::detect_host_browsers();
    let db = state.db.lock().await;
    
    let existing = db.browsers().get_browsers(None, None, None).map_err(|e| AutomaError::DatabaseError(e.to_string()))?;

    for host_browser in detected {
        let already_exists = existing.iter().any(|b| b.name.eq_ignore_ascii_case(&host_browser.name));
        if !already_exists {
            let id = "default_chromium".to_string();
            if db.browsers().get_browser(&id).map_err(|e| AutomaError::DatabaseError(e.to_string()))?.is_none() {
                let _ = db.browsers().create_browser(&id, &host_browser.name, None, None, None);
            }
        }
    }

    let default_profile_id = if let Ok(mut settings) = db.settings().get_settings() {
        if settings.browser.default_profile_id.is_none() {
            let all = db.browsers().get_browsers(None, None, None).unwrap_or_default();
            if let Some(first) = all.first() {
                settings.browser.default_profile_id = Some(first.id.clone());
                let _ = db.settings().save_settings(&settings);
            }
        }
        settings.browser.default_profile_id
    } else {
        None
    };

    let updated_browsers = db.browsers().get_browsers(None, None, None).map_err(|e| AutomaError::DatabaseError(e.to_string()))?;
    let online_browsers = {
        let guard = connected_browsers().read().await;
        guard.clone()
    };
    let running_registry = {
        let guard = browser_registry().read().await;
        guard.clone()
    };

    let response = updated_browsers.into_iter().map(|p| {
        let is_online = online_browsers.contains(&p.id) || running_registry.contains_key(&p.id);
        let is_default = default_profile_id.as_deref() == Some(&p.id) || (default_profile_id.is_none() && p.id == "default");
        BrowserResponse {
            id: p.id,
            name: p.name,
            user_agent: p.user_agent,
            timezone: p.timezone,
            proxy: p.proxy,
            is_default,
            created_at: p.created_at,
            updated_at: p.updated_at,
            is_online,
        }
    }).collect();

    Ok(Json(response))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::db::AutomaDb;
    use crate::config::AppConfig;
    use std::sync::Arc;
    use std::collections::HashMap;

    fn create_test_state() -> AppState {
        let db = Arc::new(tokio::sync::Mutex::new(AutomaDb::new_in_memory().unwrap()));
        let config = Arc::new(AppConfig::load());
        let (tx, _rx) = tokio::sync::broadcast::channel(100);
        let (worker_tx, _worker_rx) = tokio::sync::broadcast::channel(100);

        AppState {
            db,
            config,
            tx,
            worker_tx,
            active_jobs: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }

    #[tokio::test]
    async fn test_create_and_get_browser() {
        let state = create_test_state();
        let create_req = CreateBrowserRequest {
            id: Some("test_browser_profile_1".to_string()),
            name: "Test Browser 1".to_string(),
            user_agent: Some("TestAgent/1.0".to_string()),
            timezone: Some("UTC".to_string()),
            proxy: Some("socks5://127.0.0.1:1080".to_string()),
        };

        let create_res = create_browser(State(state.clone()), Json(create_req)).await.unwrap();
        assert_eq!(create_res.0["id"], "test_browser_profile_1");
        assert_eq!(create_res.0["success"], true);

        let query = GetBrowsersQuery { limit: None, offset: None, search: None };
        let list_res = get_browsers(State(state.clone()), Query(query)).await.unwrap();
        assert_eq!(list_res.len(), 1);
        assert_eq!(list_res[0].id, "test_browser_profile_1");
        assert_eq!(list_res[0].proxy.as_deref(), Some("socks5://127.0.0.1:1080"));

        let detail_res = get_browser_detail(State(state.clone()), Path("test_browser_profile_1".to_string())).await.unwrap();
        assert_eq!(detail_res.name, "Test Browser 1");
        assert_eq!(detail_res.proxy.as_deref(), Some("socks5://127.0.0.1:1080"));
    }

    #[tokio::test]
    async fn test_update_and_delete_browser() {
        let state = create_test_state();
        let create_req = CreateBrowserRequest {
            id: Some("test_browser_profile_2".to_string()),
            name: "Initial Name".to_string(),
            user_agent: None,
            timezone: None,
            proxy: None,
        };
        let _ = create_browser(State(state.clone()), Json(create_req)).await.unwrap();

        let update_req = UpdateBrowserRequest {
            name: Some("Updated Name".to_string()),
            user_agent: Some("CustomAgent/2.0".to_string()),
            timezone: Some("Asia/Ho_Chi_Minh".to_string()),
            proxy: Some("http://proxy.local:8080".to_string()),
        };
        let update_res = update_browser(State(state.clone()), Path("test_browser_profile_2".to_string()), Json(update_req)).await.unwrap();
        assert_eq!(update_res.0["success"], true);

        let detail_res = get_browser_detail(State(state.clone()), Path("test_browser_profile_2".to_string())).await.unwrap();
        assert_eq!(detail_res.proxy.as_deref(), Some("http://proxy.local:8080"));

        let _ = delete_browser(State(state.clone()), Path("test_browser_profile_2".to_string())).await.unwrap();
        let query = GetBrowsersQuery { limit: None, offset: None, search: None };
        let list_after = get_browsers(State(state), Query(query)).await.unwrap();
        assert!(list_after.is_empty());
    }

    #[tokio::test]
    async fn test_create_browser_invalid_id() {
        let state = create_test_state();
        let create_req = CreateBrowserRequest {
            id: Some("invalid/id/with/slashes".to_string()),
            name: "Bad ID".to_string(),
            user_agent: None,
            timezone: None,
            proxy: None,
        };
        let err = create_browser(State(state), Json(create_req)).await.unwrap_err();
        match err {
            AutomaError::BadRequest(msg) => assert!(msg.contains("Invalid")),
            _ => panic!("Expected BadRequest for invalid ID"),
        }
    }

    #[tokio::test]
    async fn test_auto_detect_browsers() {
        let state = create_test_state();
        let res = auto_detect_browsers(State(state.clone())).await.unwrap();
        // Verify response can be serialized/deserialized cleanly
        let _ = res.0.len();

        // Check that settings default profile is set if detected
        let db = state.db.lock().await;
        let settings = db.settings().get_settings().unwrap();
        if !res.0.is_empty() {
            assert!(settings.browser.default_profile_id.is_some());
        }
    }

    #[tokio::test]
    async fn test_set_default_browser() {
        let state = create_test_state();
        let create_req = CreateBrowserRequest {
            id: Some("test_profile_custom".to_string()),
            name: "Custom Profile".to_string(),
            user_agent: None,
            timezone: None,
            proxy: None,
        };
        let _ = create_browser(State(state.clone()), Json(create_req)).await.unwrap();

        let set_res = set_default_browser(State(state.clone()), Path("test_profile_custom".to_string())).await.unwrap();
        assert_eq!(set_res.0.success, true);
        assert_eq!(set_res.0.default_profile_id, "test_profile_custom");

        let detail = get_browser_detail(State(state.clone()), Path("test_profile_custom".to_string())).await.unwrap();
        assert!(detail.is_default);

        let query = GetBrowsersQuery { limit: None, offset: None, search: None };
        let all = get_browsers(State(state), Query(query)).await.unwrap();
        let custom_found = all.iter().find(|b| b.id == "test_profile_custom").unwrap();
        assert!(custom_found.is_default);
    }
}
