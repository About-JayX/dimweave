use crate::daemon::{
    gui::{self, ClaudeStreamPayload},
    SharedState,
};
use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    response::IntoResponse,
};
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use tauri::AppHandle;
use tokio::sync::mpsc;

/// WS handler — Claude connects here via `--sdk-url ws://127.0.0.1:4502/claude`.
/// We use this WS to send NDJSON messages TO Claude.
pub async fn ws_handler(
    State((state, app)): State<(SharedState, AppHandle)>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_ws_connection(socket, state, app))
}

async fn handle_ws_connection(socket: WebSocket, state: SharedState, app: AppHandle) {
    let (mut sink, mut stream) = socket.split();
    let (tx, mut rx) = mpsc::channel::<String>(64);

    let epoch = {
        let mut s = state.write().await;
        let epoch = s.claude_sdk_epoch();
        if !s.attach_claude_sdk_ws(epoch, tx.clone()) {
            eprintln!("[ClaudeSDK] failed to attach WS — epoch mismatch");
            return;
        }
        // Signal the launch flow that WS is connected, passing the inject sender
        if let Some(ready_tx) = s.claude_sdk_ready_tx.take() {
            let _ = ready_tx.send(tx.clone());
        }
        epoch
    };

    gui::emit_agent_status(&app, "claude", true, None, None);
    gui::emit_system_log(&app, "info", "[ClaudeSDK] Claude connected via WS");

    // Forward outbound NDJSON to the WS sink
    let sink_task = tokio::spawn(async move {
        while let Some(ndjson) = rx.recv().await {
            if sink.send(Message::Text(ndjson)).await.is_err() {
                break;
            }
        }
    });

    // Read loop: handle incoming WS messages (keep-alive pings, etc.)
    while let Some(Ok(msg)) = stream.next().await {
        match msg {
            Message::Ping(data) => {
                // Pong is handled automatically by most WS impls, but log it
                eprintln!("[ClaudeSDK] received ping ({} bytes)", data.len());
            }
            Message::Close(_) => break,
            _ => {
                // In hybrid mode Claude sends output via HTTP POST, not WS.
                // Log unexpected WS messages for debugging.
                eprintln!("[ClaudeSDK] unexpected WS message from Claude");
            }
        }
    }

    sink_task.abort();

    // Clean up on disconnect
    {
        let mut s = state.write().await;
        s.clear_claude_sdk_ws(epoch);
    }
    gui::emit_claude_stream(&app, ClaudeStreamPayload::Reset);
    gui::emit_agent_status(&app, "claude", false, None, None);
    gui::emit_system_log(&app, "info", "[ClaudeSDK] Claude disconnected");
}

#[derive(Deserialize)]
struct EventsBody {
    events: Vec<serde_json::Value>,
}

/// HTTP POST handler — Claude POSTs events as `{"events": [...]}`.
pub async fn events_handler(
    State((state, app)): State<(SharedState, AppHandle)>,
    body: String,
) -> impl IntoResponse {
    let parsed: Result<EventsBody, _> = serde_json::from_str(&body);
    match parsed {
        Ok(body) => {
            for event in body.events {
                process_sdk_event(&state, &app, event).await;
            }
            axum::Json(serde_json::json!({"ok": true}))
        }
        Err(err) => {
            eprintln!("[ClaudeSDK] failed to parse events body: {err}");
            axum::Json(serde_json::json!({"ok": false, "error": err.to_string()}))
        }
    }
}

/// Dispatch a single Claude SDK event to the event handler module.
async fn process_sdk_event(
    state: &SharedState,
    app: &AppHandle,
    event: serde_json::Value,
) {
    let role = state.read().await.claude_role.clone();
    crate::daemon::claude_sdk::event_handler::handle_events(
        vec![event],
        &role,
        state.clone(),
        app.clone(),
    )
    .await;
}
