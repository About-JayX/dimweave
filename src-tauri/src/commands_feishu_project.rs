use crate::daemon::DaemonCmd;
use crate::DaemonSender;
use tauri::State;

#[tauri::command]
pub async fn feishu_project_get_state(
    sender: State<'_, DaemonSender>,
) -> Result<crate::feishu_project::types::FeishuProjectRuntimeState, String> {
    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
    sender
        .0
        .send(DaemonCmd::GetFeishuProjectState { reply: reply_tx })
        .await
        .map_err(|_| "daemon offline".to_string())?;
    reply_rx.await.map_err(|_| "daemon dropped".to_string())
}

#[tauri::command]
pub async fn feishu_project_save_config(
    sender: State<'_, DaemonSender>,
    config: crate::feishu_project::types::FeishuProjectConfig,
) -> Result<crate::feishu_project::types::FeishuProjectRuntimeState, String> {
    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
    sender
        .0
        .send(DaemonCmd::SaveFeishuProjectConfig {
            config,
            reply: reply_tx,
        })
        .await
        .map_err(|_| "daemon offline".to_string())?;
    reply_rx.await.map_err(|_| "daemon dropped".to_string())?
}

#[tauri::command]
pub async fn feishu_project_sync_now(
    sender: State<'_, DaemonSender>,
) -> Result<(), String> {
    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
    sender
        .0
        .send(DaemonCmd::FeishuProjectSyncNow { reply: reply_tx })
        .await
        .map_err(|_| "daemon offline".to_string())?;
    reply_rx.await.map_err(|_| "daemon dropped".to_string())?
}

#[tauri::command]
pub async fn feishu_project_list_items(
    sender: State<'_, DaemonSender>,
) -> Result<Vec<crate::feishu_project::types::FeishuProjectInboxItem>, String> {
    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
    sender
        .0
        .send(DaemonCmd::FeishuProjectListItems { reply: reply_tx })
        .await
        .map_err(|_| "daemon offline".to_string())?;
    Ok(reply_rx.await.map_err(|_| "daemon dropped".to_string())?)
}

#[tauri::command]
pub async fn feishu_project_load_more(
    sender: State<'_, DaemonSender>,
) -> Result<usize, String> {
    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
    sender
        .0
        .send(DaemonCmd::FeishuProjectLoadMore { reply: reply_tx })
        .await
        .map_err(|_| "daemon offline".to_string())?;
    reply_rx.await.map_err(|_| "daemon dropped".to_string())?
}

#[tauri::command]
pub async fn feishu_project_load_more_filtered(
    sender: State<'_, DaemonSender>,
    filter: crate::feishu_project::types::IssueFilter,
) -> Result<usize, String> {
    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
    sender
        .0
        .send(DaemonCmd::FeishuProjectLoadMoreFiltered {
            filter,
            reply: reply_tx,
        })
        .await
        .map_err(|_| "daemon offline".to_string())?;
    reply_rx.await.map_err(|_| "daemon dropped".to_string())?
}

#[tauri::command]
pub async fn feishu_project_fetch_filter_options(
    sender: State<'_, DaemonSender>,
) -> Result<(), String> {
    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
    sender
        .0
        .send(DaemonCmd::FeishuProjectFetchFilterOptions { reply: reply_tx })
        .await
        .map_err(|_| "daemon offline".to_string())?;
    reply_rx.await.map_err(|_| "daemon dropped".to_string())?
}

#[tauri::command]
pub async fn feishu_project_start_handling(
    sender: State<'_, DaemonSender>,
    work_item_id: String,
) -> Result<String, String> {
    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
    sender
        .0
        .send(DaemonCmd::FeishuProjectStartHandling {
            work_item_id,
            reply: reply_tx,
        })
        .await
        .map_err(|_| "daemon offline".to_string())?;
    reply_rx.await.map_err(|_| "daemon dropped".to_string())?
}

#[tauri::command]
pub async fn feishu_project_set_ignored(
    sender: State<'_, DaemonSender>,
    work_item_id: String,
    ignored: bool,
) -> Result<(), String> {
    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
    sender
        .0
        .send(DaemonCmd::FeishuProjectSetIgnored {
            work_item_id,
            ignored,
            reply: reply_tx,
        })
        .await
        .map_err(|_| "daemon offline".to_string())?;
    reply_rx.await.map_err(|_| "daemon dropped".to_string())?
}
