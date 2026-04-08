use crate::daemon::claude_sdk::protocol::PostEventsBody as EventsBody;
use crate::daemon::{gui::{self, ClaudeStreamPayload}, SharedState};
use axum::{
    extract::{
        ws::{Message, WebSocket},
        Query, State, WebSocketUpgrade,
    },
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use futures_util::{SinkExt, StreamExt};
use tauri::AppHandle;
use tokio::sync::mpsc;
use tracing::{error, info, warn};

#[path = "claude_sdk_handler_nonce.rs"]
mod nonce;
#[path = "claude_sdk_handler_processing.rs"]
mod processing;
#[path = "claude_sdk_handler_reconnect.rs"]
mod reconnect;
use nonce::{current_launch_nonce, launch_nonce_error_response, LaunchNonceQuery};
pub(crate) use processing::process_sdk_events;
pub(crate) use reconnect::{reconnect_delay_ms, MAX_WS_RECONNECT_ATTEMPTS};
use processing::summarize_events_batch;

pub async fn ws_handler(
    State((state, app)): State<(SharedState, AppHandle)>,
    Query(query): Query<LaunchNonceQuery>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    let launch_nonce = match current_launch_nonce(&state, &query).await {
        Ok(launch_nonce) => launch_nonce,
        Err(err) => return launch_nonce_error_response(&app, "WS", &query, err),
    };
    ws.on_upgrade(move |socket| handle_ws_connection(socket, launch_nonce, state, app))
}

async fn handle_ws_connection(
    socket: WebSocket,
    launch_nonce: String,
    state: SharedState,
    app: AppHandle,
) {
    let (mut sink, mut stream) = socket.split();
    let (tx, mut rx) = mpsc::channel::<String>(64);
    let trace_nonce = crate::daemon::claude_sdk::process::redact_launch_nonce(&launch_nonce);

    let (epoch, ws_generation) = {
        let mut s = state.write().await;
        let epoch = s.claude_sdk_epoch();
        let Some(ws_generation) = s.attach_claude_sdk_ws(epoch, &launch_nonce, tx.clone()) else {
            warn!(epoch, launch_nonce = %trace_nonce, "failed to attach WS: epoch/nonce mismatch");
            return;
        };
        if let Some(ready_tx) = s.claude_sdk_ready_tx.take() {
            let _ = ready_tx.send(tx.clone());
        }
        (epoch, ws_generation)
    };

    gui::emit_agent_status(&app, "claude", true, None, None);
    gui::emit_system_log(&app, "info", "[ClaudeSDK] Claude connected via WS");
    gui::emit_system_log(
        &app,
        "info",
        &format!(
            "[Claude Trace] chain=ws_connected epoch={} launch_nonce={} ws_generation={} ws={} events={} direction=daemon->claude:ws_ndjson claude->daemon:http_post",
            epoch,
            trace_nonce,
            ws_generation,
            crate::daemon::claude_sdk::process::sdk_ws_url(crate::daemon::ports::PortConfig::from_env().daemon, None),
            crate::daemon::claude_sdk::process::sdk_events_url(crate::daemon::ports::PortConfig::from_env().daemon, None),
        ),
    );

    let sink_task = tokio::spawn(async move {
        while let Some(ndjson) = rx.recv().await {
            if sink.send(Message::Text(ndjson)).await.is_err() {
                break;
            }
        }
    });

    while let Some(Ok(msg)) = stream.next().await {
        match msg {
            Message::Ping(data) => {
                info!(bytes = data.len(), "received Claude SDK ping");
            }
            Message::Close(_) => break,
            _ => {
                warn!("unexpected WS message from Claude");
            }
        }
    }

    sink_task.abort();

    let cleared = {
        let mut s = state.write().await;
        s.clear_claude_sdk_ws(epoch, &launch_nonce, ws_generation)
    };
    if !cleared {
        gui::emit_system_log(
            &app,
            "info",
            &format!(
                "[Claude Trace] chain=ws_disconnected_ignored epoch={} launch_nonce={} ws_generation={}",
                epoch, trace_nonce, ws_generation
            ),
        );
        return;
    }
    gui::emit_claude_stream(&app, ClaudeStreamPayload::Reset);
    gui::emit_agent_status(&app, "claude", false, None, None);
    gui::emit_system_log(&app, "info", "[ClaudeSDK] Claude disconnected");
    gui::emit_system_log(
        &app,
        "info",
        &format!(
            "[Claude Trace] chain=ws_disconnected epoch={} launch_nonce={} ws_generation={}",
            epoch, trace_nonce, ws_generation
        ),
    );
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EventEnqueueError {
    QueueUnavailable,
    QueueClosed,
}

pub async fn events_handler(
    State((state, app)): State<(SharedState, AppHandle)>,
    Query(query): Query<LaunchNonceQuery>,
    body: String,
) -> impl IntoResponse {
    let launch_nonce = match current_launch_nonce(&state, &query).await {
        Ok(launch_nonce) => launch_nonce,
        Err(err) => return launch_nonce_error_response(&app, "HTTP_POST", &query, err),
    };
    let parsed: Result<EventsBody, _> = serde_json::from_str(&body);
    match parsed {
        Ok(body) => {
            let trace_nonce =
                crate::daemon::claude_sdk::process::redact_launch_nonce(&launch_nonce);
            gui::emit_system_log(
                &app,
                "info",
                &format!(
                    "[Claude Trace] chain=http_post launch_nonce={} {}",
                    trace_nonce,
                    summarize_events_batch(&body)
                ),
            );
            match enqueue_events(&state, body.events).await {
                Ok(()) => (StatusCode::OK, Json(serde_json::json!({"ok": true}))).into_response(),
                Err(EventEnqueueError::QueueUnavailable) => (
                    StatusCode::SERVICE_UNAVAILABLE,
                    Json(
                        serde_json::json!({"ok": false, "error": "claude event queue unavailable"}),
                    ),
                )
                    .into_response(),
                Err(EventEnqueueError::QueueClosed) => (
                    StatusCode::SERVICE_UNAVAILABLE,
                    Json(serde_json::json!({"ok": false, "error": "claude event queue closed"})),
                )
                    .into_response(),
            }
        }
        Err(err) => {
            error!(error = %err, "failed to parse Claude SDK events body");
            (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"ok": false, "error": err.to_string()})),
            )
                .into_response()
        }
    }
}

async fn enqueue_events(state: &SharedState, events: Vec<serde_json::Value>) -> Result<(), EventEnqueueError> {
    let tx = state
        .read()
        .await
        .claude_sdk_event_tx
        .clone()
        .ok_or(EventEnqueueError::QueueUnavailable)?;
    tx.send(events)
        .await
        .map_err(|_| EventEnqueueError::QueueClosed)
}

#[cfg(test)]
#[path = "claude_sdk_handler_tests.rs"]
mod tests;
