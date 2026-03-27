use crate::daemon::{
    gui,
    state::DaemonState,
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
        Codex(tokio::sync::mpsc::Sender<(String, bool)>, String, bool),
        NeedBuffer,
    }

    // First try with read lock — avoids write contention on the hot path
    let target = {
        let s = state.read().await;
        if s.claude_role == msg.to {
            if msg.from != "user" && msg.from != "system" && msg.from != s.codex_role {
                return RouteResult::Dropped;
            }
            if let Some(agent) = s.attached_agents.get("claude") {
                Target::Claude(agent.tx.clone())
            } else {
                Target::NeedBuffer
            }
        } else if s.codex_role == msg.to {
            if let Some(tx) = s.codex_inject_tx.clone() {
                let from_user = msg.from == "user";
                Target::Codex(tx, format_codex_input(&msg), from_user)
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
        Target::Codex(tx, input, from_user) => {
            if tx.send((input, from_user)).await.is_ok() {
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
    route_message_with_display(state, app, msg, true).await;
}

pub async fn route_message_silent(state: &SharedState, app: &AppHandle, msg: BridgeMessage) {
    route_message_with_display(state, app, msg, false).await;
}

async fn route_message_with_display(
    state: &SharedState,
    app: &AppHandle,
    msg: BridgeMessage,
    display_in_gui: bool,
) {
    let result = route_message_inner(state, msg.clone()).await;
    if display_in_gui && !matches!(result, RouteResult::Dropped) {
        gui::emit_agent_message(app, &msg);
    }
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

pub async fn route_user_input(
    state: &SharedState,
    app: &AppHandle,
    content: String,
    target: String,
) {
    let targets = {
        let s = state.read().await;
        resolve_user_targets(&s, &target)
    };
    let display_to = if targets.len() == 1 {
        targets[0].clone()
    } else {
        target
    };
    let now = chrono::Utc::now().timestamp_millis() as u64;
    let echo = BridgeMessage {
        id: format!("user_{now}"),
        from: "user".into(),
        to: display_to,
        content: content.clone(),
        timestamp: now,
        reply_to: None,
        priority: None,
    };
    gui::emit_agent_message(app, &echo);
    if targets.is_empty() {
        gui::emit_system_log(app, "warn", "[Route] no online targets for user input");
    }
    for role in targets {
        let msg = BridgeMessage {
            id: format!("user_{now}_{role}"),
            from: "user".into(),
            to: role,
            content: content.clone(),
            timestamp: now,
            reply_to: None,
            priority: None,
        };
        route_message_silent(state, app, msg).await;
    }
}

/// "auto" → online agent roles (deduplicated, excludes "user"); otherwise the literal role.
pub fn resolve_user_targets(state: &DaemonState, target: &str) -> Vec<String> {
    if target != "auto" {
        return vec![target.to_string()];
    }
    let mut targets = Vec::with_capacity(2);
    let claude_online = state.attached_agents.contains_key("claude");
    let codex_online = state.codex_inject_tx.is_some();
    if claude_online && state.claude_role != "user" {
        targets.push(state.claude_role.clone());
    }
    if codex_online && state.codex_role != "user" && !targets.contains(&state.codex_role) {
        targets.push(state.codex_role.clone());
    }
    targets
}

pub fn format_codex_input(msg: &BridgeMessage) -> String {
    if msg.from == "user" {
        msg.content.clone()
    } else {
        format!("Message from {}:\n{}", msg.from, msg.content)
    }
}

#[cfg(test)] #[path = "routing_tests.rs"]
mod tests;
