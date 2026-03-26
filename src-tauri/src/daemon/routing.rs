use crate::daemon::{
    gui,
    types::{BridgeMessage, ToAgent},
    SharedState,
};
use tauri::AppHandle;

pub enum RouteResult {
    Delivered,
    Buffered,
    Dropped,
    ToGui,
}

pub async fn route_message_inner(state: &SharedState, msg: BridgeMessage) -> RouteResult {
    if msg.to == "user" {
        return RouteResult::ToGui;
    }

    enum Target {
        Claude(tokio::sync::mpsc::Sender<ToAgent>),
        Codex(tokio::sync::mpsc::Sender<String>, String),
        NeedBuffer,
    }

    // First try with read lock — avoids write contention on the hot path
    let target = {
        let s = state.read().await;
        if s.claude_role == msg.to {
            if msg.from != "user" && msg.from != "system" && msg.from != s.codex_role {
                return RouteResult::Dropped;
            }
            if let Some(tx) = s.attached_agents.get("claude") {
                Target::Claude(tx.clone())
            } else {
                Target::NeedBuffer
            }
        } else if s.codex_role == msg.to {
            if let Some(tx) = s.codex_inject_tx.clone() {
                Target::Codex(tx, format_codex_input(&msg))
            } else {
                Target::NeedBuffer
            }
        } else {
            Target::NeedBuffer
        }
    };

    match target {
        Target::Claude(tx) => {
            if tx
                .send(ToAgent::RoutedMessage {
                    message: msg.clone(),
                })
                .await
                .is_ok()
            {
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
        Target::NeedBuffer => {
            state.write().await.buffer_message(msg);
            RouteResult::Buffered
        }
    }
}

pub async fn route_message(state: &SharedState, app: &AppHandle, msg: BridgeMessage) {
    gui::emit_agent_message(app, &msg);
    let result = route_message_inner(state, msg.clone()).await;
    let tag = match &result {
        RouteResult::Delivered => "delivered",
        RouteResult::Buffered => "buffered",
        RouteResult::Dropped => "dropped",
        RouteResult::ToGui => "gui",
    };
    eprintln!("[Route] {} → {} {tag}", msg.from, msg.to);
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
        RouteResult::Dropped => {
            gui::emit_system_log(
                app,
                "warn",
                &format!("[Route] dropped unauthorized Claude sender {}", msg.from),
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

    #[tokio::test]
    async fn route_to_claude_from_unknown_sender_drops() {
        let state = Arc::new(RwLock::new(DaemonState::new()));
        let msg = BridgeMessage {
            id: "msg-1".into(),
            from: "intruder".into(),
            to: "lead".into(),
            content: "hello".into(),
            timestamp: 1,
            reply_to: None,
            priority: None,
        };
        let result = route_message_inner(&state, msg).await;
        assert!(matches!(result, RouteResult::Dropped));
    }
}
