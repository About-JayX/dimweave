use crate::daemon::task_graph::types::Provider;
use crate::daemon::DaemonState;

pub fn sync_claude_launch_into_task(
    state: &mut DaemonState,
    task_id: &str,
    role_id: &str,
    cwd: &str,
    session_id: &str,
    transcript_path: &str,
    agent_id: Option<&str>,
) -> Option<String> {
    if let Some(existing_session_id) = state
        .task_graph
        .find_session_by_external_id(Provider::Claude, session_id)
        .map(|s| s.session_id.clone())
    {
        // Known history entry: refresh transcript path, bind agent_id, and resume.
        let _ = state
            .task_graph
            .set_transcript_path(&existing_session_id, transcript_path);
        if let Some(aid) = agent_id {
            let _ = state.task_graph.set_session_agent_id(&existing_session_id, aid);
        }
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
            agent_id,
        );
        Some(task_id.to_string())
    }
}

pub fn sync_codex_launch_into_task(
    state: &mut DaemonState,
    task_id: &str,
    role_id: &str,
    cwd: &str,
    thread_id: &str,
    agent_id: Option<&str>,
) -> Option<String> {
    if let Some(existing_session_id) = state
        .task_graph
        .find_session_by_external_id(Provider::Codex, thread_id)
        .map(|s| s.session_id.clone())
    {
        // Known history entry: bind agent_id and resume the normalized session.
        if let Some(aid) = agent_id {
            let _ = state.task_graph.set_session_agent_id(&existing_session_id, aid);
        }
        state.resume_session(&existing_session_id).ok()
    } else {
        // Unknown thread: register on the explicit task_id from the launch.
        crate::daemon::provider::codex::register_on_launch(state, task_id, role_id, cwd, thread_id, agent_id);
        Some(task_id.to_string())
    }
}
