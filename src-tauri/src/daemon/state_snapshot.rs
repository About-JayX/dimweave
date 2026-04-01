use super::*;
use crate::daemon::task_graph::types::{SessionRole, SessionStatus, Task};
use crate::daemon::types::{HistoryEntry, SessionTreeSnapshot, TaskSnapshot};

impl DaemonState {
    pub fn status_snapshot(&self) -> DaemonStatusSnapshot {
        let mut agents = vec![
            AgentRuntimeStatus {
                agent: "claude".into(),
                online: self.is_agent_online("claude"),
            },
            AgentRuntimeStatus {
                agent: "codex".into(),
                online: self.is_agent_online("codex"),
            },
        ];

        let mut other_agents: Vec<_> = self
            .attached_agents
            .keys()
            .filter(|agent| agent.as_str() != "claude" && agent.as_str() != "codex")
            .cloned()
            .collect();
        other_agents.sort();
        agents.extend(other_agents.into_iter().map(|agent| AgentRuntimeStatus {
            agent,
            online: true,
        }));

        DaemonStatusSnapshot {
            agents,
            claude_role: self.claude_role.clone(),
            codex_role: self.codex_role.clone(),
        }
    }

    /// Returns a stable-ordered snapshot of currently online agents.
    /// Order: claude first, codex second, then any other bridge agents by agent_id.
    pub fn online_agents_snapshot(&self) -> Vec<OnlineAgentInfo> {
        let mut result = Vec::new();
        if self.is_agent_online("claude") {
            result.push(OnlineAgentInfo {
                agent_id: "claude".into(),
                role: self.claude_role.clone(),
                model_source: "claude".into(),
            });
        }
        if self.is_agent_online("codex") {
            result.push(OnlineAgentInfo {
                agent_id: "codex".into(),
                role: self.codex_role.clone(),
                model_source: "codex".into(),
            });
        }
        let mut others: Vec<_> = self
            .attached_agents
            .keys()
            .filter(|k| k.as_str() != "claude" && k.as_str() != "codex")
            .cloned()
            .collect();
        others.sort();
        for agent_id in others {
            result.push(OnlineAgentInfo {
                agent_id: agent_id.clone(),
                role: "unknown".into(),
                model_source: "claude".into(),
            });
        }
        result
    }

    /// Snapshot of the active task with its sessions and artifacts.
    pub fn task_snapshot(&self) -> Option<TaskSnapshot> {
        let task_id = self.active_task_id.as_ref()?;
        let task = self.task_graph.get_task(task_id)?.clone();
        let sessions: Vec<_> = self
            .task_graph
            .sessions_for_task(task_id)
            .into_iter()
            .cloned()
            .collect();
        let artifacts: Vec<_> = self
            .task_graph
            .artifacts_for_task(task_id)
            .into_iter()
            .cloned()
            .collect();
        Some(TaskSnapshot {
            task,
            sessions,
            artifacts,
        })
    }

    /// List tasks, optionally filtered by workspace.
    pub fn task_list(&self, workspace: Option<&str>) -> Vec<Task> {
        match workspace {
            Some(ws) => self
                .task_graph
                .tasks_for_workspace(ws)
                .into_iter()
                .cloned()
                .collect(),
            None => self.task_graph.list_tasks().into_iter().cloned().collect(),
        }
    }

    /// Create a new task and set it as active.
    pub fn create_and_select_task(&mut self, workspace: &str, title: &str) -> Task {
        let task = self.task_graph.create_task(workspace, title);
        self.active_task_id = Some(task.task_id.clone());
        self.auto_save_task_graph();
        task
    }

    /// Select an existing task as active. Returns error if not found.
    pub fn select_task(&mut self, task_id: &str) -> Result<Task, String> {
        if let Some(task) = self.task_graph.get_task(task_id) {
            let task = task.clone();
            self.active_task_id = Some(task_id.to_string());
            Ok(task)
        } else {
            Err(format!("task not found: {task_id}"))
        }
    }

    /// Session tree for a task (flat list; frontend reconstructs tree via parent_session_id).
    pub fn session_tree(&self, task_id: &str) -> Option<SessionTreeSnapshot> {
        self.task_graph.get_task(task_id)?;
        let sessions: Vec<_> = self
            .task_graph
            .sessions_for_task(task_id)
            .into_iter()
            .cloned()
            .collect();
        Some(SessionTreeSnapshot {
            task_id: task_id.to_string(),
            sessions,
        })
    }

    /// Task + session history, optionally filtered by workspace.
    pub fn task_history(&self, workspace: Option<&str>) -> Vec<HistoryEntry> {
        let tasks = match workspace {
            Some(ws) => self.task_graph.tasks_for_workspace(ws),
            None => self.task_graph.list_tasks(),
        };
        tasks
            .into_iter()
            .map(|t| {
                let session_count = self.task_graph.sessions_for_task(&t.task_id).len();
                let artifact_count = self.task_graph.artifacts_for_task(&t.task_id).len();
                HistoryEntry {
                    task: t.clone(),
                    session_count,
                    artifact_count,
                }
            })
            .collect()
    }

    /// Resume a session: set its task as active, update the task's session pointer,
    /// and mark the session as Active. Minimal skeleton — provider reconnection
    /// is not yet implemented.
    /// Returns the task_id on success for event emission.
    pub fn resume_session(&mut self, session_id: &str) -> Result<String, String> {
        let sess = self
            .task_graph
            .get_session(session_id)
            .ok_or_else(|| format!("session not found: {session_id}"))?
            .clone();
        if self.task_graph.get_task(&sess.task_id).is_none() {
            return Err(format!("task not found: {}", sess.task_id));
        }
        self.active_task_id = Some(sess.task_id.clone());
        match sess.role {
            SessionRole::Lead => {
                self.task_graph.set_lead_session(&sess.task_id, session_id);
            }
            SessionRole::Coder => {
                self.task_graph.set_coder_session(&sess.task_id, session_id);
            }
        }
        self.task_graph
            .update_session_status(session_id, SessionStatus::Active);
        self.auto_save_task_graph();
        Ok(sess.task_id)
    }
}
