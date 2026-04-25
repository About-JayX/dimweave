pub mod handler;
pub(crate) mod handshake;
pub mod lifecycle;
pub mod port_pool;
mod runtime;
pub mod session;
mod structured_output;
pub(crate) mod ws_client;
mod ws_helpers;

use crate::daemon::{gui, role_config, SharedState};
use runtime::{ensure_port_available, spawn_health_monitor};
use session::SessionOpts;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::AppHandle;
use tokio::sync::{mpsc, Mutex};
use tokio_util::sync::CancellationToken;

const API_KEY_HOME_PREFIX: &str = "dimweave-codex-apikey-";

/// RAII wrapper for the temp CODEX_HOME we provision for API-key auth.
/// The directory gets removed whenever this value is dropped, so every
/// bail path (handshake failure, supersede, spawn error, normal stop)
/// cleans up automatically — no manual cleanup call needed.
pub(crate) struct TempCodexHome(PathBuf);

impl TempCodexHome {
    pub(crate) fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempCodexHome {
    fn drop(&mut self) {
        // Belt-and-suspenders: only delete paths we recognise as ours.
        if self
            .0
            .file_name()
            .and_then(|n| n.to_str())
            .map_or(false, |n| n.starts_with(API_KEY_HOME_PREFIX))
        {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
}

/// Provision a per-session temp CODEX_HOME configured for API-key auth.
///
/// Codex reads `<CODEX_HOME>/auth.json` before looking at `OPENAI_API_KEY`
/// env; when the user is signed in via ChatGPT the real `~/.codex/auth.json`
/// pins `auth_mode: "chatgpt"` and our env override is ignored. Instead we
/// hand Codex a fresh CODEX_HOME that owns an `auth_mode: "apikey"` file
/// with our key. Thread history is preserved by symlinking `sessions/`
/// back to the user's real codex home.
fn build_api_key_codex_home(session_id: &str, api_key: &str) -> anyhow::Result<TempCodexHome> {
    let base = std::env::temp_dir().join(format!(
        "{API_KEY_HOME_PREFIX}{}-{}",
        std::process::id(),
        session_id
    ));
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(&base)?;

    let auth = serde_json::json!({
        "auth_mode": "apikey",
        "OPENAI_API_KEY": api_key,
    });
    let auth_path = base.join("auth.json");
    std::fs::write(&auth_path, serde_json::to_string_pretty(&auth)?)?;
    // Tight perms — mirror Codex's own 0600 on auth.json.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&auth_path, std::fs::Permissions::from_mode(0o600));
    }

    // Symlink sessions/ so thread history + resume still work.
    if let Some(home) = dirs::home_dir() {
        let real_sessions = home.join(".codex").join("sessions");
        if real_sessions.exists() {
            let temp_sessions = base.join("sessions");
            #[cfg(unix)]
            {
                if let Err(e) = std::os::unix::fs::symlink(&real_sessions, &temp_sessions) {
                    eprintln!(
                        "[Codex] sessions symlink failed ({e}); resume may not work for this session"
                    );
                }
            }
        }
    }

    Ok(TempCodexHome(base))
}

/// Remove orphaned api-key CODEX_HOMEs from crashed prior runs.
/// Only deletes directories whose embedded PID is no longer alive.
pub fn prune_orphan_api_key_codex_homes() {
    let Ok(entries) = std::fs::read_dir(std::env::temp_dir()) else { return };
    let current_pid = std::process::id();
    for entry in entries.flatten() {
        let Some(name) = entry.file_name().to_str().map(str::to_string) else { continue };
        let Some(tail) = name.strip_prefix(API_KEY_HOME_PREFIX) else { continue };
        // Format: "<pid>-<sid>" — grab just the PID.
        let Some((pid_str, _)) = tail.split_once('-') else { continue };
        let Ok(pid) = pid_str.parse::<u32>() else { continue };
        if pid == current_pid {
            continue;
        }
        if is_pid_alive(pid) {
            continue;
        }
        let _ = std::fs::remove_dir_all(entry.path());
    }
}

#[cfg(unix)]
fn is_pid_alive(pid: u32) -> bool {
    // signal 0 is the null signal: returns Ok if the process exists, Err(ESRCH)
    // if it doesn't. We don't actually deliver anything.
    unsafe { libc::kill(pid as i32, 0) == 0 || std::io::Error::last_os_error().raw_os_error() != Some(libc::ESRCH) }
}

#[cfg(not(unix))]
fn is_pid_alive(_pid: u32) -> bool {
    // Conservative: assume alive so we don't nuke an active session.
    true
}

/// Notification sent from session/runtime tasks when a Codex session
/// exits naturally (process death or session loop end). The daemon loop
/// uses this to release the port lease and remove the stale task handle.
#[derive(Debug)]
pub struct CodexExitNotice {
    pub task_id: String,
    pub agent_id: String,
    pub port: u16,
    pub launch_id: u64,
}

pub struct CodexHandle {
    process: Arc<Mutex<Option<tokio::process::Child>>>,
    session_mgr: Arc<Mutex<crate::daemon::session_manager::SessionManager>>,
    session_id: String,
    cancel: CancellationToken,
    /// Temp CODEX_HOME for api-key mode (RAII: dropped with this handle).
    _api_key_home: Option<TempCodexHome>,
    pub port: u16,
    pub launch_id: u64,
    pub task_id: String,
    pub agent_id: String,
}

pub struct StartOpts {
    pub task_id: String,
    pub agent_id: String,
    pub role_id: String,
    pub cwd: String,
    pub model: Option<String>,
    pub effort: Option<String>,
    pub launch_epoch: u64,
    pub codex_port: u16,
}

pub struct ResumeOpts {
    pub task_id: String,
    pub agent_id: String,
    pub role_id: String,
    pub cwd: String,
    pub thread_id: String,
    pub launch_epoch: u64,
    pub codex_port: u16,
}

enum LaunchMode {
    New(SessionOpts),
    Resume { role_id: String, thread_id: String },
}

impl CodexHandle {
    pub async fn stop(&self) {
        self.cancel.cancel();
        if let Some(mut c) = self.process.lock().await.take() {
            lifecycle::stop(&mut c, self.port).await;
        }
        self.session_mgr
            .lock()
            .await
            .cleanup_session(&self.session_id);
        // Temp CODEX_HOME (_api_key_home) cleans itself up via Drop whenever
        // this CodexHandle is dropped — no manual rm-rf call needed here.
    }
}

/// Start a Codex app-server for the given role and wire it up to the daemon state.
pub async fn start(
    opts: StartOpts,
    state: SharedState,
    app: AppHandle,
    exit_tx: tokio::sync::mpsc::UnboundedSender<CodexExitNotice>,
) -> anyhow::Result<CodexHandle> {
    let StartOpts {
        task_id,
        agent_id,
        role_id,
        cwd,
        model,
        effort,
        launch_epoch,
        codex_port,
    } = opts;
    let (sandbox_mode, approval_policy, network_access, base_instructions) =
        resolve_role_launch_config(&role_id, model.as_deref());
    let session_opts = SessionOpts {
        role_id: role_id.clone(),
        cwd: cwd.clone(),
        model,
        effort,
        sandbox_mode: Some(sandbox_mode.clone()),
        network_access,
        base_instructions,
    };
    launch(
        LaunchMode::New(session_opts),
        task_id,
        agent_id,
        role_id,
        cwd,
        launch_epoch,
        codex_port,
        sandbox_mode,
        approval_policy,
        state,
        app,
        exit_tx,
    )
    .await
}

pub async fn resume(
    opts: ResumeOpts,
    state: SharedState,
    app: AppHandle,
    exit_tx: tokio::sync::mpsc::UnboundedSender<CodexExitNotice>,
) -> anyhow::Result<CodexHandle> {
    let ResumeOpts {
        task_id,
        agent_id,
        role_id,
        cwd,
        thread_id,
        launch_epoch,
        codex_port,
    } = opts;
    let (sandbox_mode, approval_policy, _, _) = resolve_role_launch_config(&role_id, None);
    launch(
        LaunchMode::Resume {
            role_id: role_id.clone(),
            thread_id,
        },
        task_id,
        agent_id,
        role_id,
        cwd,
        launch_epoch,
        codex_port,
        sandbox_mode,
        approval_policy,
        state,
        app,
        exit_tx,
    )
    .await
}

fn resolve_role_launch_config(
    role_id: &str,
    model: Option<&str>,
) -> (String, String, bool, Option<String>) {
    let role_config = match model {
        Some(model) => role_config::get_role_for_model(role_id, Some(model)),
        None => role_config::get_role(role_id),
    };
    if let Some(rc) = role_config {
        (
            rc.sandbox_mode.to_string(),
            rc.approval_policy.to_string(),
            rc.network_access,
            Some(rc.base_instructions.to_string()),
        )
    } else {
        ("workspace-write".into(), "never".into(), false, None)
    }
}

async fn launch(
    mode: LaunchMode,
    task_id: String,
    agent_id: String,
    role_id: String,
    cwd: String,
    launch_epoch: u64,
    codex_port: u16,
    sandbox_mode: String,
    approval_policy: String,
    state: SharedState,
    app: AppHandle,
    exit_tx: tokio::sync::mpsc::UnboundedSender<CodexExitNotice>,
) -> anyhow::Result<CodexHandle> {
    let session_mgr = state.read().await.session_mgr.clone();
    let session_id = session_mgr.lock().await.next_session_id();
    let codex_home =
        session_mgr
            .lock()
            .await
            .create_session(&session_id, &sandbox_mode, &approval_policy)?;

    // If a previous Codex session crashed, a port-holder orphan may survive.
    // Proactively clean it before giving up on launch.
    ensure_port_available(codex_port, std::time::Duration::from_secs(5), |port| {
        lifecycle::kill_port_holder(port)
    })
    .await?;

    let provider_auth = state
        .read()
        .await
        .task_graph
        .get_provider_auth("codex")
        .cloned();

    // Hard guard: explicit API-key mode without a key is a configuration
    // error. Without this, apply_provider_auth silently short-circuits and
    // Codex falls back to ~/.codex/auth.json (i.e. the subscription auth),
    // which is the opposite of what the user asked for.
    if let Some(ref a) = provider_auth {
        if matches!(a.active_mode.as_deref(), Some("api_key"))
            && a.api_key
                .as_deref()
                .map_or(true, |k| k.trim().is_empty())
        {
            return Err(anyhow::anyhow!(
                "Codex provider_auth is in api_key mode but no api_key is set. \
                 Open Tools → Accounts ⚙ and either enter a key or switch back to Subscription."
            ));
        }
    }

    // In API-key mode we can't share the user's ~/.codex/auth.json (it's
    // locked to "chatgpt" auth_mode when they're logged in via ChatGPT).
    // Stand up a per-session temp CODEX_HOME that (a) owns an apikey-mode
    // auth.json with our key and (b) symlinks `sessions/` back to the
    // user's real codex home so thread history stays resumable.
    //
    // `api_key_home` is a `TempCodexHome` RAII handle: whenever it leaves
    // this scope (early return from handshake failure, supersede, spawn
    // error, etc.) its Drop wipes the temp dir — no manual cleanup needed.
    let api_key_home: Option<TempCodexHome> = provider_auth
        .as_ref()
        .filter(|a| matches!(a.active_mode.as_deref(), Some("api_key")))
        .and_then(|a| a.api_key.as_deref().filter(|k| !k.trim().is_empty()))
        .map(|key| build_api_key_codex_home(&session_id, key.trim()))
        .transpose()?;
    let effective_codex_home: &Path = api_key_home
        .as_ref()
        .map(TempCodexHome::path)
        .unwrap_or(&codex_home);

    let child = lifecycle::start(
        codex_port,
        effective_codex_home,
        &cwd,
        &sandbox_mode,
        &approval_policy,
        provider_auth.as_ref(),
    )
    .await?;
    let child_arc = Arc::new(Mutex::new(Some(child)));
    gui::emit_system_log(
        &app,
        "info",
        &format!("[Codex] spawned port={codex_port} role={role_id}"),
    );

    // Poll until Codex app-server is accepting connections (up to 10 s)
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(10);
    let mut poll_delay = std::time::Duration::from_millis(50);
    loop {
        if tokio::net::TcpStream::connect(format!("127.0.0.1:{codex_port}"))
            .await
            .is_ok()
        {
            break;
        }
        // Check if child process exited prematurely
        if let Some(ref mut child) = *child_arc.lock().await {
            if let Ok(Some(status)) = child.try_wait() {
                eprintln!(
                    "[Codex] FATAL: subprocess exited prematurely on port={codex_port} status={status}"
                );
                anyhow::bail!("Codex process exited prematurely with status: {status}");
            }
        }
        if tokio::time::Instant::now() >= deadline {
            eprintln!(
                "[Codex] FATAL: app-server did not bind port={codex_port} within 10s"
            );
            anyhow::bail!("Codex app-server did not start within 10 s");
        }
        tokio::time::sleep(poll_delay).await;
        poll_delay = (poll_delay * 2).min(std::time::Duration::from_millis(500));
    }

    let (inject_tx, inject_rx) = mpsc::channel::<(Vec<serde_json::Value>, bool)>(64);
    let cancel = CancellationToken::new();
    let (ready_tx, ready_rx) = tokio::sync::oneshot::channel::<String>();

    let state2 = state.clone();
    let app2 = app.clone();
    let cancel_session = cancel.clone();
    let is_new_session = matches!(&mode, LaunchMode::New(_));
    let task_id_session = task_id.clone();
    let agent_id_session = agent_id.clone();
    let session_exit_tx = exit_tx.clone();
    let session_exit_port = codex_port;
    let session_exit_task_id = task_id.clone();
    let session_exit_agent_id = agent_id.clone();
    let session_exit_launch_id = launch_epoch;
    tokio::spawn(async move {
        tokio::select! {
            _ = cancel_session.cancelled() => {}
            _ = async {
                match mode {
                    LaunchMode::New(opts) => {
                        session::run(codex_port, launch_epoch, task_id_session, agent_id_session, opts, state2, app2, inject_rx, ready_tx).await;
                    }
                    LaunchMode::Resume { role_id, thread_id } => {
                        session::resume(codex_port, launch_epoch, task_id_session, agent_id_session, role_id, thread_id, state2, app2, inject_rx, ready_tx).await;
                    }
                }
            } => {}
        }
        // Notify daemon loop so it can release port lease and remove handle
        let _ = session_exit_tx.send(CodexExitNotice {
            task_id: session_exit_task_id,
            agent_id: session_exit_agent_id,
            port: session_exit_port,
            launch_id: session_exit_launch_id,
        });
    });

    // Wait for session handshake to complete before declaring connected
    let thread_id = match ready_rx.await {
        Ok(tid) if !tid.is_empty() => tid,
        _ => {
            cancel.cancel();
            if let Some(mut child) = child_arc.lock().await.take() {
                lifecycle::stop(&mut child, codex_port).await;
            }
            session_mgr.lock().await.cleanup_session(&session_id);
            anyhow::bail!("Codex session handshake failed");
        }
    };

    let (attached, mut buffered, provider_session) = {
        let mut s = state.write().await;
        s.codex_role = role_id.clone();
        let provider_session = crate::daemon::types::ProviderConnectionState {
            provider: crate::daemon::task_graph::types::Provider::Codex,
            external_session_id: thread_id.clone(),
            cwd: cwd.clone(),
            connection_mode: if is_new_session {
                crate::daemon::types::ProviderConnectionMode::New
            } else {
                crate::daemon::types::ProviderConnectionMode::Resumed
            },
        };
        let attached = s.attach_codex_task_session_for_agent(
            &task_id, &agent_id, launch_epoch, inject_tx.clone(), Some(provider_session.clone()),
        );
        let buffered = if attached {
            if is_new_session {
                crate::daemon::provider::codex::register_on_launch(
                    &mut s, &task_id, &role_id, &cwd, &thread_id, Some(&agent_id),
                );
            } else if let Some(existing_session_id) = s
                .task_graph
                .find_session_by_external_id(
                    crate::daemon::task_graph::types::Provider::Codex,
                    &thread_id,
                )
                .map(|session| session.session_id.clone())
            {
                let _ = s.resume_session(&existing_session_id);
            }
            s.take_buffered_for(&role_id)
        } else {
            Vec::new()
        };
        (attached, buffered, provider_session)
    };
    if !attached {
        cancel.cancel();
        if let Some(mut child) = child_arc.lock().await.take() {
            lifecycle::stop(&mut child, codex_port).await;
        }
        session_mgr.lock().await.cleanup_session(&session_id);
        anyhow::bail!("Codex session was superseded before it became active");
    }
    {
        let mut i = 0;
        while i < buffered.len() {
            let from_user = buffered[i].is_from_user();
            let text = crate::daemon::routing::format_codex_input(&buffered[i]);
            if inject_tx.send((vec![serde_json::json!({"type":"text","text":text})], from_user)).await.is_err() {
                let mut s = state.write().await;
                for m in buffered.drain(i..) {
                    s.buffer_message(m);
                }
                eprintln!("[Codex] inject replay failed, re-buffered tail");
                break;
            }
            i += 1;
        }
    }
    gui::emit_agent_status_online(&app, "codex", Some(provider_session), role_id.clone());
    gui::emit_system_log(
        &app,
        "info",
        &format!("[Codex] ready role={role_id} thread={thread_id}"),
    );

    spawn_health_monitor(
        child_arc.clone(),
        task_id.clone(),
        agent_id.clone(),
        launch_epoch,
        codex_port,
        exit_tx,
        state.clone(),
        app.clone(),
        cancel.clone(),
    );

    Ok(CodexHandle {
        process: child_arc,
        session_mgr,
        session_id,
        cancel,
        _api_key_home: api_key_home,
        port: codex_port,
        launch_id: launch_epoch,
        task_id,
        agent_id,
    })
}

#[cfg(test)]
#[path = "start_tests.rs"]
mod start_tests;
