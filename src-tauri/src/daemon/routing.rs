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
    let mut s = state.write().await;
    let target_agent = if s.claude_role == msg.to {
        Some("claude".to_string())
    } else if s.codex_role == msg.to {
        Some("codex".to_string())
    } else {
        None
    };

    if let Some(agent_id) = target_agent {
        if let Some(tx) = s.attached_agents.get(&agent_id) {
            if tx.send(msg.clone()).await.is_ok() {
                return RouteResult::Delivered;
            }
        }
    }
    s.buffer_message(msg);
    RouteResult::Buffered
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
