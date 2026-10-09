use std::net::SocketAddr;
use std::sync::Arc;
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use tower_http::cors::{Any, CorsLayer};

use crate::commands::inbox::parser::EmailParser;
use crate::commands::inbox::storage::{
    generate_uuid, now_rfc3339, EmailRecord, InboxStorage, LinkRecord, OtpRecord,
};
use crate::config::inbox::InboxConfig;

pub struct InboxServerState {
    pub storage: Arc<InboxStorage>,
    pub secret: String,
}

#[derive(Debug, Deserialize)]
pub struct WebhookPayload {
    pub id: Option<String>,
    pub from: String,
    pub to: String,
    pub subject: Option<String>,
    pub body_text: Option<String>,
    pub body_html: Option<String>,
    pub headers: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct WebhookResponse {
    pub ok: bool,
    pub email_id: String,
    pub otps_extracted: usize,
    pub links_extracted: usize,
    pub detected_service: Option<String>,
}

pub fn create_inbox_router(state: Arc<InboxServerState>) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/health", get(health_handler))
        .route("/status", get(status_handler))
        .route("/webhook", post(webhook_handler))
        .route("/api/inbox/webhook", post(webhook_handler))
        .route("/api/inbox/otp/{recipient}", get(get_otp_handler))
        .route("/api/inbox/link/{recipient}", get(get_link_handler))
        .route("/api/inbox/stats", get(stats_handler))
        .layer(cors)
        .with_state(state)
}

async fn health_handler() -> impl IntoResponse {
    (StatusCode::OK, Json(serde_json::json!({
        "status": "healthy",
        "service": "specter-inbox",
        "version": env!("CARGO_PKG_VERSION")
    })))
}

async fn status_handler(State(state): State<Arc<InboxServerState>>) -> impl IntoResponse {
    match state.storage.stats() {
        Ok(stats) => (StatusCode::OK, Json(serde_json::json!({
            "status": "online",
            "stats": stats
        }))),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
            "status": "error",
            "message": e.to_string()
        }))),
    }
}

async fn stats_handler(State(state): State<Arc<InboxServerState>>) -> impl IntoResponse {
    match state.storage.stats() {
        Ok(stats) => (StatusCode::OK, Json(serde_json::json!(stats))),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
            "error": e.to_string()
        }))),
    }
}

async fn webhook_handler(
    State(state): State<Arc<InboxServerState>>,
    headers: HeaderMap,
    Json(payload): Json<WebhookPayload>,
) -> impl IntoResponse {
    // Optional secret verification
    if !state.secret.is_empty() {
        let provided_secret = headers
            .get("X-Specter-Secret")
            .or_else(|| headers.get("x-specter-secret"))
            .and_then(|v| v.to_str().ok());

        if provided_secret != Some(&state.secret) {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({
                    "ok": false,
                    "error": "Invalid or missing X-Specter-Secret header"
                })),
            );
        }
    }

    let email_id = payload.id.unwrap_or_else(generate_uuid);
    let domain = payload
        .to
        .split('@')
        .nth(1)
        .unwrap_or("unknown")
        .to_string();

    let email_rec = EmailRecord {
        id: email_id.clone(),
        message_id: None,
        sender: payload.from.clone(),
        recipient: payload.to.clone(),
        domain,
        subject: payload.subject.clone(),
        body_text: payload.body_text.clone(),
        body_html: payload.body_html.clone(),
        received_at: now_rfc3339(),
        raw_headers: payload.headers.map(|h| h.to_string()),
        created_at: now_rfc3339(),
    };

    if let Err(e) = state.storage.save_email(&email_rec) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "ok": false,
                "error": format!("Failed to save email: {}", e)
            })),
        );
    }

    // Parse email for OTPs and Links
    let parsed = EmailParser::parse(
        &payload.from,
        payload.subject.as_deref(),
        payload.body_text.as_deref(),
        payload.body_html.as_deref(),
    );

    let otps_count = parsed.otps.len();
    let links_count = parsed.links.len();

    for otp in parsed.otps {
        let otp_rec = OtpRecord {
            id: generate_uuid(),
            email_id: email_id.clone(),
            recipient: payload.to.clone(),
            otp_code: otp.code,
            service_name: otp.service.or(parsed.detected_service.clone()),
            expires_at: None,
            consumed_at: None,
            consumed_by: None,
            created_at: now_rfc3339(),
        };
        let _ = state.storage.save_otp(&otp_rec);
    }

    for link in parsed.links {
        let link_rec = LinkRecord {
            id: generate_uuid(),
            email_id: email_id.clone(),
            recipient: payload.to.clone(),
            url: link.url,
            link_type: link.link_type,
            created_at: now_rfc3339(),
        };
        let _ = state.storage.save_link(&link_rec);
    }

    (
        StatusCode::OK,
        Json(serde_json::json!(WebhookResponse {
            ok: true,
            email_id,
            otps_extracted: otps_count,
            links_extracted: links_count,
            detected_service: parsed.detected_service,
        })),
    )
}

async fn get_otp_handler(
    State(state): State<Arc<InboxServerState>>,
    Path(recipient): Path<String>,
) -> impl IntoResponse {
    match state.storage.get_latest_otp(&recipient, true) {
        Ok(Some(otp)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "ok": true,
                "otp": otp.otp_code,
                "service": otp.service_name,
                "created_at": otp.created_at,
            })),
        ),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "ok": false,
                "error": format!("No active unconsumed OTP found for '{}'", recipient)
            })),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "ok": false,
                "error": e.to_string()
            })),
        ),
    }
}

async fn get_link_handler(
    State(state): State<Arc<InboxServerState>>,
    Path(recipient): Path<String>,
) -> impl IntoResponse {
    match state.storage.get_latest_link(&recipient, None) {
        Ok(Some(link)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "ok": true,
                "url": link.url,
                "type": link.link_type,
                "created_at": link.created_at,
            })),
        ),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "ok": false,
                "error": format!("No verification or action link found for '{}'", recipient)
            })),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "ok": false,
                "error": e.to_string()
            })),
        ),
    }
}

pub fn get_pid_path() -> std::path::PathBuf {
    crate::config::expander::canonical_specter_dir()
        .join(crate::constants::PILLAR_INBOX)
        .join("inbox.pid")
}

pub fn write_pid(pid: u32) -> std::io::Result<()> {
    let path = get_pid_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, pid.to_string())
}

pub fn remove_pid() {
    let path = get_pid_path();
    let _ = std::fs::remove_file(path);
}

pub fn read_pid() -> Option<u32> {
    let path = get_pid_path();
    if path.exists() {
        let s = std::fs::read_to_string(path).ok()?;
        s.trim().parse::<u32>().ok()
    } else {
        None
    }
}

pub async fn run_server(config: InboxConfig) -> Result<(), Box<dyn std::error::Error>> {
    let storage = Arc::new(InboxStorage::open(InboxConfig::sqlite_path())?);
    let state = Arc::new(InboxServerState {
        storage,
        secret: config.webhook_secret.clone().unwrap_or_default(),
    });

    let app = create_inbox_router(state);
    let addr: SocketAddr = format!("{}:{}", config.host, config.port).parse()?;

    println!("⚡ Specter Inbox Webhook Daemon listening on http://{}", addr);
    write_pid(std::process::id())?;

    let listener = tokio::net::TcpListener::bind(addr).await?;
    
    // Run with graceful shutdown on Ctrl-C
    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
            println!("\nShutting down Specter Inbox daemon...");
        })
        .await?;

    remove_pid();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tower::ServiceExt;
    use axum::http::Request;

    #[tokio::test]
    async fn test_inbox_api_router() {
        let storage = Arc::new(InboxStorage::open_in_memory().unwrap());
        let state = Arc::new(InboxServerState {
            storage,
            secret: "test-secret".to_string(),
        });

        let app = create_inbox_router(state);

        // 1. Test /health
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(res.status(), StatusCode::OK);

        // 2. Test /webhook with valid secret
        let webhook_payload = serde_json::json!({
            "from": "security@facebookmail.com",
            "to": "target_acc@domain.com",
            "subject": "Your Facebook confirmation code is 519283",
            "body_text": "Enter code 519283 or click https://facebook.com/confirm?code=519283"
        });

        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/webhook")
                    .header("Content-Type", "application/json")
                    .header("X-Specter-Secret", "test-secret")
                    .body(axum::body::Body::from(serde_json::to_vec(&webhook_payload).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(res.status(), StatusCode::OK);

        // 3. Test /api/inbox/otp/target_acc@domain.com
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/inbox/otp/target_acc@domain.com")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(res.status(), StatusCode::OK);

        // 4. Test /api/inbox/link/target_acc@domain.com
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/inbox/link/target_acc@domain.com")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(res.status(), StatusCode::OK);
    }
}
