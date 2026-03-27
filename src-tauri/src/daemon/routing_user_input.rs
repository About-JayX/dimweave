use crate::daemon::{gui, routing, state::DaemonState, types::BridgeMessage, SharedState};
use tauri::AppHandle;

pub async fn route_user_input(
    state: &SharedState,
    app: &AppHandle,
    content: String,
    target: String,
) {
    if content.trim().is_empty() {
        gui::emit_system_log(app, "warn", "[Route] ignoring empty user input");
        return;
    }
    let targets = {
        let s = state.read().await;
        resolve_user_targets(&s, &target)
    };
    if targets.is_empty() {
        gui::emit_system_log(app, "warn", "[Route] no online targets for user input");
        return;
    }
    let display_to = if targets.len() == 1 {
        targets[0].clone()
    } else {
        target
    };
    let now = chrono::Utc::now().timestamp_millis() as u64;
    gui::emit_agent_message(app, &build_user_message(now, &display_to, &content));
    for role in targets {
        routing::route_message_silent(state, app, build_user_message(now, &role, &content)).await;
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

fn build_user_message(now: u64, to: &str, content: &str) -> BridgeMessage {
    let suffix = if to == "user" {
        String::new()
    } else {
        format!("_{to}")
    };
    BridgeMessage {
        id: format!("user_{now}{suffix}"),
        from: "user".into(),
        to: to.to_string(),
        content: content.to_string(),
        timestamp: now,
        reply_to: None,
        priority: None,
        status: None,
    }
}
