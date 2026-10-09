use std::convert::Infallible;
use std::sync::Arc;
use axum::{
    extract::{Query, State},
    response::sse::{Event, Sse},
};
use futures::stream::Stream;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

use crate::core::browser::worker_coordinator::connected_browsers;
use crate::AppState;
use super::types::WorkerSseQuery;

#[utoipa::path(
    tag = "Jobs",
    get,
    path = "/api/v1/internal/worker/events",
    operation_id = "worker_sse",
    summary = "Subscribe to worker event stream (SSE)",
    description = "Establishes a long-lived Server-Sent Events (SSE) connection used by browser workers to receive workflow job dispatches and cancellation commands.",
    params(
        ("browserId" = Option<String>, Query, description = "Target browser instance identifier (defaults to 'daemon_worker')")
    ),
    responses(
        (status = 200, description = "SSE Stream for Worker jobs", content_type = "text/event-stream")
    )
)]
pub async fn worker_sse(
    State(state): State<AppState>,
    Query(query): Query<WorkerSseQuery>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = state.worker_tx.subscribe();
    let expected_browser = query.browser_id.unwrap_or_else(|| "daemon_worker".to_string());

    // Register the connected browser
    {
        let mut browsers = connected_browsers().write().await;
        browsers.insert(expected_browser.clone());
    }

    // Create a guard to unregister on drop
    struct BrowserGuard {
        browser_id: String,
    }
    impl Drop for BrowserGuard {
        fn drop(&mut self) {
            let browser_id = self.browser_id.clone();
            if let Ok(handle) = tokio::runtime::Handle::try_current() {
                handle.spawn(async move {
                    let mut browsers = connected_browsers().write().await;
                    browsers.remove(&browser_id);
                });
            }
        }
    }
    let _guard = Arc::new(BrowserGuard { browser_id: expected_browser.clone() });

    let stream = BroadcastStream::new(rx).filter_map(move |msg| {
        let _guard_clone = _guard.clone();
        let expected_browser = expected_browser.clone();
        let data = msg.ok()?;
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&data)
            && let Some(msg_browser) = json.get("browserId").and_then(|v| v.as_str())
            && msg_browser != expected_browser
        {
            return None;
        }
        Some(Ok(Event::default().data(data)))
    });

    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(std::time::Duration::from_secs(2))
    )
}
