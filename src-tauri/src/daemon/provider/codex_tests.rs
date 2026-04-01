use crate::daemon::provider::codex;
use crate::daemon::provider::shared::{
    ProviderHistoryEntry, ProviderHistoryPage, SessionRegistration,
};
use crate::daemon::task_graph::store::TaskGraphStore;
use crate::daemon::task_graph::types::*;
use serde_json::json;
use std::fs;
use uuid::Uuid;

// ── register_session ───────────────────────────────────────

#[test]
fn register_codex_session_binds_thread_id() {
    let mut store = TaskGraphStore::new();
    let task = store.create_task("/ws", "T1");

    let reg = SessionRegistration {
        task_id: task.task_id.clone(),
        parent_session_id: None,
        role: SessionRole::Coder,
        cwd: "/ws".into(),
        title: "Codex coder".into(),
        external_id: Some("thread_abc123".into()),
        transcript_path: None,
    };
    let sess = codex::register_session(&mut store, reg);

    assert_eq!(sess.provider, Provider::Codex);
    assert_eq!(sess.role, SessionRole::Coder);
    assert_eq!(sess.external_session_id.as_deref(), Some("thread_abc123"));
    assert_eq!(sess.task_id, task.task_id);
    assert_eq!(sess.status, SessionStatus::Active);
}

#[test]
fn register_codex_session_without_thread_id() {
    let mut store = TaskGraphStore::new();
    let task = store.create_task("/ws", "T1");

    let reg = SessionRegistration {
        task_id: task.task_id.clone(),
        parent_session_id: None,
        role: SessionRole::Coder,
        cwd: "/ws".into(),
        title: "Codex coder".into(),
        external_id: None,
        transcript_path: None,
    };
    let sess = codex::register_session(&mut store, reg);
    assert!(sess.external_session_id.is_none());
}

#[test]
fn register_codex_session_as_child_of_lead() {
    let mut store = TaskGraphStore::new();
    let task = store.create_task("/ws", "T1");
    let lead = store.create_session(CreateSessionParams {
        task_id: &task.task_id,
        parent_session_id: None,
        provider: Provider::Claude,
        role: SessionRole::Lead,
        cwd: "/ws",
        title: "Lead",
    });

    let reg = SessionRegistration {
        task_id: task.task_id.clone(),
        parent_session_id: Some(lead.session_id.clone()),
        role: SessionRole::Coder,
        cwd: "/ws".into(),
        title: "Codex coder".into(),
        external_id: Some("thread_xyz".into()),
        transcript_path: None,
    };
    let sess = codex::register_session(&mut store, reg);

    assert_eq!(
        sess.parent_session_id.as_deref(),
        Some(lead.session_id.as_str())
    );
    let children = store.children_of_session(&lead.session_id);
    assert_eq!(children.len(), 1);
    assert_eq!(
        children[0].external_session_id.as_deref(),
        Some("thread_xyz")
    );
}

#[test]
fn bind_thread_id_updates_existing_session() {
    let mut store = TaskGraphStore::new();
    let task = store.create_task("/ws", "T1");
    let sess = store.create_session(CreateSessionParams {
        task_id: &task.task_id,
        parent_session_id: None,
        provider: Provider::Codex,
        role: SessionRole::Coder,
        cwd: "/ws",
        title: "Codex",
    });
    assert!(sess.external_session_id.is_none());

    let ok = codex::bind_thread_id(&mut store, &sess.session_id, "thread_late");
    assert!(ok);
    let updated = store.get_session(&sess.session_id).unwrap();
    assert_eq!(updated.external_session_id.as_deref(), Some("thread_late"));
}

#[test]
fn bind_thread_id_returns_false_for_missing_session() {
    let mut store = TaskGraphStore::new();
    assert!(!codex::bind_thread_id(&mut store, "no-such", "thread_x"));
}

// ── register_on_launch (integration with DaemonState) ──────

use crate::daemon::DaemonState;

#[test]
fn register_on_launch_creates_session_with_thread_id() {
    let mut s = DaemonState::new();
    let task = s.task_graph.create_task("/ws", "T1");
    let tid = task.task_id.clone();
    s.task_graph
        .update_task_status(&tid, TaskStatus::Implementing);
    s.set_active_task(Some(tid.clone()));

    codex::register_on_launch(&mut s, "coder", "/ws", "thread_abc");

    // Session registered with correct external_session_id
    let task = s.task_graph.get_task(&tid).unwrap();
    let coder_sid = task
        .current_coder_session_id
        .as_ref()
        .expect("coder session set");
    let sess = s.task_graph.get_session(coder_sid).unwrap();
    assert_eq!(sess.provider, Provider::Codex);
    assert_eq!(sess.role, SessionRole::Coder);
    assert_eq!(sess.external_session_id.as_deref(), Some("thread_abc"));
    assert_eq!(sess.task_id, tid);
}

#[test]
fn register_on_launch_noop_without_active_task() {
    let mut s = DaemonState::new();
    // No active task — should not panic or create anything
    codex::register_on_launch(&mut s, "coder", "/ws", "thread_xyz");
    assert!(s.task_graph.list_tasks().is_empty());
}

#[test]
fn register_on_launch_links_to_lead_session() {
    let mut s = DaemonState::new();
    let task = s.task_graph.create_task("/ws", "T1");
    let tid = task.task_id.clone();
    let lead = s.task_graph.create_session(CreateSessionParams {
        task_id: &tid,
        parent_session_id: None,
        provider: Provider::Claude,
        role: SessionRole::Lead,
        cwd: "/ws",
        title: "Lead",
    });
    s.task_graph.set_lead_session(&tid, &lead.session_id);
    s.task_graph
        .update_task_status(&tid, TaskStatus::Implementing);
    s.set_active_task(Some(tid.clone()));

    codex::register_on_launch(&mut s, "coder", "/ws", "thread_child");

    let task = s.task_graph.get_task(&tid).unwrap();
    let coder_sid = task.current_coder_session_id.as_ref().unwrap();
    let sess = s.task_graph.get_session(coder_sid).unwrap();
    assert_eq!(
        sess.parent_session_id.as_deref(),
        Some(lead.session_id.as_str())
    );
    assert_eq!(s.task_graph.children_of_session(&lead.session_id).len(), 1);
}

// ── adapter trait shape (future-proofing) ──────────────────

#[test]
fn codex_adapter_has_expected_interface() {
    let _ =
        codex::register_session as fn(&mut TaskGraphStore, SessionRegistration) -> SessionHandle;
    let _ = codex::bind_thread_id as fn(&mut TaskGraphStore, &str, &str) -> bool;
    let _ = codex::list_threads;
    let _ = codex::fork_thread;
    let _ = codex::archive_thread;
}

#[test]
fn map_thread_page_response_maps_history_entries() {
    let page = codex::map_thread_page_response(
        &json!({
            "data": [
                {
                    "id": "thr_a",
                    "preview": "Create a TUI",
                    "ephemeral": false,
                    "modelProvider": "openai",
                    "createdAt": 1730831111_u64,
                    "updatedAt": 1730832222_u64,
                    "name": "TUI prototype",
                    "status": { "type": "notLoaded" }
                },
                {
                    "id": "thr_b",
                    "preview": "Fix tests",
                    "ephemeral": true,
                    "modelProvider": "openai",
                    "createdAt": 1730750000_u64,
                    "updatedAt": 1730750100_u64,
                    "status": { "type": "active", "activeFlags": ["waitingOnApproval"] }
                }
            ],
            "nextCursor": "cursor_2"
        }),
        false,
    )
    .unwrap();

    assert_eq!(
        page,
        ProviderHistoryPage {
            entries: vec![
                ProviderHistoryEntry {
                    provider: Provider::Codex,
                    external_id: "thr_a".into(),
                    title: Some("TUI prototype".into()),
                    preview: Some("Create a TUI".into()),
                    cwd: None,
                    archived: false,
                    created_at: 1730831111,
                    updated_at: 1730832222,
                    status: SessionStatus::Paused,
                    normalized_session_id: None,
                    normalized_task_id: None,
                },
                ProviderHistoryEntry {
                    provider: Provider::Codex,
                    external_id: "thr_b".into(),
                    title: None,
                    preview: Some("Fix tests".into()),
                    cwd: None,
                    archived: false,
                    created_at: 1730750000,
                    updated_at: 1730750100,
                    status: SessionStatus::Active,
                    normalized_session_id: None,
                    normalized_task_id: None,
                },
            ],
            next_cursor: Some("cursor_2".into()),
        }
    );
}

#[test]
fn map_thread_page_response_rejects_missing_thread_id() {
    let err = codex::map_thread_page_response(
        &json!({
            "data": [{ "preview": "oops", "status": { "type": "idle" } }],
            "nextCursor": null
        }),
        true,
    )
    .unwrap_err();

    assert!(err.contains("missing thread id"));
}

#[test]
fn list_local_sessions_reads_workspace_history_without_app_server() {
    let base = std::env::temp_dir().join(format!("codex-local-history-{}", Uuid::new_v4()));
    let sessions_dir = base.join("sessions").join("2026").join("04").join("01");
    fs::create_dir_all(&sessions_dir).unwrap();

    let matching = sessions_dir.join("rollout-2026-04-01T14-31-53-thread_ws.jsonl");
    fs::write(
        &matching,
        concat!(
            "{\"timestamp\":\"2026-04-01T06:31:53.678Z\",\"type\":\"session_meta\",",
            "\"payload\":{\"id\":\"thread_ws\",\"timestamp\":\"2026-04-01T06:31:53.678Z\",",
            "\"cwd\":\"/tmp/ws\"}}\n",
            "{\"timestamp\":\"2026-04-01T06:32:05.995Z\",\"type\":\"response_item\",",
            "\"payload\":{\"type\":\"message\",\"role\":\"user\",\"content\":[",
            "{\"type\":\"input_text\",\"text\":\"Restore the workspace history panel\"}]}}\n",
            "{\"timestamp\":\"2026-04-01T06:32:10.995Z\",\"type\":\"event_msg\",",
            "\"payload\":{\"type\":\"agent_message\",\"message\":\"Investigating provider history.\"}}\n"
        ),
    )
    .unwrap();

    let other = sessions_dir.join("rollout-2026-04-01T14-31-53-thread_other.jsonl");
    fs::write(
        &other,
        concat!(
            "{\"timestamp\":\"2026-04-01T06:31:53.678Z\",\"type\":\"session_meta\",",
            "\"payload\":{\"id\":\"thread_other\",\"timestamp\":\"2026-04-01T06:31:53.678Z\",",
            "\"cwd\":\"/tmp/elsewhere\"}}\n"
        ),
    )
    .unwrap();

    let page =
        codex::list_local_sessions("/tmp/ws", Some(base.join("sessions").as_path())).unwrap();

    assert_eq!(page.next_cursor, None);
    assert_eq!(page.entries.len(), 1);
    assert_eq!(page.entries[0].external_id, "thread_ws");
    assert_eq!(
        page.entries[0].title.as_deref(),
        Some("Restore the workspace history panel")
    );
    assert_eq!(
        page.entries[0].preview.as_deref(),
        Some("Investigating provider history.")
    );
    assert_eq!(page.entries[0].cwd.as_deref(), Some("/tmp/ws"));
    assert_eq!(page.entries[0].provider, Provider::Codex);

    let _ = fs::remove_dir_all(base);
}

#[test]
fn list_local_sessions_ignores_environment_context_and_uses_user_message_summary() {
    let base = std::env::temp_dir().join(format!("codex-local-history-{}", Uuid::new_v4()));
    let sessions_dir = base.join("sessions").join("2026").join("04").join("01");
    fs::create_dir_all(&sessions_dir).unwrap();

    let transcript = sessions_dir.join("rollout-2026-04-01T14-31-53-thread_ws.jsonl");
    fs::write(
        &transcript,
        concat!(
            "{\"timestamp\":\"2026-04-01T06:31:53.678Z\",\"type\":\"session_meta\",",
            "\"payload\":{\"id\":\"thread_ws\",\"timestamp\":\"2026-04-01T06:31:53.678Z\",",
            "\"cwd\":\"/tmp/ws\"}}\n",
            "{\"timestamp\":\"2026-04-01T06:32:02.261Z\",\"type\":\"response_item\",",
            "\"payload\":{\"type\":\"message\",\"role\":\"user\",\"content\":[",
            "{\"type\":\"input_text\",\"text\":\"<environment_context>\\n  <cwd>/tmp/ws</cwd>\\n</environment_context>\"}]}}\n",
            "{\"timestamp\":\"2026-04-01T06:32:03.261Z\",\"type\":\"event_msg\",",
            "\"payload\":{\"type\":\"user_message\",\"message\":\"都回答我1\"}}\n",
            "{\"timestamp\":\"2026-04-01T06:32:05.625Z\",\"type\":\"event_msg\",",
            "\"payload\":{\"type\":\"agent_message\",\"message\":\"{\\\"message\\\":\\\"1\\\",\\\"send_to\\\":\\\"user\\\",\\\"status\\\":\\\"done\\\"}\"}}\n"
        ),
    )
    .unwrap();

    let page =
        codex::list_local_sessions("/tmp/ws", Some(base.join("sessions").as_path())).unwrap();

    assert_eq!(page.entries.len(), 1);
    assert_eq!(page.entries[0].title.as_deref(), Some("都回答我1"));
    assert_eq!(page.entries[0].preview.as_deref(), Some("1"));

    let _ = fs::remove_dir_all(base);
}

#[test]
fn register_forked_session_preserves_task_and_parent_linkage() {
    let mut store = TaskGraphStore::new();
    let task = store.create_task("/ws", "T1");
    let lead = store.create_session(CreateSessionParams {
        task_id: &task.task_id,
        parent_session_id: None,
        provider: Provider::Claude,
        role: SessionRole::Lead,
        cwd: "/ws",
        title: "Lead",
    });
    let source = store.create_session(CreateSessionParams {
        task_id: &task.task_id,
        parent_session_id: Some(&lead.session_id),
        provider: Provider::Codex,
        role: SessionRole::Coder,
        cwd: "/ws",
        title: "Source coder",
    });

    let forked = codex::register_forked_session(
        &mut store,
        &source.session_id,
        "thread_forked",
        Some("Forked coder"),
    )
    .unwrap();

    assert_eq!(forked.task_id, task.task_id);
    assert_eq!(forked.provider, Provider::Codex);
    assert_eq!(forked.role, SessionRole::Coder);
    assert_eq!(
        forked.parent_session_id.as_deref(),
        Some(lead.session_id.as_str())
    );
    assert_eq!(forked.external_session_id.as_deref(), Some("thread_forked"));
    assert_eq!(forked.title, "Forked coder");
}

#[test]
fn build_resume_target_requires_external_thread_id() {
    let session = SessionHandle {
        session_id: "sess_1".into(),
        task_id: "task_1".into(),
        parent_session_id: None,
        provider: Provider::Codex,
        role: SessionRole::Coder,
        external_session_id: None,
        transcript_path: None,
        status: SessionStatus::Paused,
        cwd: "/ws".into(),
        title: "Codex coder".into(),
        created_at: 1,
        updated_at: 2,
    };

    let err = codex::build_resume_target(&session).unwrap_err();
    assert!(err.contains("missing external thread id"));
}

#[test]
fn mark_archived_updates_normalized_session_status() {
    let mut store = TaskGraphStore::new();
    let task = store.create_task("/ws", "T1");
    let session = store.create_session(CreateSessionParams {
        task_id: &task.task_id,
        parent_session_id: None,
        provider: Provider::Codex,
        role: SessionRole::Coder,
        cwd: "/ws",
        title: "Coder",
    });

    assert!(codex::mark_session_archived(
        &mut store,
        &session.session_id
    ));
    assert_eq!(
        store.get_session(&session.session_id).unwrap().status,
        SessionStatus::Completed
    );
}
