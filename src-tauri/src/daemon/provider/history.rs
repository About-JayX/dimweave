use crate::daemon::provider::{claude, codex, shared::ProviderHistoryEntry};
use crate::daemon::SharedState;
use tauri::AppHandle;

pub async fn list_workspace_provider_history(
    state: &SharedState,
    workspace: &str,
    app: &AppHandle,
) -> Vec<ProviderHistoryEntry> {
    let mut entries = Vec::new();

    match claude::list_sessions(workspace, None) {
        Ok(page) => entries.extend(page.entries),
        Err(err) => eprintln!("[Daemon] Claude history list failed for {workspace}: {err}"),
    }

    entries.extend(
        load_codex_history_entries(
            workspace,
            codex::is_app_server_reachable(crate::daemon::ports::PortConfig::from_env().codex).await,
            || async {
                codex::list_threads(
                    crate::daemon::ports::PortConfig::from_env().codex,
                    &codex::ListThreadsParams {
                        archived: false,
                        cwd: Some(workspace.to_string()),
                        ..Default::default()
                    },
                    app,
                )
                .await
            },
            || codex::list_local_sessions(workspace, None),
        )
        .await,
    );

    let daemon = state.read().await;
    for entry in &mut entries {
        if let Some(session) = daemon
            .task_graph
            .find_session_by_external_id(entry.provider, &entry.external_id)
        {
            entry.normalized_session_id = Some(session.session_id.clone());
            entry.normalized_task_id = Some(session.task_id.clone());
        }
    }
    drop(daemon);

    entries.sort_by(|a, b| {
        b.updated_at
            .cmp(&a.updated_at)
            .then_with(|| provider_rank(a.provider).cmp(&provider_rank(b.provider)))
            .then_with(|| a.external_id.cmp(&b.external_id))
    });
    entries
}

async fn load_codex_history_entries<Remote, RemoteFut, Local>(
    workspace: &str,
    remote_available: bool,
    remote_list: Remote,
    local_list: Local,
) -> Vec<ProviderHistoryEntry>
where
    Remote: FnOnce() -> RemoteFut,
    RemoteFut: std::future::Future<
        Output = Result<crate::daemon::provider::shared::ProviderHistoryPage, String>,
    >,
    Local: FnOnce() -> Result<crate::daemon::provider::shared::ProviderHistoryPage, String>,
{
    if remote_available {
        match remote_list().await {
            Ok(page) => return page.entries,
            Err(err) => {
                eprintln!("[Daemon] Codex history list failed for {workspace}: {err}");
            }
        }
    }

    match local_list() {
        Ok(page) => page.entries,
        Err(local_err) => {
            eprintln!("[Daemon] Codex local history fallback failed for {workspace}: {local_err}");
            Vec::new()
        }
    }
}

fn provider_rank(provider: crate::daemon::task_graph::types::Provider) -> u8 {
    match provider {
        crate::daemon::task_graph::types::Provider::Claude => 0,
        crate::daemon::task_graph::types::Provider::Codex => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::load_codex_history_entries;
    use crate::daemon::provider::shared::{ProviderHistoryEntry, ProviderHistoryPage};
    use crate::daemon::task_graph::types::{Provider, SessionStatus};
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };

    fn entry(external_id: &str) -> ProviderHistoryEntry {
        ProviderHistoryEntry {
            provider: Provider::Codex,
            external_id: external_id.to_string(),
            title: Some("thread".into()),
            preview: None,
            cwd: Some("/ws".into()),
            archived: false,
            created_at: 1,
            updated_at: 2,
            status: SessionStatus::Active,
            normalized_session_id: None,
            normalized_task_id: None,
        }
    }

    #[tokio::test]
    async fn skips_remote_codex_history_when_app_server_is_unreachable() {
        let remote_called = Arc::new(AtomicBool::new(false));
        let remote_called_clone = remote_called.clone();

        let entries = load_codex_history_entries(
            "/ws",
            false,
            move || {
                let remote_called = remote_called_clone.clone();
                async move {
                    remote_called.store(true, Ordering::SeqCst);
                    Ok(ProviderHistoryPage {
                        entries: vec![entry("remote")],
                        next_cursor: None,
                    })
                }
            },
            || {
                Ok(ProviderHistoryPage {
                    entries: vec![entry("local")],
                    next_cursor: None,
                })
            },
        )
        .await;

        assert!(!remote_called.load(Ordering::SeqCst));
        assert_eq!(entries[0].external_id, "local");
    }

    #[tokio::test]
    async fn falls_back_to_local_codex_history_when_remote_listing_fails() {
        let entries = load_codex_history_entries(
            "/ws",
            true,
            || async { Err("connect failed for thread/list".to_string()) },
            || {
                Ok(ProviderHistoryPage {
                    entries: vec![entry("local")],
                    next_cursor: None,
                })
            },
        )
        .await;

        assert_eq!(entries[0].external_id, "local");
    }
}
