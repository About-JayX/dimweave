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

    match codex::list_threads(
        4500,
        &codex::ListThreadsParams {
            archived: false,
            cwd: Some(workspace.to_string()),
            ..Default::default()
        },
        app,
    )
    .await
    {
        Ok(page) => entries.extend(page.entries),
        Err(err) => {
            eprintln!("[Daemon] Codex history list failed for {workspace}: {err}");
            match codex::list_local_sessions(workspace, None) {
                Ok(page) => entries.extend(page.entries),
                Err(local_err) => {
                    eprintln!(
                        "[Daemon] Codex local history fallback failed for {workspace}: {local_err}"
                    );
                }
            }
        }
    }

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

fn provider_rank(provider: crate::daemon::task_graph::types::Provider) -> u8 {
    match provider {
        crate::daemon::task_graph::types::Provider::Claude => 0,
        crate::daemon::task_graph::types::Provider::Codex => 1,
    }
}
