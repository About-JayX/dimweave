use crate::daemon::provider::shared::ProviderHistoryEntry;
use crate::daemon::task_graph::types::{Provider, SessionRole, Task};
use crate::daemon::types::{
    self, HistoryEntry, PermissionBehavior, ProviderConnectionMode, SessionTreeSnapshot,
    TaskSnapshot,
};
use tokio::sync::{mpsc, oneshot};

pub enum DaemonCmd {
    SendUserInput {
        content: String,
        target: String,
    },
    LaunchCodex {
        role_id: String,
        cwd: String,
        model: Option<String>,
        reasoning_effort: Option<String>,
        resume_thread_id: Option<String>,
        reply: oneshot::Sender<Result<(), String>>,
    },
    StopCodex,
    Shutdown {
        reply: oneshot::Sender<()>,
    },
    ReadStatusSnapshot {
        reply: oneshot::Sender<types::DaemonStatusSnapshot>,
    },
    RegisterClaudeLaunch {
        role_id: String,
        cwd: String,
        external_id: String,
        transcript_path: String,
        connection_mode: ProviderConnectionMode,
        reply: oneshot::Sender<Result<(), String>>,
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
        reply: oneshot::Sender<Task>,
    },
    ListTasks {
        workspace: Option<String>,
        reply: oneshot::Sender<Vec<Task>>,
    },
    SelectTask {
        task_id: String,
        reply: oneshot::Sender<Result<(), String>>,
    },
    GetTaskSnapshot {
        reply: oneshot::Sender<Option<TaskSnapshot>>,
    },
    ApproveReview {
        reply: oneshot::Sender<Result<(), String>>,
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
}

pub fn channel() -> (mpsc::Sender<DaemonCmd>, mpsc::Receiver<DaemonCmd>) {
    mpsc::channel(64)
}

const AGENT_ROLES: &[&str] = &["lead", "coder", "reviewer"];

pub fn is_valid_agent_role(role: &str) -> bool {
    AGENT_ROLES.contains(&role)
}
