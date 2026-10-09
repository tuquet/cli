use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Json,
};

use super::types::SystemResponse;

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
