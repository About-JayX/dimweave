use crate::daemon::{
    task_graph::types::{Provider, Task, TaskAgent},
    types::{SessionTreeSnapshot, TaskSnapshot},
    DaemonCmd,
};
use crate::DaemonSender;
use tauri::State;

fn parse_provider(s: &str) -> Result<Provider, String> {
    match s {
        "claude" => Ok(Provider::Claude),
        "codex" => Ok(Provider::Codex),
        _ => Err(format!("unknown provider: {s}")),
    }
}

#[tauri::command]
pub async fn daemon_create_task(
    workspace: String,
    title: String,
    lead_provider: Option<String>,
    coder_provider: Option<String>,
    sender: State<'_, DaemonSender>,
) -> Result<Task, String> {
    crate::daemon::task_workspace::validate_git_root(&workspace)?;

    let lp = lead_provider.map(|s| parse_provider(&s)).transpose()?;
    let cp = coder_provider.map(|s| parse_provider(&s)).transpose()?;

    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
    sender
        .0
        .send(DaemonCmd::CreateTask {
            workspace,
            title,
            lead_provider: lp,
            coder_provider: cp,
            reply: reply_tx,
        })
        .await
        .map_err(|e| e.to_string())?;
    reply_rx
        .await
        .map_err(|_| "daemon dropped create_task reply".to_string())?
}

#[tauri::command]
pub async fn daemon_update_task_config(
    task_id: String,
    lead_provider: String,
    coder_provider: String,
    sender: State<'_, DaemonSender>,
) -> Result<Task, String> {
    let lp = parse_provider(&lead_provider)?;
    let cp = parse_provider(&coder_provider)?;
    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
    sender
        .0
        .send(DaemonCmd::UpdateTaskConfig {
            task_id,
            lead_provider: lp,
            coder_provider: cp,
            reply: reply_tx,
        })
        .await
        .map_err(|e| e.to_string())?;
    reply_rx
        .await
        .map_err(|_| "daemon dropped update_task_config reply".to_string())?
}

#[tauri::command]
pub async fn daemon_list_tasks(
    workspace: Option<String>,
    sender: State<'_, DaemonSender>,
) -> Result<Vec<Task>, String> {
    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
    sender
        .0
        .send(DaemonCmd::ListTasks {
            workspace,
            reply: reply_tx,
        })
        .await
        .map_err(|e| e.to_string())?;
    reply_rx
        .await
        .map_err(|_| "daemon dropped list_tasks reply".to_string())
}

#[tauri::command]
pub async fn daemon_select_task(
    task_id: String,
    sender: State<'_, DaemonSender>,
) -> Result<(), String> {
    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
    sender
        .0
        .send(DaemonCmd::SelectTask {
            task_id,
            reply: reply_tx,
        })
        .await
        .map_err(|e| e.to_string())?;
    reply_rx
        .await
        .map_err(|_| "daemon dropped select_task reply".to_string())?
}

#[tauri::command]
pub async fn daemon_delete_task(
    task_id: String,
    sender: State<'_, DaemonSender>,
) -> Result<(), String> {
    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
    sender
        .0
        .send(DaemonCmd::DeleteTask {
            task_id,
            reply: reply_tx,
        })
        .await
        .map_err(|e| e.to_string())?;
    reply_rx
        .await
        .map_err(|_| "daemon dropped delete_task reply".to_string())?
}

#[tauri::command]
pub async fn daemon_clear_active_task(sender: State<'_, DaemonSender>) -> Result<(), String> {
    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
    sender
        .0
        .send(DaemonCmd::ClearActiveTask { reply: reply_tx })
        .await
        .map_err(|e| e.to_string())?;
    reply_rx
        .await
        .map_err(|_| "daemon dropped clear_active_task reply".to_string())?
}

#[tauri::command]
pub async fn daemon_get_task_snapshot(
    sender: State<'_, DaemonSender>,
) -> Result<Option<TaskSnapshot>, String> {
    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
    sender
        .0
        .send(DaemonCmd::GetTaskSnapshot { reply: reply_tx })
        .await
        .map_err(|e| e.to_string())?;
    reply_rx
        .await
        .map_err(|_| "daemon dropped task_snapshot reply".to_string())
}

#[tauri::command]
pub async fn daemon_list_session_tree(
    task_id: String,
    sender: State<'_, DaemonSender>,
) -> Result<Option<SessionTreeSnapshot>, String> {
    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
    sender
        .0
        .send(DaemonCmd::ListSessionTree {
            task_id,
            reply: reply_tx,
        })
        .await
        .map_err(|e| e.to_string())?;
    reply_rx
        .await
        .map_err(|_| "daemon dropped list_session_tree reply".to_string())
}

#[tauri::command]
pub async fn daemon_add_task_agent(
    task_id: String,
    provider: String,
    role: String,
    display_name: Option<String>,
    model: Option<String>,
    effort: Option<String>,
    sender: State<'_, DaemonSender>,
) -> Result<TaskAgent, String> {
    let p = parse_provider(&provider)?;
    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
    sender
        .0
        .send(DaemonCmd::AddTaskAgent {
            task_id,
            provider: p,
            role,
            display_name,
            model,
            effort,
            reply: reply_tx,
        })
        .await
        .map_err(|e| e.to_string())?;
    reply_rx
        .await
        .map_err(|_| "daemon dropped add_task_agent reply".to_string())?
}

#[tauri::command]
pub async fn daemon_remove_task_agent(
    agent_id: String,
    sender: State<'_, DaemonSender>,
) -> Result<(), String> {
    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
    sender
        .0
        .send(DaemonCmd::RemoveTaskAgent {
            agent_id,
            reply: reply_tx,
        })
        .await
        .map_err(|e| e.to_string())?;
    reply_rx
        .await
        .map_err(|_| "daemon dropped remove_task_agent reply".to_string())?
}

#[tauri::command]
pub async fn daemon_update_task_agent(
    agent_id: String,
    provider: String,
    role: String,
    display_name: Option<String>,
    model: Option<String>,
    effort: Option<String>,
    sender: State<'_, DaemonSender>,
) -> Result<(), String> {
    let p = parse_provider(&provider)?;
    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
    sender
        .0
        .send(DaemonCmd::UpdateTaskAgent {
            agent_id,
            provider: p,
            role,
            display_name,
            model,
            effort,
            reply: reply_tx,
        })
        .await
        .map_err(|e| e.to_string())?;
    reply_rx
        .await
        .map_err(|_| "daemon dropped update_task_agent reply".to_string())?
}

#[tauri::command]
pub async fn daemon_reorder_task_agents(
    task_id: String,
    agent_ids: Vec<String>,
    sender: State<'_, DaemonSender>,
) -> Result<(), String> {
    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
    sender
        .0
        .send(DaemonCmd::ReorderTaskAgents {
            task_id,
            agent_ids,
            reply: reply_tx,
        })
        .await
        .map_err(|e| e.to_string())?;
    reply_rx
        .await
        .map_err(|_| "daemon dropped reorder_task_agents reply".to_string())?
}
