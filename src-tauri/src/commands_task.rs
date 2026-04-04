use crate::daemon::{
    task_graph::types::Task,
    types::{SessionTreeSnapshot, TaskSnapshot},
    DaemonCmd,
};
use crate::DaemonSender;
use tauri::State;

#[tauri::command]
pub async fn daemon_create_task(
    workspace: String,
    title: String,
    sender: State<'_, DaemonSender>,
) -> Result<Task, String> {
    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
    sender
        .0
        .send(DaemonCmd::CreateTask {
            workspace,
            title,
            reply: reply_tx,
        })
        .await
        .map_err(|e| e.to_string())?;
    reply_rx
        .await
        .map_err(|_| "daemon dropped create_task reply".to_string())
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
pub async fn daemon_approve_review(sender: State<'_, DaemonSender>) -> Result<(), String> {
    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
    sender
        .0
        .send(DaemonCmd::ApproveReview { reply: reply_tx })
        .await
        .map_err(|e| e.to_string())?;
    reply_rx
        .await
        .map_err(|_| "daemon dropped approve_review reply".to_string())?
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
