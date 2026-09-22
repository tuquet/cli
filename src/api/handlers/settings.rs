use axum::{extract::State, Json};
use crate::AppState;
use crate::core::models::settings::{AppSettings, UpdateAppSettingsRequest};

#[utoipa::path(
    tag = "Settings",
    get,
    path = "/api/v1/system/settings",
    operation_id = "get_app_settings",
    summary = "Get current application and grid settings",
    description = "Retrieves current daemon configuration including Grid Matrix layout, default browser preferences, and Runner concurrency settings.",
    responses(
        (status = 200, description = "Current application and grid settings", body = AppSettings),
        (status = 500, description = "Database query error", body = crate::core::error::ApiErrorResponse)
    )
)]
pub async fn get_settings(
    State(state): State<AppState>,
) -> Result<Json<AppSettings>, crate::core::error::AutomaError> {
    let db = state.db.lock().await;
    let settings = db.settings().get_settings()?;
    Ok(Json(settings))
}

#[utoipa::path(
    tag = "Settings",
    put,
    path = "/api/v1/system/settings",
    operation_id = "update_app_settings",
    summary = "Overwrite application and grid settings",
    description = "Replaces the entire application configuration and persists new grid, browser, and runner settings.",
    request_body = AppSettings,
    responses(
        (status = 200, description = "Application settings replaced successfully", body = AppSettings),
        (status = 400, description = "Invalid settings payload", body = crate::core::error::ApiErrorResponse),
        (status = 500, description = "Database write error", body = crate::core::error::ApiErrorResponse)
    )
)]
pub async fn update_settings(
    State(state): State<AppState>,
    Json(payload): Json<AppSettings>,
) -> Result<Json<AppSettings>, crate::core::error::AutomaError> {
    let db = state.db.lock().await;
    db.settings().save_settings(&payload)?;
    Ok(Json(payload))
}

#[utoipa::path(
    tag = "Settings",
    patch,
    path = "/api/v1/system/settings",
    operation_id = "patch_app_settings",
    summary = "Partially update application settings",
    description = "Selectively updates specific sections of settings (e.g. grid matrix rows/columns or browser default type) without overwriting other properties.",
    request_body = UpdateAppSettingsRequest,
    responses(
        (status = 200, description = "Application settings patched successfully", body = AppSettings),
        (status = 400, description = "Invalid patch payload", body = crate::core::error::ApiErrorResponse),
        (status = 500, description = "Database update error", body = crate::core::error::ApiErrorResponse)
    )
)]
pub async fn patch_settings(
    State(state): State<AppState>,
    Json(patch): Json<UpdateAppSettingsRequest>,
) -> Result<Json<AppSettings>, crate::core::error::AutomaError> {
    let db = state.db.lock().await;
    let mut settings = db.settings().get_settings()?;
    settings.apply_patch(patch);
    db.settings().save_settings(&settings)?;
    Ok(Json(settings))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::db::AutomaDb;
    use crate::core::models::settings::*;
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use tokio::sync::broadcast;

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
    async fn test_get_default_settings() {
        let state = create_test_state();
        let Json(settings) = get_settings(State(state)).await.unwrap();
        assert_eq!(settings.grid.matrix.columns, 3);
        assert_eq!(settings.grid.matrix.rows, 2);
        assert_eq!(settings.grid.display.screen_width, 1920);
        assert!(settings.grid.enabled);
    }

    #[tokio::test]
    async fn test_patch_grid_settings() {
        let state = create_test_state();
        let patch = UpdateAppSettingsRequest {
            grid: Some(UpdateGridSettingsRequest {
                enabled: Some(true),
                matrix: Some(UpdateGridMatrixRequest {
                    columns: Some(4),
                    rows: Some(3),
                }),
                display: Some(UpdateDisplaySettingsRequest {
                    screen_width: Some(2560),
                    screen_height: Some(1440),
                    offset_x: None,
                    offset_y: None,
                    margin: Some(16),
                    monitor_index: Some(1),
                }),
                behavior: None,
            }),
            browser: None,
            runner: None,
        };

        let Json(updated) = patch_settings(State(state.clone()), Json(patch)).await.unwrap();
        assert_eq!(updated.grid.matrix.columns, 4);
        assert_eq!(updated.grid.matrix.rows, 3);
        assert_eq!(updated.grid.display.screen_width, 2560);
        assert_eq!(updated.grid.display.screen_height, 1440);
        assert_eq!(updated.grid.display.margin, 16);
        assert_eq!(updated.grid.display.monitor_index, 1);

        // Verify persistence
        let Json(fetched) = get_settings(State(state)).await.unwrap();
        assert_eq!(fetched.grid.matrix.columns, 4);
        assert_eq!(fetched.grid.display.screen_width, 2560);
    }

    #[tokio::test]
    async fn test_update_full_settings() {
        let state = create_test_state();
        let mut custom = AppSettings::default();
        custom.browser.default_type = "chromium".to_string();
        custom.runner.max_concurrent_jobs = 10;

        let Json(saved) = update_settings(State(state.clone()), Json(custom)).await.unwrap();
        assert_eq!(saved.browser.default_type, "chromium");
        assert_eq!(saved.runner.max_concurrent_jobs, 10);

        let Json(fetched) = get_settings(State(state)).await.unwrap();
        assert_eq!(fetched.browser.default_type, "chromium");
        assert_eq!(fetched.runner.max_concurrent_jobs, 10);
    }

    #[tokio::test]
    async fn test_patch_browser_default_profile_id() {
        let state = create_test_state();
        let patch = UpdateAppSettingsRequest {
            grid: None,
            browser: Some(UpdateBrowserSettingsRequest {
                default_type: None,
                default_profile_id: Some("profile_custom_99".to_string()),
                executable_path: None,
                headless: None,
                default_user_agent: None,
            }),
            runner: None,
        };

        let Json(updated) = patch_settings(State(state.clone()), Json(patch)).await.unwrap();
        assert_eq!(updated.browser.default_profile_id.as_deref(), Some("profile_custom_99"));

        let Json(fetched) = get_settings(State(state)).await.unwrap();
        assert_eq!(fetched.browser.default_profile_id.as_deref(), Some("profile_custom_99"));
    }
}
