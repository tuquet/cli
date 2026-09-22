use axum::{
    extract::State,
    response::sse::{Event, Sse},
};
use futures::stream::Stream;
use futures::StreamExt;
use std::convert::Infallible;
use tokio_stream::wrappers::BroadcastStream;

#[utoipa::path(
    tag = "Events",
    get,
    path = "/api/v1/events",
    operation_id = "subscribe_events_sse",
    summary = "Subscribe to global telemetry and logs SSE stream",
    description = "Establishes a real-time Server-Sent Events stream for task progression, job lifecycle changes, logs, and matrix events.",
    responses(
        (status = 200, description = "Server-Sent Events stream for task logs and progress", content_type = "text/event-stream")
    )
)]
pub async fn sse(
    State(state): State<crate::AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let receiver = state.tx.subscribe();
    
    let stream = BroadcastStream::new(receiver)
        .filter_map(|res| async {
            match res {
                Ok(msg) => Some(Ok(Event::default().data(msg))),
                Err(_) => None, // Handle lag by simply dropping/ignoring
            }
        });

    Sse::new(stream).keep_alive(axum::response::sse::KeepAlive::new())
}
