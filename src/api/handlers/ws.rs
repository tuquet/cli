use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
};
use futures::{sink::SinkExt, stream::StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio_stream::wrappers::BroadcastStream;
use crate::AppState;

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum WsCommand {
    #[serde(rename = "PING")]
    Ping { timestamp: Option<u64> },
    #[serde(rename = "SUBSCRIBE_EVENTS")]
    SubscribeEvents { channels: Option<Vec<String>> },
    #[serde(rename = "KILL_JOB")]
    KillJob {
        #[serde(rename = "jobId")]
        job_id: String,
    },
    #[serde(rename = "PAUSE_JOB")]
    PauseJob {
        #[serde(rename = "jobId")]
        job_id: String,
    },
    #[serde(rename = "RESUME_JOB")]
    ResumeJob {
        #[serde(rename = "jobId")]
        job_id: String,
    },
}

#[utoipa::path(
    tag = "Events",
    get,
    path = "/api/v1/ws",
    operation_id = "ws_handler",
    summary = "Establish WebSocket connection for real-time control",
    description = "Low-latency bidirectional WebSocket connection consuming types from @automa/types/ws for PAUSE_JOB, RESUME_JOB, KILL_JOB, and live breakpoints.",
    responses(
        (status = 101, description = "WebSocket Upgrade for 2-way real-time communication")
    )
)]
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: AppState) {
    let (mut sender, mut receiver) = socket.split();
    let client_id = uuid::Uuid::new_v4().to_string();

    // 1. Send initial handshake greeting
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    let greeting = json!({
        "type": "CONNECTED",
        "serverVersion": env!("CARGO_PKG_VERSION"),
        "clientId": client_id,
        "timestamp": now
    });

    if let Err(e) = sender.send(Message::Text(greeting.to_string().into())).await {
        tracing::warn!("[WebSocket] Failed to send greeting: {:?}", e);
        return;
    }

    // 2. Subscribe to internal core event broadcast channel
    let mut broadcast_rx = BroadcastStream::new(state.tx.subscribe());

    // 3. Bidirectional loop (select!)
    loop {
        tokio::select! {
            // Forward internal broadcast events to WebSocket client
            Some(Ok(event_str)) = broadcast_rx.next() => {
                if let Err(e) = sender.send(Message::Text(event_str.into())).await {
                    tracing::debug!("[WebSocket] Client disconnected on event push: {:?}", e);
                    break;
                }
            }

            // Receive and process incoming client commands
            Some(msg_result) = receiver.next() => {
                match msg_result {
                    Ok(Message::Text(text)) => {
                        let parsed: Result<WsCommand, _> = serde_json::from_str(&text);
                        match parsed {
                            Ok(WsCommand::Ping { timestamp }) => {
                                let pong_ts = timestamp.unwrap_or_else(|| {
                                    SystemTime::now()
                                        .duration_since(UNIX_EPOCH)
                                        .unwrap_or_default()
                                        .as_millis() as u64
                                });
                                let reply = json!({
                                    "type": "PONG",
                                    "timestamp": pong_ts
                                });
                                if sender.send(Message::Text(reply.to_string().into())).await.is_err() {
                                    break;
                                }
                            }
                            Ok(WsCommand::KillJob { job_id }) => {
                                let mut jobs = state.active_jobs.write().await;
                                if let Some(handle) = jobs.remove(&job_id) {
                                    handle.cancel();
                                    let _ = sender.send(Message::Text(json!({
                                        "type": "JOB_STATUS_CHANGED",
                                        "jobId": job_id,
                                        "status": "stopped"
                                    }).to_string().into())).await;
                                }
                            }
                            Ok(WsCommand::SubscribeEvents { .. }) => {
                                // Default subscription is active via BroadcastStream
                                let _ = sender.send(Message::Text(json!({
                                    "type": "SUBSCRIPTION_CONFIRMED",
                                    "status": "subscribed"
                                }).to_string().into())).await;
                            }
                            Ok(cmd) => {
                                tracing::info!("[WebSocket] Received command: {:?}", cmd);
                            }
                            Err(e) => {
                                let _ = sender.send(Message::Text(json!({
                                    "type": "ERROR",
                                    "code": "INVALID_COMMAND",
                                    "message": format!("Failed to parse command: {}", e)
                                }).to_string().into())).await;
                            }
                        }
                    }
                    Ok(Message::Ping(payload)) => {
                        if sender.send(Message::Pong(payload)).await.is_err() {
                            break;
                        }
                    }
                    Ok(Message::Close(_)) => {
                        tracing::debug!("[WebSocket] Client closed connection");
                        break;
                    }
                    Err(e) => {
                        tracing::debug!("[WebSocket] Client connection error: {:?}", e);
                        break;
                    }
                    _ => {}
                }
            }
            else => break,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ws_ping_command() {
        let json_str = r#"{"type": "PING", "timestamp": 123456}"#;
        let cmd: WsCommand = serde_json::from_str(json_str).expect("Should parse PING");
        match cmd {
            WsCommand::Ping { timestamp } => assert_eq!(timestamp, Some(123456)),
            _ => panic!("Expected Ping variant"),
        }
    }

    #[test]
    fn test_parse_ws_kill_job_command() {
        let json_str = r#"{"type": "KILL_JOB", "jobId": "job-abc"}"#;
        let cmd: WsCommand = serde_json::from_str(json_str).expect("Should parse KILL_JOB");
        match cmd {
            WsCommand::KillJob { job_id } => assert_eq!(job_id, "job-abc"),
            _ => panic!("Expected KillJob variant"),
        }
    }
}
