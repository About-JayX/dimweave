use crate::daemon::provider::shared::ProviderHistoryEntry;
use crate::daemon::task_graph::types::{Provider, SessionRole, Task};
use crate::daemon::types::{
    self, HistoryEntry, PermissionBehavior, SessionTreeSnapshot, TaskSnapshot,
};
use tokio::sync::{mpsc, oneshot};

pub enum DaemonCmd {
    SendUserInput {
        content: String,
        target: String,
        attachments: Option<Vec<crate::daemon::types::Attachment>>,
    },
    LaunchCodex {
        role_id: String,
        cwd: String,
        model: Option<String>,
        reasoning_effort: Option<String>,
        resume_thread_id: Option<String>,
        reply: oneshot::Sender<Result<(), String>>,
    },
    LaunchClaudeSdk {
        role_id: String,
        cwd: String,
        model: Option<String>,
        effort: Option<String>,
        resume_session_id: Option<String>,
        reply: oneshot::Sender<Result<(), String>>,
    },
    StopClaudeSdk,
    StopCodex,
    Shutdown {
        reply: oneshot::Sender<()>,
    },
    ReadStatusSnapshot {
        reply: oneshot::Sender<types::DaemonStatusSnapshot>,
    },
    ReadClaudeRole {
        reply: oneshot::Sender<String>,
    },
    SetClaudeRole {
        role: String,
        reply: oneshot::Sender<Result<(), String>>,
    },
    SetCodexRole {
        role: String,
        reply: oneshot::Sender<Result<(), String>>,
    },
    RespondPermission {
        request_id: String,
        behavior: PermissionBehavior,
    },
    ForceDisconnectAgent {
        agent_id: String,
    },
    // ── Task management ───────────────────────────────────────
    CreateTask {
        workspace: String,
        title: String,
        reply: oneshot::Sender<Result<Task, String>>,
    },
    ListTasks {
        workspace: Option<String>,
        reply: oneshot::Sender<Vec<Task>>,
    },
    SelectTask {
        task_id: String,
        reply: oneshot::Sender<Result<(), String>>,
    },
    ClearActiveTask {
        reply: oneshot::Sender<Result<(), String>>,
    },
    GetTaskSnapshot {
        reply: oneshot::Sender<Option<TaskSnapshot>>,
    },
    ListSessionTree {
        task_id: String,
        reply: oneshot::Sender<Option<SessionTreeSnapshot>>,
    },
    ListHistory {
        workspace: Option<String>,
        reply: oneshot::Sender<Vec<HistoryEntry>>,
    },
    ListProviderHistory {
        workspace: Option<String>,
        reply: oneshot::Sender<Vec<ProviderHistoryEntry>>,
    },
    ResumeSession {
        session_id: String,
        reply: oneshot::Sender<Result<(), String>>,
    },
    AttachProviderHistory {
        provider: Provider,
        external_id: String,
        cwd: String,
        role: SessionRole,
        reply: oneshot::Sender<Result<(), String>>,
    },
    // ── Feishu Project ────────────────────────────────────────
    GetFeishuProjectState {
        reply: oneshot::Sender<crate::feishu_project::types::FeishuProjectRuntimeState>,
    },
    SaveFeishuProjectConfig {
        config: crate::feishu_project::types::FeishuProjectConfig,
        reply: oneshot::Sender<Result<crate::feishu_project::types::FeishuProjectRuntimeState, String>>,
    },
    FeishuProjectSyncNow {
        reply: oneshot::Sender<Result<(), String>>,
    },
    FeishuProjectListItems {
        reply: oneshot::Sender<Vec<crate::feishu_project::types::FeishuProjectInboxItem>>,
    },
    FeishuProjectStartHandling {
        work_item_id: String,
        reply: oneshot::Sender<Result<String, String>>,
    },
    FeishuProjectLoadMore {
        reply: oneshot::Sender<Result<usize, String>>,
    },
    FeishuProjectLoadMoreFiltered {
        filter: crate::feishu_project::types::IssueFilter,
        reply: oneshot::Sender<Result<usize, String>>,
    },
    FeishuProjectFetchFilterOptions {
        reply: oneshot::Sender<Result<(), String>>,
    },
    FeishuProjectSetIgnored {
        work_item_id: String,
        ignored: bool,
        reply: oneshot::Sender<Result<(), String>>,
    },
    // ── Telegram ─────────────────────────────────────────────
    GetTelegramState {
        reply: oneshot::Sender<crate::telegram::types::TelegramRuntimeState>,
    },
    SaveTelegramConfig {
        bot_token: String,
        enabled: bool,
        notifications_enabled: bool,
        reply: oneshot::Sender<Result<crate::telegram::types::TelegramRuntimeState, String>>,
    },
    GenerateTelegramPairCode {
        reply: oneshot::Sender<Result<crate::telegram::types::TelegramRuntimeState, String>>,
    },
    ClearTelegramPairing {
        reply: oneshot::Sender<Result<crate::telegram::types::TelegramRuntimeState, String>>,
    },
}

pub fn channel() -> (mpsc::Sender<DaemonCmd>, mpsc::Receiver<DaemonCmd>) {
    mpsc::channel(64)
}

const AGENT_ROLES: &[&str] = &["lead", "coder"];

pub fn is_valid_agent_role(role: &str) -> bool {
    AGENT_ROLES.contains(&role)
}
