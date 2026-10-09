use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use std::process::Command;

use super::types::SystemResponse;

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
        let _ = Command::new("cmd").args(["/C", "start", "", &url]).spawn();
        
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
