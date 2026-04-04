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
) {
    if !has_user_input_payload(&content, &attachments) {
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
    let mut display_msg = build_user_message(now, &display_to, &content, &attachments);
    {
        let s = state.read().await;
        s.stamp_message_context("user", &mut display_msg);
    }
    gui::emit_agent_message(app, &display_msg);
    for role in targets {
        let mut msg = build_user_message(now, &role, &content, &attachments);
        {
            let s = state.read().await;
            s.stamp_message_context("user", &mut msg);
        }
        routing::route_message_silent(state, app, msg).await;
    }
}

fn has_user_input_payload(content: &str, attachments: &Option<Vec<Attachment>>) -> bool {
    !content.trim().is_empty() || attachments.as_ref().is_some_and(|atts| !atts.is_empty())
}

/// "auto" → online agent roles (deduplicated, excludes "user"); otherwise the literal role.
pub fn resolve_user_targets(state: &DaemonState, target: &str) -> Vec<String> {
    if target != "auto" {
        return vec![target.to_string()];
    }
    if let Some(preferred) = state.preferred_auto_target() {
        if role_is_online(state, &preferred) {
            return vec![preferred];
        }
    }
    let mut targets = Vec::with_capacity(2);
    let claude_online = state.is_agent_online("claude");
    let codex_online = state.is_agent_online("codex");
    if claude_online
        && state.claude_role != "user"
        && state.role_has_compatible_online_agent(&state.claude_role)
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
mod tests {
    use super::*;
    use crate::daemon::state::AgentSender;
    use crate::daemon::task_graph::types::{CreateSessionParams, Provider, SessionRole};
    use crate::daemon::types::{ProviderConnectionMode, ProviderConnectionState, ToAgent};

    fn file_attachment() -> Attachment {
        Attachment {
            file_path: "/tmp/spec.md".into(),
            file_name: "spec.md".into(),
            is_image: false,
            media_type: None,
        }
    }

    #[test]
    fn attachments_only_user_input_counts_as_payload() {
        assert!(has_user_input_payload(
            "   \n\t",
            &Some(vec![file_attachment()])
        ));
        assert!(!has_user_input_payload("   \n\t", &None));
        assert!(!has_user_input_payload("   \n\t", &Some(vec![])));
    }

    #[test]
    fn auto_target_ignores_online_agent_bound_to_another_task_session() {
        let mut state = DaemonState::new();
        let task = state.task_graph.create_task("/repo-b", "repo-b");
        state.active_task_id = Some(task.task_id.clone());
        let lead = state.task_graph.create_session(CreateSessionParams {
            task_id: &task.task_id,
            parent_session_id: None,
            provider: Provider::Claude,
            role: SessionRole::Lead,
            cwd: "/repo-b",
            title: "Lead",
        });
        state
            .task_graph
            .set_lead_session(&task.task_id, &lead.session_id);
        state
            .task_graph
            .set_external_session_id(&lead.session_id, "claude_current");

        let (claude_tx, _claude_rx) = tokio::sync::mpsc::channel::<ToAgent>(1);
        state
            .attached_agents
            .insert("claude".into(), AgentSender::new(claude_tx, 0));
        state.claude_role = "lead".into();
        state.set_provider_connection(
            "claude",
            ProviderConnectionState {
                provider: Provider::Claude,
                external_session_id: "claude_stale".into(),
                cwd: "/repo-a".into(),
                connection_mode: ProviderConnectionMode::Resumed,
            },
        );

        assert!(resolve_user_targets(&state, "auto").is_empty());
    }
}
