use super::process::ClaudeLaunchOpts;
use super::runtime::{current_session_id, emit_runtime_health, kill_child, spawn_runtime};
use crate::daemon::{gui, SharedState};
use std::sync::Arc;
use tauri::AppHandle;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

pub async fn wait_for_ws_disconnect(
    state: &SharedState,
    task_id: &str,
    agent_id: &str,
    epoch: u64,
) -> bool {
    loop {
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
        let daemon = state.read().await;
        if daemon.claude_task_epoch_for_agent(task_id, agent_id) != Some(epoch) {
            return false;
        }
        if !daemon.is_claude_agent_online(task_id, agent_id) {
            return true;
        }
    }
}

pub async fn recover_ws_connection(
    child: &mut Arc<Mutex<Option<tokio::process::Child>>>,
    epoch: &mut u64,
    base_opts: &ClaudeLaunchOpts,
    task_id: &str,
    agent_id: &str,
    state: &SharedState,
    app: &AppHandle,
    cancel: &CancellationToken,
) -> bool {
    let session_id = current_session_id(base_opts);
    for attempt in 1..=crate::daemon::control::claude_sdk_handler::MAX_WS_RECONNECT_ATTEMPTS {
        let Some(delay_ms) = crate::daemon::control::claude_sdk_handler::reconnect_delay_ms(attempt)
        else {
            break;
        };
        let reconnecting_message = format!(
            "Claude reconnecting ({attempt}/{})",
            crate::daemon::control::claude_sdk_handler::MAX_WS_RECONNECT_ATTEMPTS
        );
        emit_runtime_health(
            state,
            app,
            crate::daemon::types::RuntimeHealthLevel::Warning,
            reconnecting_message.clone(),
        )
        .await;
        gui::emit_system_log(app, "warn", &format!("[Claude SDK] {reconnecting_message}"));
        tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
        if cancel.is_cancelled() {
            return false;
        }

        kill_child(child).await;
        let mut next_opts = base_opts.clone();
        next_opts.resume = Some(session_id.clone());
        next_opts.launch_nonce = uuid::Uuid::new_v4().to_string();
        match spawn_runtime(&next_opts, task_id, agent_id, state.clone(), app.clone()).await {
            Ok((next_child, next_epoch)) => {
                *child = next_child;
                *epoch = next_epoch;
                gui::emit_system_log(app, "info", "[Claude SDK] reconnected");
                return true;
            }
            Err(err) => {
                gui::emit_system_log(
                    app,
                    "error",
                    &format!("[Claude SDK] reconnect attempt {attempt} failed: {err}"),
                );
            }
        }
    }

    emit_runtime_health(
        state,
        app,
        crate::daemon::types::RuntimeHealthLevel::Error,
        format!(
            "Claude reconnect failed after {} attempts",
            crate::daemon::control::claude_sdk_handler::MAX_WS_RECONNECT_ATTEMPTS
        ),
    )
    .await;
    let (is_current, affected_task_id) = {
        let mut s = state.write().await;
        let is_current = s.claude_task_epoch_for_agent(task_id, agent_id) == Some(*epoch);
        let tid = s.invalidate_claude_agent_session_if_current(task_id, agent_id, *epoch);
        (is_current, tid)
    };
    if is_current {
        gui::emit_agent_status(app, "claude", false, None, None);
        gui::emit_claude_stream(app, None, None, gui::ClaudeStreamPayload::Reset);
    }
    if let Some(tid) = affected_task_id {
        crate::daemon::gui_task::emit_task_context_events(state, app, &tid).await;
    }
    false
}
