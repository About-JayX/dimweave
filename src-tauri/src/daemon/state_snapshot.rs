use super::*;
use crate::daemon::task_graph::types::{SessionRole, SessionStatus, Task};
use crate::daemon::types::{HistoryEntry, SessionTreeSnapshot, TaskSnapshot};

impl DaemonState {
    pub fn status_snapshot(&self) -> DaemonStatusSnapshot {
        let mut agents = vec![
            AgentRuntimeStatus {
                agent: "claude".into(),
                online: self.is_agent_online("claude"),
                provider_session: self.provider_connection("claude"),
            },
            AgentRuntimeStatus {
                agent: "codex".into(),
                online: self.is_agent_online("codex"),
                provider_session: self.provider_connection("codex"),
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
            provider_session: None,
        }));

        DaemonStatusSnapshot {
            agents,
            runtime_health: self.runtime_health.clone(),
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

    /// Returns online agents filtered by a specific task's provider bindings.
    /// Only includes agents that serve a role in the given task.
    pub fn task_scoped_online_agents(&self, task_id: &str) -> Vec<OnlineAgentInfo> {
        let Some(task) = self.task_graph.get_task(task_id) else {
            return self.online_agents_snapshot();
        };
        let mut result = Vec::new();
        let lead_agent = match task.lead_provider {
            crate::daemon::task_graph::types::Provider::Claude => "claude",
            crate::daemon::task_graph::types::Provider::Codex => "codex",
        };
        let coder_agent = match task.coder_provider {
            crate::daemon::task_graph::types::Provider::Claude => "claude",
            crate::daemon::task_graph::types::Provider::Codex => "codex",
        };
        let is_lead_online = self.is_task_agent_online(task_id, lead_agent);
        let is_coder_online = self.is_task_agent_online(task_id, coder_agent);
        if is_lead_online {
            result.push(OnlineAgentInfo {
                agent_id: lead_agent.into(),
                role: "lead".into(),
                model_source: lead_agent.into(),
            });
        }
        if is_coder_online {
            result.push(OnlineAgentInfo {
                agent_id: coder_agent.into(),
                role: "coder".into(),
                model_source: coder_agent.into(),
            });
        }
        result
    }

    /// Provider binding summary for a specific task (AC5).
    pub fn task_provider_summary(
        &self,
        task_id: &str,
    ) -> Option<crate::daemon::types::TaskProviderSummary> {
        let task = self.task_graph.get_task(task_id)?;
        let lead_agent = match task.lead_provider {
            crate::daemon::task_graph::types::Provider::Claude => "claude",
            crate::daemon::task_graph::types::Provider::Codex => "codex",
        };
        let coder_agent = match task.coder_provider {
            crate::daemon::task_graph::types::Provider::Claude => "claude",
            crate::daemon::task_graph::types::Provider::Codex => "codex",
        };
        let lead_online = self.is_task_agent_online(task_id, lead_agent);
        let coder_online = self.is_task_agent_online(task_id, coder_agent);
        Some(crate::daemon::types::TaskProviderSummary {
            task_id: task.task_id.clone(),
            lead_provider: format!("{:?}", task.lead_provider).to_lowercase(),
            coder_provider: format!("{:?}", task.coder_provider).to_lowercase(),
            lead_online,
            coder_online,
            lead_provider_session: if lead_online {
                self.task_provider_connection(task_id, lead_agent)
            } else {
                None
            },
            coder_provider_session: if coder_online {
                self.task_provider_connection(task_id, coder_agent)
            } else {
                None
            },
        })
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
        let provider_summary = self.task_provider_summary(task_id);
        Some(TaskSnapshot {
            task,
            sessions,
            artifacts,
            provider_summary,
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
    ///
    /// Does **not** auto-persist. Callers that need persistence must call
    /// `save_task_graph()` explicitly so the result can drive status events.
    pub fn create_and_select_task(&mut self, workspace: &str, title: &str) -> Task {
        let task = self.task_graph.create_task(workspace, title);
        self.active_task_id = Some(task.task_id.clone());
        task
    }

    /// Register a per-task runtime after worktree creation.
    pub fn init_task_runtime(&mut self, task_id: &str, workspace_root: std::path::PathBuf) {
        self.task_runtimes.insert(
            task_id.to_string(),
            crate::daemon::task_runtime::TaskRuntime::new(
                task_id.to_string(),
                workspace_root,
            ),
        );
    }

    /// Look up the runtime for a given task.
    pub fn get_task_runtime(&self, task_id: &str) -> Option<&crate::daemon::task_runtime::TaskRuntime> {
        self.task_runtimes.get(task_id)
    }

    /// Rollback a partially-created task: remove from task graph, clear
    /// active_task_id if it matches, and remove any task_runtime entry.
    pub fn rollback_task_creation(&mut self, task_id: &str) {
        self.task_graph.remove_task(task_id);
        if self.active_task_id.as_deref() == Some(task_id) {
            self.active_task_id = None;
        }
        self.task_runtimes.remove(task_id);
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

    /// Resume a normalized session locally: set its task as active, update the
    /// task's current session pointer, and mark the session as Active.
    ///
    /// Note: provider-native reconnect is orchestrated by `daemon/mod.rs`
    /// before calling this method when the session carries an external
    /// Claude/Codex identifier. This function only updates normalized task
    /// graph state and persists it.
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
