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
    session_epoch: u64,
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
                        let cleared_current = {
                            let mut daemon = state.write().await;
                            daemon.clear_codex_session_if_current(session_epoch)
                        };
                        if cleared_current {
                            gui::emit_agent_status(&app, "codex", false, None, None);
                            gui::emit_system_log(
                                &app,
                                "warn",
                                &format!("[Codex] exited: {status}"),
                            );
                        }
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
