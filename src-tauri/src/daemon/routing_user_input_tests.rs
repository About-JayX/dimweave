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

#[test]
fn explicit_task_id_stamps_without_mutating_active_task() {
    let mut state = DaemonState::new();
    let task_a = state.task_graph.create_task("/repo-a", "Task A");
    let task_b = state.task_graph.create_task("/repo-b", "Task B");
    state.active_task_id = Some(task_a.task_id.clone());
    let sess_b = state.task_graph.create_session(CreateSessionParams {
        task_id: &task_b.task_id,
        parent_session_id: None,
        provider: Provider::Codex,
        role: SessionRole::Coder,
        cwd: "/repo-b",
        title: "Coder B",
    });
    state
        .task_graph
        .set_coder_session(&task_b.task_id, &sess_b.session_id);

    let mut msg = build_user_message(1, "coder", "hello", &None);
    stamp_user_message(&state, Some(&task_b.task_id), &mut msg);

    assert_eq!(msg.task_id.as_deref(), Some(task_b.task_id.as_str()));
    assert_eq!(state.active_task_id.as_deref(), Some(task_a.task_id.as_str()));
}

#[test]
fn explicit_task_resolves_targets_from_task_context() {
    use crate::daemon::task_runtime::{ClaudeTaskSlot, TaskRuntime};

    let mut state = DaemonState::new();
    state.claude_role = "lead".into();
    let task_a = state.task_graph.create_task("/repo-a", "Task A");
    let task_b = state.task_graph.create_task("/repo-b", "Task B");
    state.active_task_id = Some(task_a.task_id.clone());

    // Set up task B with a claude lead slot online
    let mut rt_b = TaskRuntime::new(task_b.task_id.clone(), "/repo-b".into());
    let mut claude_slot = ClaudeTaskSlot::new();
    let (tx, _rx) = tokio::sync::mpsc::channel::<String>(1);
    claude_slot.ws_tx = Some(tx);
    rt_b.claude_slot = Some(claude_slot);
    state.task_runtimes.insert(task_b.task_id.clone(), rt_b);

    // Task A has no sessions — global resolution finds no compatible agent
    assert!(resolve_user_targets(&state, "auto").is_empty());

    // Task-scoped resolution for task B should find lead online
    let targets = resolve_user_targets_for_task(&state, "auto", &task_b.task_id);
    assert!(targets.contains(&"lead".to_string()));
}

#[test]
fn no_explicit_task_id_uses_active_task() {
    let mut state = DaemonState::new();
    let task = state.task_graph.create_task("/repo", "Task");
    state.active_task_id = Some(task.task_id.clone());

    let mut msg = build_user_message(1, "lead", "hi", &None);
    stamp_user_message(&state, None, &mut msg);

    assert_eq!(msg.task_id.as_deref(), Some(task.task_id.as_str()));
}
