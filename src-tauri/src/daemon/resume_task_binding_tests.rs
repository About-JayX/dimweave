use crate::daemon::task_graph::types::{CreateSessionParams, Provider, SessionRole};
use crate::daemon::DaemonState;
use crate::daemon::{is_retryable_codex_port_error, resume_task_id_for_claude_session};

#[test]
fn claude_resume_uses_session_task_when_active_task_differs() {
    let mut state = DaemonState::new();
    let origin = state.task_graph.create_task("/ws/origin", "Origin");
    let session = state.task_graph.create_session(CreateSessionParams {
        task_id: &origin.task_id,
        parent_session_id: None,
        provider: Provider::Claude,
        role: SessionRole::Lead,
        cwd: "/ws/origin",
        title: "Claude lead",
        agent_id: None,
    });
    let other = state.task_graph.create_task("/ws/other", "Other");

    let resume_task_id = resume_task_id_for_claude_session(Some(other.task_id.as_str()), &session);

    assert_eq!(resume_task_id, origin.task_id);
}

#[test]
fn claude_resume_uses_session_task_when_no_active_task_exists() {
    let mut state = DaemonState::new();
    let origin = state.task_graph.create_task("/ws/origin", "Origin");
    let session = state.task_graph.create_session(CreateSessionParams {
        task_id: &origin.task_id,
        parent_session_id: None,
        provider: Provider::Claude,
        role: SessionRole::Lead,
        cwd: "/ws/origin",
        title: "Claude lead",
        agent_id: None,
    });

    let resume_task_id = resume_task_id_for_claude_session(None, &session);

    assert_eq!(resume_task_id, origin.task_id);
}

#[test]
fn codex_port_in_use_errors_are_retryable() {
    assert!(is_retryable_codex_port_error(
        "Port 4502 still in use after 5s"
    ));
    assert!(is_retryable_codex_port_error(
        "Port 4500 still in use after 250ms"
    ));
    assert!(!is_retryable_codex_port_error(
        "Codex provider_auth is in api_key mode but no api_key is set"
    ));
    assert!(!is_retryable_codex_port_error("WebSocket handshake failed"));
}
