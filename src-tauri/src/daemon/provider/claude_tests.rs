use crate::daemon::provider::claude;
use crate::daemon::provider::shared::{
    ProviderHistoryPage, ProviderResumeTarget, SessionRegistration,
};
use crate::daemon::task_graph::store::TaskGraphStore;
use crate::daemon::task_graph::types::*;
use crate::daemon::DaemonState;
use std::fs;

// ── register_session ───────────────────────────────────────

#[test]
fn register_claude_session_binds_session_id() {
    let mut store = TaskGraphStore::new();
    let task = store.create_task("/ws", "T1");

    let reg = SessionRegistration {
        task_id: task.task_id.clone(),
        parent_session_id: None,
        role: SessionRole::Lead,
        cwd: "/ws".into(),
        title: "Claude lead".into(),
        external_id: Some("claude_sess_abc".into()),
        transcript_path: Some("/tmp/.claude/projects/-ws/claude_sess_abc.jsonl".into()),
    };
    let sess = claude::register_session(&mut store, reg);

    assert_eq!(sess.provider, Provider::Claude);
    assert_eq!(sess.role, SessionRole::Lead);
    assert_eq!(sess.external_session_id.as_deref(), Some("claude_sess_abc"));
    assert_eq!(
        sess.transcript_path.as_deref(),
        Some("/tmp/.claude/projects/-ws/claude_sess_abc.jsonl")
    );
    assert_eq!(sess.task_id, task.task_id);
}

#[test]
fn register_claude_session_without_external_id() {
    let mut store = TaskGraphStore::new();
    let task = store.create_task("/ws", "T1");

    let reg = SessionRegistration {
        task_id: task.task_id.clone(),
        parent_session_id: None,
        role: SessionRole::Lead,
        cwd: "/ws".into(),
        title: "Claude lead".into(),
        external_id: None,
        transcript_path: None,
    };
    let sess = claude::register_session(&mut store, reg);
    assert!(sess.external_session_id.is_none());
    assert!(sess.transcript_path.is_none());
    assert_eq!(sess.provider, Provider::Claude);
}

// ── bind_session_id ────────────────────────────────────────

#[test]
fn bind_session_id_updates_existing_session() {
    let mut store = TaskGraphStore::new();
    let task = store.create_task("/ws", "T1");
    let sess = store.create_session(CreateSessionParams {
        task_id: &task.task_id,
        parent_session_id: None,
        provider: Provider::Claude,
        role: SessionRole::Lead,
        cwd: "/ws",
        title: "Claude",
    });

    let ok = claude::bind_session_id(&mut store, &sess.session_id, "claude_sess_late");
    assert!(ok);
    let updated = store.get_session(&sess.session_id).unwrap();
    assert_eq!(
        updated.external_session_id.as_deref(),
        Some("claude_sess_late")
    );
}

#[test]
fn bind_session_id_returns_false_for_missing() {
    let mut store = TaskGraphStore::new();
    assert!(!claude::bind_session_id(&mut store, "no-such", "x"));
}

// ── register_on_connect (integration with DaemonState) ─────

#[test]
fn register_on_connect_creates_lead_session() {
    let mut s = DaemonState::new();
    let task = s.task_graph.create_task("/ws", "T1");
    let tid = task.task_id.clone();
    s.task_graph.update_task_status(&tid, TaskStatus::Planning);
    s.set_active_task(Some(tid.clone()));

    claude::register_on_connect(&mut s, "lead", "/ws", Some("claude_sess_123"));

    let task = s.task_graph.get_task(&tid).unwrap();
    let lead_sid = task.lead_session_id.as_ref().expect("lead session set");
    let sess = s.task_graph.get_session(lead_sid).unwrap();
    assert_eq!(sess.provider, Provider::Claude);
    assert_eq!(sess.role, SessionRole::Lead);
    assert_eq!(sess.external_session_id.as_deref(), Some("claude_sess_123"));
}

#[test]
fn register_on_connect_as_coder_sets_coder_session() {
    let mut s = DaemonState::new();
    let task = s.task_graph.create_task("/ws", "T1");
    let tid = task.task_id.clone();
    s.task_graph
        .update_task_status(&tid, TaskStatus::Implementing);
    s.set_active_task(Some(tid.clone()));

    claude::register_on_connect(&mut s, "coder", "/ws", Some("claude_coder_1"));

    let task = s.task_graph.get_task(&tid).unwrap();
    let coder_sid = task
        .current_coder_session_id
        .as_ref()
        .expect("coder session set");
    let sess = s.task_graph.get_session(coder_sid).unwrap();
    assert_eq!(sess.provider, Provider::Claude);
    assert_eq!(sess.role, SessionRole::Coder);
    assert_eq!(sess.external_session_id.as_deref(), Some("claude_coder_1"));
}

#[test]
fn register_on_connect_noop_without_active_task() {
    let mut s = DaemonState::new();
    claude::register_on_connect(&mut s, "lead", "/ws", Some("sess_x"));
    assert!(s.task_graph.list_tasks().is_empty());
}

#[test]
fn register_on_connect_without_session_id() {
    let mut s = DaemonState::new();
    let task = s.task_graph.create_task("/ws", "T1");
    let tid = task.task_id.clone();
    s.set_active_task(Some(tid.clone()));

    claude::register_on_connect(&mut s, "lead", "/ws", None);

    let task = s.task_graph.get_task(&tid).unwrap();
    let lead_sid = task.lead_session_id.as_ref().expect("lead session set");
    let sess = s.task_graph.get_session(lead_sid).unwrap();
    assert!(sess.external_session_id.is_none());
}

#[test]
fn register_on_launch_captures_transcript_path() {
    let mut s = DaemonState::new();
    let task = s.task_graph.create_task("/ws", "T1");
    let tid = task.task_id.clone();
    s.set_active_task(Some(tid.clone()));

    claude::register_on_launch(
        &mut s,
        "lead",
        "/ws",
        "claude_launch_123",
        "/tmp/.claude/projects/-ws/claude_launch_123.jsonl",
    );

    let task = s.task_graph.get_task(&tid).unwrap();
    let lead_sid = task.lead_session_id.as_ref().expect("lead session set");
    let sess = s.task_graph.get_session(lead_sid).unwrap();
    assert_eq!(
        sess.external_session_id.as_deref(),
        Some("claude_launch_123")
    );
    assert_eq!(
        sess.transcript_path.as_deref(),
        Some("/tmp/.claude/projects/-ws/claude_launch_123.jsonl")
    );
}

#[test]
fn workspace_history_dir_scopes_to_workspace_slug() {
    let root = std::env::temp_dir().join("claude-history-root");
    let path = claude::workspace_history_dir("/Users/example/project-x", &root);
    assert_eq!(
        path,
        root.join("-Users-example-project-x"),
        "workspace history dir should follow Claude's project slug layout"
    );
}

#[test]
fn workspace_history_dir_normalizes_relative_paths() {
    let root = std::env::temp_dir().join("claude-history-root-rel");
    let expected = std::env::current_dir().unwrap().join("relative-ws");
    let mut slug = String::new();
    for ch in expected.to_string_lossy().chars() {
        match ch {
            '/' | '\\' | ':' => slug.push('-'),
            _ => slug.push(ch),
        }
    }

    let path = claude::workspace_history_dir("relative-ws", &root);
    assert_eq!(path, root.join(slug));
}

#[test]
fn list_sessions_reads_workspace_transcripts() {
    let base = std::env::temp_dir().join(format!(
        "agentnexus-claude-history-{}",
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
    ));
    let history_dir = claude::workspace_history_dir("/tmp/ws", &base);
    fs::create_dir_all(&history_dir).unwrap();

    let recent = history_dir.join("recent-session.jsonl");
    fs::write(
        &recent,
        concat!(
            "{\"type\":\"user\",\"timestamp\":\"2026-03-31T10:00:00.000Z\",\"sessionId\":\"recent-session\",\"cwd\":\"/tmp/ws\",\"message\":{\"role\":\"user\",\"content\":\"Investigate resume path\"}}\n",
            "{\"type\":\"assistant\",\"timestamp\":\"2026-03-31T10:05:00.000Z\",\"sessionId\":\"recent-session\",\"cwd\":\"/tmp/ws\",\"message\":{\"role\":\"assistant\",\"content\":[{\"type\":\"text\",\"text\":\"Resume path analyzed.\"}]}}\n"
        ),
    )
    .unwrap();

    let older = history_dir.join("older-session.jsonl");
    fs::write(
        &older,
        concat!(
            "{\"type\":\"user\",\"timestamp\":\"2026-03-30T09:00:00.000Z\",\"sessionId\":\"older-session\",\"cwd\":\"/tmp/ws\",\"message\":{\"role\":\"user\",\"content\":\"Old task\"}}\n",
            "{\"type\":\"assistant\",\"timestamp\":\"2026-03-30T09:10:00.000Z\",\"sessionId\":\"older-session\",\"cwd\":\"/tmp/ws\",\"message\":{\"role\":\"assistant\",\"content\":[{\"type\":\"text\",\"text\":\"Older reply.\"}]}}\n"
        ),
    )
    .unwrap();

    let page = claude::list_sessions("/tmp/ws", Some(base.as_path())).unwrap();
    assert_eq!(
        page,
        ProviderHistoryPage {
            entries: vec![
                crate::daemon::provider::shared::ProviderHistoryEntry {
                    provider: Provider::Claude,
                    external_id: "recent-session".into(),
                    title: Some("Investigate resume path".into()),
                    preview: Some("Resume path analyzed.".into()),
                    cwd: Some("/tmp/ws".into()),
                    archived: false,
                    created_at: 1_774_951_200_000,
                    updated_at: 1_774_951_500_000,
                    status: SessionStatus::Paused,
                    normalized_session_id: None,
                    normalized_task_id: None,
                },
                crate::daemon::provider::shared::ProviderHistoryEntry {
                    provider: Provider::Claude,
                    external_id: "older-session".into(),
                    title: Some("Old task".into()),
                    preview: Some("Older reply.".into()),
                    cwd: Some("/tmp/ws".into()),
                    archived: false,
                    created_at: 1_774_861_200_000,
                    updated_at: 1_774_861_800_000,
                    status: SessionStatus::Paused,
                    normalized_session_id: None,
                    normalized_task_id: None,
                },
            ],
            next_cursor: None,
        }
    );

    fs::remove_dir_all(base).unwrap();
}

#[test]
fn build_resume_target_requires_external_session_id() {
    let session = SessionHandle {
        session_id: "sess_1".into(),
        task_id: "task_1".into(),
        parent_session_id: None,
        provider: Provider::Claude,
        role: SessionRole::Lead,
        external_session_id: None,
        transcript_path: Some("/tmp/.claude/projects/-tmp-ws/session.jsonl".into()),
        status: SessionStatus::Paused,
        cwd: "/tmp/ws".into(),
        title: "Claude lead".into(),
        created_at: 1,
        updated_at: 2,
    };

    let err = claude::build_resume_target(&session).unwrap_err();
    assert!(err.contains("missing external Claude session id"));
}

#[test]
fn build_resume_target_uses_external_id_and_cwd() {
    let session = SessionHandle {
        session_id: "sess_1".into(),
        task_id: "task_1".into(),
        parent_session_id: None,
        provider: Provider::Claude,
        role: SessionRole::Coder,
        external_session_id: Some("claude_resume_42".into()),
        transcript_path: Some("/tmp/.claude/projects/-tmp-ws/claude_resume_42.jsonl".into()),
        status: SessionStatus::Paused,
        cwd: "/tmp/ws".into(),
        title: "Claude coder".into(),
        created_at: 1,
        updated_at: 2,
    };

    assert_eq!(
        claude::build_resume_target(&session).unwrap(),
        ProviderResumeTarget {
            role: SessionRole::Coder,
            cwd: "/tmp/ws".into(),
            external_id: "claude_resume_42".into(),
        }
    );
}
