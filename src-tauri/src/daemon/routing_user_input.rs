use crate::daemon::{
    gui, routing,
    state::DaemonState,
    types::{Attachment, BridgeMessage},
    SharedState,
};
use tauri::AppHandle;

pub async fn route_user_input(
    state: &SharedState,
    app: &AppHandle,
    content: String,
    target: String,
    attachments: Option<Vec<Attachment>>,
    explicit_task_id: Option<String>,
) {
    if !has_user_input_payload(&content, &attachments) {
        gui::emit_system_log(app, "warn", "[Route] ignoring empty user input");
        return;
    }
    let targets = {
        let s = state.read().await;
        match explicit_task_id.as_deref() {
            Some(tid) if s.task_graph.get_task(tid).is_some() => {
                resolve_user_targets_for_task(&s, &target, tid)
            }
            _ => resolve_user_targets(&s, &target),
        }
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
    let mut display_msg = build_user_message(now, &display_to, &content, &attachments);
    {
        let s = state.read().await;
        stamp_user_message(&s, explicit_task_id.as_deref(), &mut display_msg);
    }
    gui::emit_agent_message(app, &display_msg);
    for role in targets {
        let mut msg = build_user_message(now, &role, &content, &attachments);
        {
            let s = state.read().await;
            stamp_user_message(&s, explicit_task_id.as_deref(), &mut msg);
        }
        routing::route_message_silent(state, app, msg).await;
    }
}

/// Stamp a user message with task context.  When the frontend supplied an
/// explicit `task_id`, use task-specific stamping so the daemon's
/// `active_task_id` is never mutated as a send side-effect.
fn stamp_user_message(
    s: &DaemonState,
    explicit_task_id: Option<&str>,
    msg: &mut BridgeMessage,
) {
    match explicit_task_id {
        Some(tid) if s.task_graph.get_task(tid).is_some() => {
            s.stamp_message_context_for_task(tid, "user", msg);
        }
        _ => {
            s.stamp_message_context("user", msg);
        }
    }
}

fn has_user_input_payload(content: &str, attachments: &Option<Vec<Attachment>>) -> bool {
    !content.trim().is_empty() || attachments.as_ref().is_some_and(|atts| !atts.is_empty())
}

/// Resolve targets using task_agents[] as primary truth for a specific task.
/// "lead" is promoted to first position when present (AC3).
/// Falls back to singleton fields for pre-migration tasks without task_agents.
pub fn resolve_user_targets_for_task(
    state: &DaemonState,
    target: &str,
    task_id: &str,
) -> Vec<String> {
    if target != "auto" {
        return vec![target.to_string()];
    }
    let agents = state.task_graph.agents_for_task(task_id);
    if agents.is_empty() {
        // Fallback to singleton-based resolution for pre-migration tasks
        return resolve_user_targets_for_task_legacy(state, task_id);
    }
    // Collect unique roles preserving agent order
    let mut roles: Vec<String> = Vec::new();
    for a in &agents {
        if !roles.contains(&a.role) {
            roles.push(a.role.clone());
        }
    }
    // Promote "lead" to first position (AC3)
    if let Some(idx) = roles.iter().position(|r| r == "lead") {
        if idx != 0 {
            roles.swap(0, idx);
        }
    }
    // Keep only roles that have at least one online provider
    roles
        .into_iter()
        .filter(|role| {
            state
                .resolve_task_role_providers(task_id, role)
                .iter()
                .any(|m| state.is_task_agent_online(task_id, m.runtime))
        })
        .collect()
}

/// Legacy auto-target for tasks without task_agents records.
fn resolve_user_targets_for_task_legacy(
    state: &DaemonState,
    task_id: &str,
) -> Vec<String> {
    let Some(task) = state.task_graph.get_task(task_id) else {
        return vec![];
    };
    let mut targets = Vec::with_capacity(2);
    if let Some(preferred) = crate::daemon::orchestrator::task_flow::preferred_auto_target(task) {
        let agent = state.resolve_task_provider_agent(task_id, &preferred);
        if agent.map_or(false, |a| state.is_task_agent_online(task_id, a)) {
            targets.push(preferred);
        }
    }
    let lead_agent = state.resolve_task_provider_agent(task_id, "lead");
    let coder_agent = state.resolve_task_provider_agent(task_id, "coder");
    if let Some(agent) = lead_agent {
        if state.is_task_agent_online(task_id, agent) && !targets.contains(&"lead".to_string()) {
            targets.push("lead".into());
        }
    }
    if let Some(agent) = coder_agent {
        if state.is_task_agent_online(task_id, agent) && !targets.contains(&"coder".to_string()) {
            targets.push("coder".into());
        }
    }
    targets
}

/// "auto" → online agent roles (deduplicated, excludes "user"); otherwise the literal role.
pub fn resolve_user_targets(state: &DaemonState, target: &str) -> Vec<String> {
    if target != "auto" {
        return vec![target.to_string()];
    }
    let mut targets = Vec::with_capacity(2);
    if let Some(preferred) = state.preferred_auto_target() {
        if role_is_online(state, &preferred) {
            targets.push(preferred);
        }
    }
    let claude_online = state.is_agent_online("claude");
    let codex_online = state.is_agent_online("codex");
    if claude_online
        && state.claude_role != "user"
        && state.role_has_compatible_online_agent(&state.claude_role)
        && !targets.contains(&state.claude_role)
    {
        targets.push(state.claude_role.clone());
    }
    if codex_online
        && state.codex_role != "user"
        && state.role_has_compatible_online_agent(&state.codex_role)
        && !targets.contains(&state.codex_role)
    {
        targets.push(state.codex_role.clone());
    }
    targets
}

fn role_is_online(state: &DaemonState, role: &str) -> bool {
    state.role_has_compatible_online_agent(role)
}

fn build_user_message(
    now: u64,
    to: &str,
    content: &str,
    attachments: &Option<Vec<Attachment>>,
) -> BridgeMessage {
    let suffix = if to == "user" {
        String::new()
    } else {
        format!("_{to}")
    };
    BridgeMessage {
        id: format!("user_{now}{suffix}"),
        from: "user".into(),
        display_source: Some("user".into()),
        to: to.to_string(),
        content: content.to_string(),
        timestamp: now,
        reply_to: None,
        priority: None,
        status: None,
        task_id: None,
        session_id: None,
        sender_agent_id: None,
        attachments: attachments.clone(),
    }
}

#[cfg(test)]
#[path = "routing_user_input_tests.rs"]
mod tests;
