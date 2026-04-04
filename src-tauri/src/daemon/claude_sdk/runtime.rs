use super::{process, stdio};
use crate::daemon::{gui, SharedState};
use process::ClaudeLaunchOpts;
use serde_json::Value;
use std::sync::Arc;
use tauri::AppHandle;
use tokio::sync::{mpsc, oneshot, Mutex};

const CLAUDE_RUNTIME_HEALTH_SOURCE: &str = "claude_sdk";

fn runtime_health(
    level: crate::daemon::types::RuntimeHealthLevel,
    message: impl Into<String>,
) -> crate::daemon::types::RuntimeHealthStatus {
    crate::daemon::types::RuntimeHealthStatus {
        level,
        source: CLAUDE_RUNTIME_HEALTH_SOURCE.into(),
        message: message.into(),
    }
}

pub fn current_session_id(opts: &ClaudeLaunchOpts) -> String {
    opts.resume
        .clone()
        .unwrap_or_else(|| opts.session_id.clone())
}

pub async fn emit_runtime_health(
    state: &SharedState,
    app: &AppHandle,
    level: crate::daemon::types::RuntimeHealthLevel,
    message: impl Into<String>,
) {
    let health = runtime_health(level, message);
    state.write().await.set_runtime_health(health.clone());
    gui::emit_runtime_health(app, Some(health));
}

async fn prepare_launch_channels(
    state: SharedState,
    app: AppHandle,
    launch_nonce: String,
) -> (oneshot::Receiver<mpsc::Sender<String>>, u64) {
    let (ready_tx, ready_rx) = oneshot::channel::<mpsc::Sender<String>>();
    let (event_tx, mut event_rx) = mpsc::channel::<Vec<Value>>(256);
    let epoch = {
        let mut s = state.write().await;
        let epoch = s.begin_claude_sdk_launch(launch_nonce);
        s.claude_sdk_ready_tx = Some(ready_tx);
        s.claude_sdk_event_tx = Some(event_tx);
        epoch
    };
    let event_state = state.clone();
    let event_app = app.clone();
    tokio::spawn(async move {
        while let Some(events) = event_rx.recv().await {
            crate::daemon::control::claude_sdk_handler::process_sdk_events(
                &event_state,
                &event_app,
                events,
            )
            .await;
        }
    });
    (ready_rx, epoch)
}

pub async fn spawn_runtime(
    opts: &ClaudeLaunchOpts,
    state: SharedState,
    app: AppHandle,
) -> anyhow::Result<(Arc<Mutex<Option<tokio::process::Child>>>, u64)> {
    let session_id = current_session_id(opts);
    let role_id = opts.role.clone().unwrap_or_else(|| "lead".into());
    let is_resume = opts.resume.is_some();
    let (ready_rx, epoch) =
        prepare_launch_channels(state.clone(), app.clone(), opts.launch_nonce.clone()).await;

    let mut child = match process::spawn_claude(opts) {
        Ok(child) => child,
        Err(err) => {
            state
                .write()
                .await
                .invalidate_claude_sdk_session_if_current(epoch);
            return Err(err);
        }
    };
    stdio::spawn_stdio_drainers(child.stdout.take(), child.stderr.take());
    let child_arc = Arc::new(Mutex::new(Some(child)));
    gui::emit_system_log(
        &app,
        "info",
        &format!("[Claude SDK] spawned session={session_id} role={role_id} resume={is_resume}"),
    );
    gui::emit_system_log(
        &app,
        "info",
        &format!("[Claude Trace] {}", process::format_launch_trace(opts)),
    );

    let connected = tokio::select! {
        result = ready_rx => result.is_ok(),
        _ = tokio::time::sleep(std::time::Duration::from_secs(30)) => false,
        _ = poll_child_exit(&child_arc, false) => false,
    };

    if !connected {
        kill_child(&child_arc).await;
        state
            .write()
            .await
            .invalidate_claude_sdk_session_if_current(epoch);
        anyhow::bail!("Claude SDK did not connect within 30s");
    }

    let provider_session = crate::daemon::types::ProviderConnectionState {
        provider: crate::daemon::task_graph::types::Provider::Claude,
        external_session_id: session_id.clone(),
        cwd: opts.cwd.clone(),
        connection_mode: if is_resume {
            crate::daemon::types::ProviderConnectionMode::Resumed
        } else {
            crate::daemon::types::ProviderConnectionMode::New
        },
    };
    {
        let mut s = state.write().await;
        s.set_provider_connection("claude", provider_session.clone());
        s.claude_role = role_id.clone();
        s.clear_runtime_health();
    }
    gui::emit_runtime_health(&app, None);
    gui::emit_agent_status(&app, "claude", true, None, Some(provider_session.clone()));
    gui::emit_system_log(
        &app,
        "info",
        &format!("[Claude SDK] ready session={session_id}"),
    );
    gui::emit_system_log(
        &app,
        "info",
        &format!(
            "[Claude Trace] chain=ready session={} provider_session={{provider=claude,external_session_id={},cwd={},connection_mode={}}}",
            session_id,
            provider_session.external_session_id,
            provider_session.cwd,
            provider_session.connection_mode.as_str(),
        ),
    );

    Ok((child_arc, epoch))
}

pub async fn kill_child(child: &Arc<Mutex<Option<tokio::process::Child>>>) {
    if let Some(mut process) = child.lock().await.take() {
        let _ = process.kill().await;
        let _ = process.wait().await;
    }
}

pub async fn poll_child_exit(child: &Arc<Mutex<Option<tokio::process::Child>>>, take: bool) {
    let interval = if take { 500 } else { 200 };
    loop {
        tokio::time::sleep(std::time::Duration::from_millis(interval)).await;
        let mut guard = child.lock().await;
        if let Some(ref mut c) = *guard {
            match c.try_wait() {
                Ok(Some(_)) | Err(_) => {
                    if take {
                        *guard = None;
                    }
                    return;
                }
                Ok(None) => {}
            }
        } else {
            return;
        }
    }
}
