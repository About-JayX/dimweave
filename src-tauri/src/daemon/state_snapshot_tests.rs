use super::*;
use crate::daemon::task_graph::types::Provider;
use crate::daemon::types::{
    OnlineAgentInfo, ProviderConnectionMode, ProviderConnectionState, RuntimeHealthLevel,
    RuntimeHealthStatus,
};

#[test]
fn init_and_get_task_runtime() {
    let mut s = DaemonState::new();
    let task = s.create_and_select_task("/ws", "RT Test");
    assert!(s.get_task_runtime(&task.task_id).is_none());

    s.init_task_runtime(&task.task_id, std::path::PathBuf::from("/ws/tasks/t1"));
    let rt = s.get_task_runtime(&task.task_id).expect("runtime exists");
    assert_eq!(rt.task_id, task.task_id);
    assert_eq!(rt.workspace_root, std::path::PathBuf::from("/ws/tasks/t1"));
}

#[test]
fn task_runtimes_empty_by_default() {
    let s = DaemonState::new();
    assert!(s.task_runtimes.is_empty());
}

#[test]
fn online_agents_snapshot_empty_when_no_agents() {
    let s = DaemonState::new();
    let snapshot = s.online_agents_snapshot();
    assert!(snapshot.is_empty());
}

#[test]
fn online_agents_snapshot_only_claude_sdk() {
    let mut s = DaemonState::new();
    let (tx, _rx) = tokio::sync::mpsc::channel::<String>(1);
    let epoch = s.begin_claude_sdk_launch("nonce-a".into());
    s.claude_role = "lead".into();
    assert!(s.attach_claude_sdk_ws(epoch, "nonce-a", tx).is_some());

    let snapshot = s.online_agents_snapshot();
    assert_eq!(snapshot.len(), 1);
    assert_eq!(
        snapshot[0],
        OnlineAgentInfo {
            agent_id: "claude".into(),
            role: "lead".into(),
            model_source: "claude".into(),
        }
    );
}

#[test]
fn online_agents_snapshot_excludes_bridge_only_claude() {
    let mut s = DaemonState::new();
    let (tx, _rx) = tokio::sync::mpsc::channel::<ToAgent>(1);
    s.attached_agents.insert(
        "claude".into(),
        crate::daemon::state::AgentSender::new(tx, 0),
    );

    let snapshot = s.online_agents_snapshot();
    assert!(snapshot.is_empty());
}

#[test]
fn online_agents_snapshot_only_codex() {
    let mut s = DaemonState::new();
    let (tx, _rx) = tokio::sync::mpsc::channel::<(Vec<serde_json::Value>, bool)>(1);
    s.codex_role = "coder".into();
    s.codex_inject_tx = Some(tx);

    let snapshot = s.online_agents_snapshot();
    assert_eq!(snapshot.len(), 1);
    assert_eq!(
        snapshot[0],
        OnlineAgentInfo {
            agent_id: "codex".into(),
            role: "coder".into(),
            model_source: "codex".into(),
        }
    );
}

#[test]
fn online_agents_snapshot_both_agents_in_fixed_order() {
    let mut s = DaemonState::new();
    let (claude_tx, _claude_rx) = tokio::sync::mpsc::channel::<String>(1);
    let (codex_tx, _codex_rx) = tokio::sync::mpsc::channel::<(Vec<serde_json::Value>, bool)>(1);
    let epoch = s.begin_claude_sdk_launch("nonce-a".into());
    s.claude_role = "lead".into();
    assert!(s
        .attach_claude_sdk_ws(epoch, "nonce-a", claude_tx)
        .is_some());
    s.codex_inject_tx = Some(codex_tx);

    let snapshot = s.online_agents_snapshot();
    assert_eq!(snapshot.len(), 2);
    assert_eq!(snapshot[0].agent_id, "claude");
    assert_eq!(snapshot[1].agent_id, "codex");
}

#[test]
fn online_agents_snapshot_role_reflects_current_state() {
    let mut s = DaemonState::new();
    let (claude_tx, _claude_rx) = tokio::sync::mpsc::channel::<String>(1);
    let (codex_tx, _codex_rx) = tokio::sync::mpsc::channel::<(Vec<serde_json::Value>, bool)>(1);
    let epoch = s.begin_claude_sdk_launch("nonce-a".into());
    s.claude_role = "coder".into();
    s.codex_role = "lead".into();
    assert!(s
        .attach_claude_sdk_ws(epoch, "nonce-a", claude_tx)
        .is_some());
    s.codex_inject_tx = Some(codex_tx);

    let snapshot = s.online_agents_snapshot();
    assert_eq!(snapshot[0].role, "coder");
    assert_eq!(snapshot[1].role, "lead");
}

#[test]
fn online_agents_snapshot_multi_claude_per_agent_slots() {
    let mut s = DaemonState::new();
    let task = s.create_and_select_task("/ws", "Multi");
    let agent_a = s
        .task_graph
        .add_task_agent(&task.task_id, Provider::Claude, "lead");
    let agent_b = s
        .task_graph
        .add_task_agent(&task.task_id, Provider::Claude, "coder");
    s.claude_role = "lead".into();
    s.init_task_runtime(&task.task_id, std::path::PathBuf::from("/ws"));
    let (tx_a, _rx_a) = tokio::sync::mpsc::channel::<String>(1);
    let (tx_b, _rx_b) = tokio::sync::mpsc::channel::<String>(1);
    s.task_runtimes
        .get_mut(&task.task_id)
        .unwrap()
        .get_or_create_claude_slot(&agent_a.agent_id)
        .ws_tx = Some(tx_a);
    s.task_runtimes
        .get_mut(&task.task_id)
        .unwrap()
        .get_or_create_claude_slot(&agent_b.agent_id)
        .ws_tx = Some(tx_b);

    let snapshot = s.online_agents_snapshot();
    assert_eq!(snapshot.len(), 2, "two Claude agents should both appear");
    let ids: Vec<_> = snapshot.iter().map(|a| a.agent_id.as_str()).collect();
    assert!(ids.contains(&agent_a.agent_id.as_str()));
    assert!(ids.contains(&agent_b.agent_id.as_str()));
    let a = snapshot
        .iter()
        .find(|a| a.agent_id == agent_a.agent_id)
        .unwrap();
    let b = snapshot
        .iter()
        .find(|a| a.agent_id == agent_b.agent_id)
        .unwrap();
    assert_eq!(a.role, "lead");
    assert_eq!(a.model_source, "claude");
    assert_eq!(b.role, "coder");
    assert_eq!(b.model_source, "claude");
}

#[test]
fn online_agents_snapshot_mixed_providers_per_agent() {
    let mut s = DaemonState::new();
    let task = s.create_and_select_task("/ws", "Mixed");
    let claude_agent = s
        .task_graph
        .add_task_agent(&task.task_id, Provider::Claude, "coder");
    let codex_agent = s
        .task_graph
        .add_task_agent(&task.task_id, Provider::Codex, "lead");
    s.claude_role = "coder".into();
    s.codex_role = "lead".into();
    s.init_task_runtime(&task.task_id, std::path::PathBuf::from("/ws"));
    let (claude_tx, _) = tokio::sync::mpsc::channel::<String>(1);
    let (codex_tx, _) = tokio::sync::mpsc::channel::<(Vec<serde_json::Value>, bool)>(1);
    s.task_runtimes
        .get_mut(&task.task_id)
        .unwrap()
        .get_or_create_claude_slot(&claude_agent.agent_id)
        .ws_tx = Some(claude_tx);
    s.task_runtimes
        .get_mut(&task.task_id)
        .unwrap()
        .get_or_create_codex_slot(&codex_agent.agent_id, 4500)
        .inject_tx = Some(codex_tx);

    let snapshot = s.online_agents_snapshot();
    assert_eq!(snapshot.len(), 2);
    let claude = snapshot
        .iter()
        .find(|a| a.agent_id == claude_agent.agent_id)
        .unwrap();
    let codex = snapshot
        .iter()
        .find(|a| a.agent_id == codex_agent.agent_id)
        .unwrap();
    assert_eq!(claude.role, "coder");
    assert_eq!(claude.model_source, "claude");
    assert_eq!(codex.role, "lead");
    assert_eq!(codex.model_source, "codex");
}

#[test]
fn online_agents_snapshot_offline_slots_excluded() {
    let mut s = DaemonState::new();
    let task = s.create_and_select_task("/ws", "Partial");
    let agent_a = s
        .task_graph
        .add_task_agent(&task.task_id, Provider::Claude, "lead");
    let _agent_b = s
        .task_graph
        .add_task_agent(&task.task_id, Provider::Claude, "coder");
    s.init_task_runtime(&task.task_id, std::path::PathBuf::from("/ws"));
    let (tx, _) = tokio::sync::mpsc::channel::<String>(1);
    // Only agent_a gets a channel (online); agent_b has no slot
    s.task_runtimes
        .get_mut(&task.task_id)
        .unwrap()
        .get_or_create_claude_slot(&agent_a.agent_id)
        .ws_tx = Some(tx);

    let snapshot = s.online_agents_snapshot();
    assert_eq!(snapshot.len(), 1);
    assert_eq!(snapshot[0].agent_id, agent_a.agent_id);
}

#[test]
fn online_agents_snapshot_no_phantom_singleton_when_per_agent_online() {
    // Regression: attach_claude_task_ws_for_agent and
    // attach_codex_task_session_for_agent mirror the channel into
    // the singleton fields. Phase 2 must not emit an extra "claude"/"codex"
    // row when a real per-agent slot already covers that provider.
    let mut s = DaemonState::new();
    let task = s.create_and_select_task("/ws", "Mirror");
    let claude_agent = s
        .task_graph
        .add_task_agent(&task.task_id, Provider::Claude, "lead");
    let codex_agent = s
        .task_graph
        .add_task_agent(&task.task_id, Provider::Codex, "coder");
    s.claude_role = "lead".into();
    s.codex_role = "coder".into();
    s.init_task_runtime(&task.task_id, std::path::PathBuf::from("/ws"));

    // Use the real attach paths that mirror into singleton fields.
    let (claude_tx, _claude_rx) = tokio::sync::mpsc::channel::<String>(1);
    let claude_epoch = s
        .begin_claude_task_launch_for_agent(&task.task_id, &claude_agent.agent_id, "nonce-c".into())
        .unwrap();
    s.attach_claude_task_ws_for_agent(
        &task.task_id,
        &claude_agent.agent_id,
        claude_epoch,
        "nonce-c",
        claude_tx,
    )
    .unwrap();

    let (codex_tx, _codex_rx) = tokio::sync::mpsc::channel::<(Vec<serde_json::Value>, bool)>(1);
    let codex_epoch = s
        .begin_codex_task_launch_for_agent(&task.task_id, &codex_agent.agent_id, 4500)
        .unwrap();
    s.attach_codex_task_session_for_agent(
        &task.task_id,
        &codex_agent.agent_id,
        codex_epoch,
        codex_tx,
        None,
    );

    // Singleton mirrors are now set (verified precondition).
    assert!(
        s.claude_sdk_ws_tx.is_some(),
        "singleton mirror should be set"
    );
    assert!(
        s.codex_inject_tx.is_some(),
        "singleton mirror should be set"
    );

    let snapshot = s.online_agents_snapshot();
    // Must have exactly 2 rows — the real agent_ids — no phantom "claude"/"codex".
    assert_eq!(snapshot.len(), 2, "snapshot: {snapshot:?}");
    assert!(
        snapshot.iter().all(|a| a.agent_id != "claude"),
        "no phantom claude row"
    );
    assert!(
        snapshot.iter().all(|a| a.agent_id != "codex"),
        "no phantom codex row"
    );
    let claude = snapshot
        .iter()
        .find(|a| a.agent_id == claude_agent.agent_id)
        .unwrap();
    let codex = snapshot
        .iter()
        .find(|a| a.agent_id == codex_agent.agent_id)
        .unwrap();
    assert_eq!(claude.role, "lead");
    assert_eq!(claude.model_source, "claude");
    assert_eq!(codex.role, "coder");
    assert_eq!(codex.model_source, "codex");
}

#[test]
fn status_snapshot_includes_runtime_health() {
    let mut s = DaemonState::new();
    s.set_runtime_health(RuntimeHealthStatus {
        level: RuntimeHealthLevel::Error,
        source: "claude_sdk".into(),
        message: "Claude reconnect failed after 5 attempts".into(),
    });

    let snapshot = s.status_snapshot();

    assert_eq!(
        snapshot.runtime_health,
        Some(RuntimeHealthStatus {
            level: RuntimeHealthLevel::Error,
            source: "claude_sdk".into(),
            message: "Claude reconnect failed after 5 attempts".into(),
        })
    );
}

#[test]
fn rollback_task_creation_cleans_up_all_state() {
    let mut s = DaemonState::new();
    let task = s.create_and_select_task("/ws", "Rollback Test");
    let task_id = task.task_id.clone();
    s.init_task_runtime(&task_id, std::path::PathBuf::from("/ws/tasks/t1"));

    // Verify state exists before rollback
    assert!(s.task_graph.get_task(&task_id).is_some());
    assert_eq!(s.active_task_id.as_deref(), Some(task_id.as_str()));
    assert!(s.get_task_runtime(&task_id).is_some());

    s.rollback_task_creation(&task_id);

    assert!(s.task_graph.get_task(&task_id).is_none());
    assert!(s.active_task_id.is_none());
    assert!(s.get_task_runtime(&task_id).is_none());
}

#[test]
fn task_provider_summary_same_provider_uses_each_agent_connection() {
    let mut s = DaemonState::new();
    let task = s.create_and_select_task("/ws", "Same Provider");
    let lead = s
        .task_graph
        .add_task_agent(&task.task_id, Provider::Codex, "lead");
    let coder = s
        .task_graph
        .add_task_agent(&task.task_id, Provider::Codex, "coder");
    s.init_task_runtime(&task.task_id, std::path::PathBuf::from("/ws"));

    let lead_conn = ProviderConnectionState {
        provider: Provider::Codex,
        external_session_id: "codex_lead_thread".into(),
        cwd: "/ws".into(),
        connection_mode: ProviderConnectionMode::New,
    };
    let coder_conn = ProviderConnectionState {
        provider: Provider::Codex,
        external_session_id: "codex_coder_thread".into(),
        cwd: "/ws".into(),
        connection_mode: ProviderConnectionMode::New,
    };

    let (lead_tx, _) = tokio::sync::mpsc::channel(1);
    let lead_epoch = s
        .begin_codex_task_launch_for_agent(&task.task_id, &lead.agent_id, 4500)
        .unwrap();
    assert!(s.attach_codex_task_session_for_agent(
        &task.task_id,
        &lead.agent_id,
        lead_epoch,
        lead_tx,
        Some(lead_conn.clone()),
    ));

    let (coder_tx, _) = tokio::sync::mpsc::channel(1);
    let coder_epoch = s
        .begin_codex_task_launch_for_agent(&task.task_id, &coder.agent_id, 4501)
        .unwrap();
    assert!(s.attach_codex_task_session_for_agent(
        &task.task_id,
        &coder.agent_id,
        coder_epoch,
        coder_tx,
        Some(coder_conn.clone()),
    ));

    let summary = s
        .task_provider_summary(&task.task_id)
        .expect("summary exists");

    assert!(summary.lead_online);
    assert!(summary.coder_online);
    assert_eq!(summary.lead_agent_id, lead.agent_id);
    assert_eq!(summary.coder_agent_id, coder.agent_id);
    assert_eq!(summary.lead_provider_session, Some(lead_conn));
    assert_eq!(summary.coder_provider_session, Some(coder_conn));
}
