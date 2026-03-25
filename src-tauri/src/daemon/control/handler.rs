use crate::daemon::{
    gui, routing,
    types::{BridgeMessage, FromAgent, ToAgent},
    SharedState,
};
use axum::extract::ws::{Message, WebSocket};
use futures_util::{SinkExt, StreamExt};
use tauri::AppHandle;
use tokio::sync::mpsc;

pub async fn handle_connection(socket: WebSocket, state: SharedState, app: AppHandle) {
    let (mut sink, mut stream) = socket.split();
    let (tx, mut rx) = mpsc::channel::<BridgeMessage>(64);
    let mut agent_id: Option<String> = None;

    // Forward outbound messages to WS sink
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            let payload =
                serde_json::to_string(&ToAgent::RoutedMessage { message: msg }).unwrap();
            if sink.send(Message::Text(payload.into())).await.is_err() {
                break;
            }
        }
    });

    while let Some(Ok(msg)) = stream.next().await {
        let Message::Text(txt) = msg else { continue };
        let Ok(from_agent) = serde_json::from_str::<FromAgent>(&txt) else {
            continue;
        };

        match from_agent {
            FromAgent::AgentConnect { agent_id: id } => {
                agent_id = Some(id.clone());
                state
                    .write()
                    .await
                    .attached_agents
                    .insert(id.clone(), tx.clone());
                gui::emit_agent_status(&app, &id, true, None);
                gui::emit_system_log(&app, "info", &format!("[Control] {id} connected"));
            }
            FromAgent::AgentReply { message } => {
                routing::route_message(&state, &app, message).await;
            }
            FromAgent::AgentDisconnect => break,
        }
    }

    if let Some(id) = &agent_id {
        state.write().await.attached_agents.remove(id);
        gui::emit_agent_status(&app, id, false, None);
        gui::emit_system_log(&app, "info", &format!("[Control] {id} disconnected"));
    }
}
