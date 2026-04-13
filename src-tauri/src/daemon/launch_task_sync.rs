use crate::daemon::task_graph::types::Provider;
use crate::daemon::DaemonState;

pub fn sync_claude_launch_into_task(
    state: &mut DaemonState,
    task_id: &str,
    role_id: &str,
    cwd: &str,
    session_id: &str,
    transcript_path: &str,
) -> Option<String> {
    if let Some(existing_session_id) = state
        .task_graph
        .find_session_by_external_id(Provider::Claude, session_id)
        .map(|s| s.session_id.clone())
    {
        // Known history entry: refresh transcript path and resume the normalized session.
        // resume_session() switches active_task_id to the session's task.
        let _ = state
            .task_graph
            .set_transcript_path(&existing_session_id, transcript_path);
        state.resume_session(&existing_session_id).ok()
    } else {
        // Unknown session: register on the explicit task.
        crate::daemon::provider::claude::register_on_launch(
            state,
            task_id,
            role_id,
            cwd,
            session_id,
            transcript_path,
        );
        Some(task_id.to_string())
    }
}

pub fn sync_codex_launch_into_task(
    state: &mut DaemonState,
    role_id: &str,
    cwd: &str,
    thread_id: &str,
) -> Option<String> {
    if let Some(existing_session_id) = state
        .task_graph
        .find_session_by_external_id(Provider::Codex, thread_id)
        .map(|s| s.session_id.clone())
    {
        // Known history entry: resume the normalized session.
        // resume_session() switches active_task_id to the session's task.
        state.resume_session(&existing_session_id).ok()
    } else {
        // Unknown thread: register on the currently active task.
        crate::daemon::provider::codex::register_on_launch(state, role_id, cwd, thread_id);
        state.active_task_id.clone()
    }
}
