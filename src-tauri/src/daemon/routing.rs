use crate::daemon::{gui, types::BridgeMessage, SharedState};
use tauri::AppHandle;

pub enum RouteResult {
    Delivered,
    Buffered,
    ToGui,
}

pub async fn route_message_inner(state: &SharedState, msg: BridgeMessage) -> RouteResult {
    if msg.to == "user" {
        return RouteResult::ToGui;
    }
    enum Target {
        Claude(tokio::sync::mpsc::Sender<BridgeMessage>),
        Codex(tokio::sync::mpsc::Sender<String>, String),
        Buffer,
    }

    let target = {
        let mut s = state.write().await;

        if s.claude_role == msg.to {
            if let Some(tx) = s.attached_agents.get("claude") {
                Target::Claude(tx.clone())
            } else {
                s.buffer_message(msg.clone());
                Target::Buffer
            }
        } else if s.codex_role == msg.to {
            if let Some(tx) = s.codex_inject_tx.clone() {
                Target::Codex(tx, format_codex_input(&msg))
            } else {
                s.buffer_message(msg.clone());
                Target::Buffer
            }
        } else {
            s.buffer_message(msg.clone());
            Target::Buffer
        }
    };

    match target {
        Target::Claude(tx) => {
            if tx.send(msg.clone()).await.is_ok() {
                RouteResult::Delivered
            } else {
                state.write().await.buffer_message(msg);
                RouteResult::Buffered
            }
        }
        Target::Codex(tx, input) => {
            if tx.send(input).await.is_ok() {
                RouteResult::Delivered
            } else {
                state.write().await.buffer_message(msg);
                RouteResult::Buffered
            }
        }
        Target::Buffer => RouteResult::Buffered,
    }
}

pub async fn route_message(state: &SharedState, app: &AppHandle, msg: BridgeMessage) {
    gui::emit_agent_message(app, &msg);
    let result = route_message_inner(state, msg.clone()).await;
    match result {
        RouteResult::Delivered => {
            gui::emit_system_log(
                app,
                "info",
                &format!("[Route] {} → {} delivered", msg.from, msg.to),
            );
        }
        RouteResult::Buffered => {
            gui::emit_system_log(
                app,
                "warn",
                &format!("[Route] {} offline, buffered", msg.to),
            );
        }
        RouteResult::ToGui => {}
    }
}

pub fn format_codex_input(msg: &BridgeMessage) -> String {
    if msg.from == "user" {
        msg.content.clone()
    } else {
        format!("Message from {}:\n{}", msg.from, msg.content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::daemon::{state::DaemonState, types::BridgeMessage};
    use std::sync::Arc;
    use tokio::sync::RwLock;

    #[tokio::test]
    async fn route_to_offline_agent_buffers() {
        let state = Arc::new(RwLock::new(DaemonState::new()));
        let msg = BridgeMessage::system("hello", "lead");
        let result = route_message_inner(&state, msg).await;
        assert!(matches!(result, RouteResult::Buffered));
        assert_eq!(state.read().await.buffered_messages.len(), 1);
    }

    #[tokio::test]
    async fn route_to_user_returns_to_gui() {
        let state = Arc::new(RwLock::new(DaemonState::new()));
        let msg = BridgeMessage::system("hello", "user");
        let result = route_message_inner(&state, msg).await;
        assert!(matches!(result, RouteResult::ToGui));
    }
}
