use crate::daemon::DaemonState;

pub fn sync_claude_launch_into_active_task(
    state: &mut DaemonState,
    role_id: &str,
    cwd: &str,
    session_id: &str,
    transcript_path: &str,
) -> Option<String> {
    crate::daemon::provider::claude::register_on_launch(
        state,
        role_id,
        cwd,
        session_id,
        transcript_path,
    );
    state.active_task_id.clone()
}
