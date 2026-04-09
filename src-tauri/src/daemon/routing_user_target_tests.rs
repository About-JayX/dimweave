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
