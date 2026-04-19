use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Draft,
    Planning,
    Implementing,
    Reviewing,
    Done,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    Active,
    Paused,
    Completed,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Provider {
    Claude,
    Codex,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionRole {
    Lead,
    Coder,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactKind {
    Research,
    Plan,
    Review,
    Diff,
    Verification,
    Summary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub task_id: String,
    /// The immutable project root directory (e.g. the git repo root).
    /// Tasks in the same project share this value so they appear in the
    /// same workspace list even if each task has its own worktree.
    pub project_root: String,
    /// The runtime working directory for this specific task. May differ
    /// from `project_root` when the task uses a dedicated git worktree.
    /// Defaults to `project_root` when no worktree has been assigned.
    #[serde(default)]
    pub task_worktree_root: String,
    pub title: String,
    pub status: TaskStatus,
    pub lead_session_id: Option<String>,
    pub current_coder_session_id: Option<String>,
    #[serde(default = "default_lead_provider")]
    pub lead_provider: Provider,
    #[serde(default = "default_coder_provider")]
    pub coder_provider: Provider,
    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionHandle {
    pub session_id: String,
    pub task_id: String,
    pub parent_session_id: Option<String>,
    pub provider: Provider,
    pub role: SessionRole,
    pub external_session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transcript_path: Option<String>,
    /// The TaskAgent that owns this session, if known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_id: Option<String>,
    pub status: SessionStatus,
    pub cwd: String,
    pub title: String,
    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Artifact {
    pub artifact_id: String,
    pub task_id: String,
    pub session_id: String,
    pub kind: ArtifactKind,
    pub title: String,
    pub content_ref: String,
    pub created_at: u64,
}

/// Per-task agent identity. Replaces singleton `lead_provider`/`coder_provider`
/// as the primary truth for which agents belong to a task.
/// Role is an extensible string (not limited to lead/coder).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskAgent {
    pub agent_id: String,
    pub task_id: String,
    pub provider: Provider,
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effort: Option<String>,
    pub order: u32,
    pub created_at: u64,
    pub updated_at: u64,
}

/// Parameters for creating a new session.
pub struct CreateSessionParams<'a> {
    pub task_id: &'a str,
    pub parent_session_id: Option<&'a str>,
    pub provider: Provider,
    pub role: SessionRole,
    pub cwd: &'a str,
    pub title: &'a str,
    pub agent_id: Option<&'a str>,
}

fn default_lead_provider() -> Provider {
    Provider::Claude
}

fn default_coder_provider() -> Provider {
    Provider::Codex
}

/// User-supplied provider override used by the Authentication dialog.
/// Stored in the global `provider_auth` table, one row per
/// `provider ∈ {"claude", "codex"}`.
///
/// Absent rows mean "default behavior" (subscription via CLI/keychain).
/// Setting `api_key` alone routes through the official endpoint with the
/// key injected as an env var; `api_key` + `base_url` routes through a
/// third-party endpoint via `--config model_providers.*` (Codex) or
/// `ANTHROPIC_BASE_URL` (Claude).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderAuthConfig {
    pub provider: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wire_api: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth_mode: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_name: Option<String>,
    pub updated_at: u64,
}

/// Parameters for creating a new artifact.
pub struct CreateArtifactParams<'a> {
    pub task_id: &'a str,
    pub session_id: &'a str,
    pub kind: ArtifactKind,
    pub title: &'a str,
    pub content_ref: &'a str,
}
