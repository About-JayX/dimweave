use super::*;
use crate::daemon::{
    state::DaemonState,
    task_graph::types::{CreateSessionParams, Provider, SessionRole, TaskStatus},
    types::{ProviderConnectionMode, ProviderConnectionState},
};

#[test]
fn explicit_target_returns_single_role() {
    let s = DaemonState::new();
    assert_eq!(resolve_user_targets(&s, "coder"), vec!["coder"]);
}

#[test]
fn auto_with_no_agents_returns_empty() {
    let s = DaemonState::new();
    assert!(resolve_user_targets(&s, "auto").is_empty());
}

#[test]
fn auto_with_claude_bridge_only_returns_empty() {
    let mut s = DaemonState::new();
    let (tx, _rx) = tokio::sync::mpsc::channel(1);
    s.attached_agents.insert(
        "claude".into(),
        crate::daemon::state::AgentSender::new(tx, 0),
    );
    assert!(resolve_user_targets(&s, "auto").is_empty());
}

#[test]
fn auto_with_claude_sdk_only() {
    let mut s = DaemonState::new();
    let (tx, _rx) = tokio::sync::mpsc::channel(1);
    let epoch = s.begin_claude_sdk_launch("nonce-a".into());
    s.claude_role = "lead".into();
    assert!(s.attach_claude_sdk_ws(epoch, "nonce-a", tx).is_some());
    assert_eq!(resolve_user_targets(&s, "auto"), vec!["lead"]);
}

#[test]
fn auto_with_codex_only() {
    let mut s = DaemonState::new();
    let (tx, _rx) = tokio::sync::mpsc::channel(1);
    s.codex_role = "coder".into();
    s.codex_inject_tx = Some(tx);
    assert_eq!(resolve_user_targets(&s, "auto"), vec!["coder"]);
}

#[test]
fn auto_with_both_agents_returns_two_roles() {
    let mut s = DaemonState::new();
    let (claude_tx, _) = tokio::sync::mpsc::channel(1);
    let (codex_tx, _) = tokio::sync::mpsc::channel(1);
    let epoch = s.begin_claude_sdk_launch("nonce-a".into());
    s.claude_role = "lead".into();
    s.codex_role = "coder".into();
    assert!(s.attach_claude_sdk_ws(epoch, "nonce-a", claude_tx).is_some());
    s.codex_inject_tx = Some(codex_tx);
    assert_eq!(resolve_user_targets(&s, "auto"), vec!["lead", "coder"]);
}

#[test]
fn auto_dedupes_when_same_role() {
    let mut s = DaemonState::new();
    s.claude_role = "coder".into();
    s.codex_role = "coder".into();
    let (claude_tx, _) = tokio::sync::mpsc::channel(1);
    let (codex_tx, _) = tokio::sync::mpsc::channel(1);
    let epoch = s.begin_claude_sdk_launch("nonce-a".into());
    assert!(s.attach_claude_sdk_ws(epoch, "nonce-a", claude_tx).is_some());
    s.codex_inject_tx = Some(codex_tx);
    assert_eq!(resolve_user_targets(&s, "auto"), vec!["coder"]);
}

#[test]
fn auto_excludes_user_role() {
    let mut s = DaemonState::new();
    s.claude_role = "user".into();
    let (tx, _) = tokio::sync::mpsc::channel(1);
    let epoch = s.begin_claude_sdk_launch("nonce-a".into());
    assert!(s.attach_claude_sdk_ws(epoch, "nonce-a", tx).is_some());
    assert!(resolve_user_targets(&s, "auto").is_empty());
}

#[test]
fn auto_keeps_preferred_task_role_first_but_still_fanouts() {
    let mut s = DaemonState::new();
    let task = s.task_graph.create_task("/ws", "Task");
    s.set_active_task(Some(task.task_id.clone()));
    let lead = s.task_graph.create_session(CreateSessionParams {
        task_id: &task.task_id,
        parent_session_id: None,
        provider: Provider::Claude,
        role: SessionRole::Lead,
        cwd: "/ws",
        title: "Lead",
        agent_id: None,
    });
    s.task_graph.set_lead_session(&task.task_id, &lead.session_id);
    s.task_graph
        .set_external_session_id(&lead.session_id, "claude_current");
    let coder = s.task_graph.create_session(CreateSessionParams {
        task_id: &task.task_id,
        parent_session_id: Some(&lead.session_id),
        provider: Provider::Codex,
        role: SessionRole::Coder,
        cwd: "/ws",
        title: "Coder",
        agent_id: None,
    });
    s.task_graph
        .set_coder_session(&task.task_id, &coder.session_id);
    s.task_graph
        .set_external_session_id(&coder.session_id, "codex_current");
    s.task_graph
        .update_task_status(&task.task_id, TaskStatus::Reviewing);
    let (claude_tx, _) = tokio::sync::mpsc::channel(1);
    let (codex_tx, _) = tokio::sync::mpsc::channel(1);
    s.attached_agents.insert(
        "claude".into(),
        crate::daemon::state::AgentSender::new(claude_tx, 0),
    );
    s.codex_inject_tx = Some(codex_tx);
    s.set_provider_connection(
        "claude",
        ProviderConnectionState {
            provider: Provider::Claude,
            external_session_id: "claude_current".into(),
            cwd: "/ws".into(),
            connection_mode: ProviderConnectionMode::Resumed,
        },
    );
    s.set_provider_connection(
        "codex",
        ProviderConnectionState {
            provider: Provider::Codex,
            external_session_id: "codex_current".into(),
            cwd: "/ws".into(),
            connection_mode: ProviderConnectionMode::Resumed,
        },
    );

    assert_eq!(resolve_user_targets(&s, "auto"), vec!["lead", "coder"]);

    s.task_graph
        .update_task_status(&task.task_id, TaskStatus::Implementing);
    assert_eq!(resolve_user_targets(&s, "auto"), vec!["coder", "lead"]);
}

#[test]
fn auto_prefers_bound_claude_coder_for_active_task() {
    let mut s = DaemonState::new();
    let task = s.task_graph.create_task("/ws", "Task");
    s.set_active_task(Some(task.task_id.clone()));
    let lead = s.task_graph.create_session(CreateSessionParams {
        task_id: &task.task_id,
        parent_session_id: None,
        provider: Provider::Codex,
        role: SessionRole::Lead,
        cwd: "/ws",
        title: "Lead",
        agent_id: None,
    });
    s.task_graph.set_lead_session(&task.task_id, &lead.session_id);
    s.task_graph
        .set_external_session_id(&lead.session_id, "codex_lead_current");
    let coder = s.task_graph.create_session(CreateSessionParams {
        task_id: &task.task_id,
        parent_session_id: Some(&lead.session_id),
        provider: Provider::Claude,
        role: SessionRole::Coder,
        cwd: "/ws",
        title: "Coder",
        agent_id: None,
    });
    s.task_graph.set_coder_session(&task.task_id, &coder.session_id);
    s.task_graph
        .set_external_session_id(&coder.session_id, "claude_coder_current");
    s.task_graph
        .update_task_status(&task.task_id, TaskStatus::Implementing);

    s.claude_role = "coder".into();
    s.codex_role = "lead".into();
    let (claude_tx, _) = tokio::sync::mpsc::channel(1);
    let (codex_tx, _) = tokio::sync::mpsc::channel(1);
    s.attached_agents.insert(
        "claude".into(),
        crate::daemon::state::AgentSender::new(claude_tx, 0),
    );
    s.codex_inject_tx = Some(codex_tx);
    s.set_provider_connection(
        "claude",
        ProviderConnectionState {
            provider: Provider::Claude,
            external_session_id: "claude_coder_current".into(),
            cwd: "/ws".into(),
            connection_mode: ProviderConnectionMode::Resumed,
        },
    );
    s.set_provider_connection(
        "codex",
        ProviderConnectionState {
            provider: Provider::Codex,
            external_session_id: "codex_lead_current".into(),
            cwd: "/ws".into(),
            connection_mode: ProviderConnectionMode::Resumed,
        },
    );

    assert_eq!(resolve_user_targets(&s, "auto"), vec!["coder", "lead"]);
}

// ── agent_id routing: auto-target from task_agents ────────────

#[test]
fn agent_id_routing_auto_task_agents_prefers_lead() {
    use crate::daemon::task_runtime::{ClaudeTaskSlot, CodexTaskSlot, TaskRuntime};

    let mut s = DaemonState::new();
    let task = s.task_graph.create_task("/ws", "T");
    // Add agents: coder first (order 0), lead second (order 1)
    s.task_graph.add_task_agent(&task.task_id, Provider::Codex, "coder");
    s.task_graph.add_task_agent(&task.task_id, Provider::Claude, "lead");

    // Set up task runtime with both online
    let mut rt = TaskRuntime::new(task.task_id.clone(), "/ws".into());
    let mut claude_slot = ClaudeTaskSlot::new();
    let (claude_tx, _) = tokio::sync::mpsc::channel::<String>(1);
    claude_slot.ws_tx = Some(claude_tx);
    rt.claude_slot = Some(claude_slot);
    let mut codex_slot = CodexTaskSlot::new(4501);
    let (codex_tx, _) = tokio::sync::mpsc::channel(1);
    codex_slot.inject_tx = Some(codex_tx);
    rt.codex_slot = Some(codex_slot);
    s.task_runtimes.insert(task.task_id.clone(), rt);

    let targets = resolve_user_targets_for_task(&s, "auto", &task.task_id);
    assert!(!targets.is_empty(), "should resolve at least one target");
    assert_eq!(targets[0], "lead", "lead must come first even though coder has lower order");
}

#[test]
fn agent_id_routing_auto_task_agents_first_role_when_no_lead() {
    use crate::daemon::task_runtime::{ClaudeTaskSlot, TaskRuntime};

    let mut s = DaemonState::new();
    let task = s.task_graph.create_task("/ws", "T");
    // Only a "reviewer" agent, no lead
    s.task_graph.add_task_agent(&task.task_id, Provider::Claude, "reviewer");

    let mut rt = TaskRuntime::new(task.task_id.clone(), "/ws".into());
    let mut claude_slot = ClaudeTaskSlot::new();
    let (tx, _) = tokio::sync::mpsc::channel::<String>(1);
    claude_slot.ws_tx = Some(tx);
    rt.claude_slot = Some(claude_slot);
    s.task_runtimes.insert(task.task_id.clone(), rt);

    let targets = resolve_user_targets_for_task(&s, "auto", &task.task_id);
    assert_eq!(targets, vec!["reviewer"], "should fall back to first ordered role");
}

#[test]
fn agent_id_routing_auto_task_agents_empty_task_returns_empty() {
    let mut s = DaemonState::new();
    let task = s.task_graph.create_task("/ws", "T");
    // No agents added

    let targets = resolve_user_targets_for_task(&s, "auto", &task.task_id);
    assert!(targets.is_empty(), "task with no agents should return empty auto targets");
}
