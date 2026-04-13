use crate::daemon::{gui, SharedState};
use std::future::Future;
use std::sync::Arc;
use tauri::AppHandle;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

pub(super) async fn ensure_port_available<F, Fut>(
    codex_port: u16,
    timeout: std::time::Duration,
    mut cleanup: F,
) -> anyhow::Result<()>
where
    F: FnMut(u16) -> Fut,
    Fut: Future<Output = ()>,
{
    let deadline = tokio::time::Instant::now() + timeout;
    let mut cleanup_attempted = false;

    while tokio::net::TcpStream::connect(format!("127.0.0.1:{codex_port}"))
        .await
        .is_ok()
    {
        if !cleanup_attempted {
            cleanup(codex_port).await;
            cleanup_attempted = true;
        }
        if tokio::time::Instant::now() >= deadline {
            let suffix = if timeout.as_secs() > 0 {
                format!(" after {}s", timeout.as_secs())
            } else {
                format!(" after {}ms", timeout.as_millis())
            };
            anyhow::bail!("Port {codex_port} still in use{suffix}");
        }
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }

    Ok(())
}

pub(super) fn spawn_health_monitor(
    child: Arc<Mutex<Option<tokio::process::Child>>>,
    task_id: String,
    session_epoch: u64,
    port: u16,
    exit_tx: tokio::sync::mpsc::UnboundedSender<super::CodexExitNotice>,
    state: SharedState,
    app: AppHandle,
    cancel: CancellationToken,
) {
    tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = cancel.cancelled() => return,
                _ = tokio::time::sleep(std::time::Duration::from_secs(2)) => {}
            }
            let mut guard = child.lock().await;
            if let Some(ref mut proc) = *guard {
                match proc.try_wait() {
                    Ok(Some(status)) => {
                        eprintln!("[Codex] health_monitor: process exited with status={status}");
                        cancel.cancel();
                        let cleanup = {
                            let mut daemon = state.write().await;
                            let cleared = daemon.clear_codex_task_session(&task_id, session_epoch);
                            let any_online = daemon.is_codex_online();
                            (cleared, any_online)
                        };
                        if cleanup.0.is_some() && !cleanup.1 {
                            gui::emit_agent_status(&app, "codex", false, None, None);
                        }
                        if cleanup.0.is_some() {
                            gui::emit_system_log(
                                &app,
                                "warn",
                                &format!("[Codex] exited: {status}"),
                            );
                        }
                        if let Some(tid) = cleanup.0 {
                            crate::daemon::gui_task::emit_task_context_events(
                                &state, &app, &tid,
                            )
                            .await;
                        }
                        let _ = exit_tx.send(super::CodexExitNotice {
                            task_id: task_id.clone(),
                            port,
                            launch_id: session_epoch,
                        });
                        return;
                    }
                    Ok(None) => {}
                    Err(_) => return,
                }
            } else {
                return;
            }
        }
    });
}
