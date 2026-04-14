use crate::daemon::task_graph::types::{Artifact, Provider, SessionHandle, Task, TaskAgent};
use serde::Serialize;
use tauri::{AppHandle, Emitter};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskUiEvent {
    TaskUpdated(Task),
    ActiveTaskChanged {
        task_id: Option<String>,
    },
    SessionTreeChanged {
        task_id: String,
        sessions: Vec<SessionHandle>,
    },
    ArtifactsChanged {
        task_id: String,
        artifacts: Vec<Artifact>,
    },
    TaskAgentsChanged {
        task_id: String,
        agents: Vec<TaskAgent>,
    },
}

impl TaskUiEvent {
    pub fn emit(&self, app: &AppHandle) {
        match self {
            TaskUiEvent::TaskUpdated(task) => {
                let _ = app.emit("task_updated", task.clone());
            }
            TaskUiEvent::ActiveTaskChanged { task_id } => {
                #[derive(Serialize, Clone)]
                #[serde(rename_all = "camelCase")]
                struct Payload {
                    task_id: Option<String>,
                }
                let _ = app.emit(
                    "active_task_changed",
                    Payload {
                        task_id: task_id.clone(),
                    },
                );
            }
            TaskUiEvent::SessionTreeChanged { task_id, sessions } => {
                #[derive(Serialize, Clone)]
                #[serde(rename_all = "camelCase")]
                struct Payload {
                    task_id: String,
                    sessions: Vec<SessionHandle>,
                }
                let _ = app.emit(
                    "session_tree_changed",
                    Payload {
                        task_id: task_id.clone(),
                        sessions: sessions.clone(),
                    },
                );
            }
            TaskUiEvent::ArtifactsChanged { task_id, artifacts } => {
                #[derive(Serialize, Clone)]
                #[serde(rename_all = "camelCase")]
                struct Payload {
                    task_id: String,
                    artifacts: Vec<Artifact>,
                }
                let _ = app.emit(
                    "artifacts_changed",
                    Payload {
                        task_id: task_id.clone(),
                        artifacts: artifacts.clone(),
                    },
                );
            }
            TaskUiEvent::TaskAgentsChanged { task_id, agents } => {
                #[derive(Serialize, Clone)]
                #[serde(rename_all = "camelCase")]
                struct Payload {
                    task_id: String,
                    agents: Vec<TaskAgent>,
                }
                let _ = app.emit(
                    "task_agents_changed",
                    Payload {
                        task_id: task_id.clone(),
                        agents: agents.clone(),
                    },
                );
            }
        }
    }
}

pub fn build_task_state_events(task: &Task) -> Vec<TaskUiEvent> {
    vec![TaskUiEvent::TaskUpdated(task.clone())]
}

pub fn build_task_change_events(before: Option<&Task>, after: Option<&Task>) -> Vec<TaskUiEvent> {
    if before == after {
        Vec::new()
    } else {
        after.map(build_task_state_events).unwrap_or_default()
    }
}

/// Build UI events for a task-context sync.
///
/// `active_task_id` is the backend's current active task. `ActiveTaskChanged`
/// is only emitted when the task being refreshed matches the active task,
/// so non-active-task refreshes (e.g. during disconnect churn) cannot bounce
/// the frontend selection back to a stale task.
pub fn build_task_context_events(
    task: Option<&Task>,
    task_id: &str,
    sessions: &[SessionHandle],
    artifacts: &[Artifact],
    agents: &[TaskAgent],
    active_task_id: Option<&str>,
) -> Vec<TaskUiEvent> {
    let mut events = task.map(build_task_state_events).unwrap_or_default();
    if active_task_id.map_or(false, |active| active == task_id) {
        events.push(TaskUiEvent::ActiveTaskChanged {
            task_id: Some(task_id.to_string()),
        });
    }
    events.push(TaskUiEvent::SessionTreeChanged {
        task_id: task_id.to_string(),
        sessions: sessions.to_vec(),
    });
    events.push(TaskUiEvent::ArtifactsChanged {
        task_id: task_id.to_string(),
        artifacts: artifacts.to_vec(),
    });
    events.push(TaskUiEvent::TaskAgentsChanged {
        task_id: task_id.to_string(),
        agents: agents.to_vec(),
    });
    events
}

/// Emit a full task-context sync to the frontend for the given task.
///
/// This is the single entry point for notifying the UI about session/artifact
/// changes. Accessible from submodules (codex/, claude_sdk/, control/).
pub async fn emit_task_context_events(
    state: &crate::daemon::SharedState,
    app: &AppHandle,
    task_id: &str,
) {
    let s = state.read().await;
    let sess: Vec<_> = s
        .task_graph
        .sessions_for_task(task_id)
        .into_iter()
        .cloned()
        .collect();
    let arts: Vec<_> = s
        .task_graph
        .artifacts_for_task(task_id)
        .into_iter()
        .cloned()
        .collect();
    let agents: Vec<_> = s
        .task_graph
        .agents_for_task(task_id)
        .into_iter()
        .cloned()
        .collect();
    let active_task_id = s.active_task_id.as_deref();
    let events =
        build_task_context_events(s.task_graph.get_task(task_id), task_id, &sess, &arts, &agents, active_task_id);
    drop(s);
    for event in events {
        event.emit(app);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::daemon::task_graph::types::TaskStatus;

    #[test]
    fn active_task_changed_payload_serializes_camel_case() {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Payload {
            task_id: Option<String>,
        }
        let json = serde_json::to_value(Payload {
            task_id: Some("t1".into()),
        })
        .unwrap();
        assert_eq!(json["taskId"], "t1");
    }

    #[test]
    fn session_tree_payload_serializes_correctly() {
        use crate::daemon::task_graph::types::*;
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Payload {
            task_id: String,
            sessions: Vec<SessionHandle>,
        }
        let json = serde_json::to_value(Payload {
            task_id: "t1".into(),
            sessions: vec![SessionHandle {
                session_id: "s1".into(),
                task_id: "t1".into(),
                parent_session_id: None,
                provider: Provider::Claude,
                role: SessionRole::Lead,
                external_session_id: None,
                transcript_path: None,
                agent_id: None,
                status: SessionStatus::Active,
                cwd: "/ws".into(),
                title: "Lead".into(),
                created_at: 100,
                updated_at: 200,
            }],
        })
        .unwrap();
        assert_eq!(json["taskId"], "t1");
        assert_eq!(json["sessions"][0]["sessionId"], "s1");
    }

    #[test]
    fn artifacts_payload_serializes_correctly() {
        use crate::daemon::task_graph::types::*;
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Payload {
            task_id: String,
            artifacts: Vec<Artifact>,
        }
        let json = serde_json::to_value(Payload {
            task_id: "t1".into(),
            artifacts: vec![Artifact {
                artifact_id: "a1".into(),
                task_id: "t1".into(),
                session_id: "s1".into(),
                kind: ArtifactKind::Diff,
                title: "patch".into(),
                content_ref: "ref".into(),
                created_at: 300,
            }],
        })
        .unwrap();
        assert_eq!(json["taskId"], "t1");
        assert_eq!(json["artifacts"][0]["artifactId"], "a1");
        assert_eq!(json["artifacts"][0]["kind"], "diff");
    }

    #[test]
    fn task_updated_payload_matches_task_shape() {
        let task = Task {
            task_id: "task_1".into(),
            project_root: "/ws".into(),
            workspace_root: "/ws".into(),
            title: "T1".into(),
            status: TaskStatus::Planning,
            lead_session_id: None,
            current_coder_session_id: None,
            lead_provider: Provider::Claude,
            coder_provider: Provider::Codex,
            created_at: 100,
            updated_at: 200,
        };
        let json = serde_json::to_value(&task).unwrap();
        assert_eq!(json["taskId"], "task_1");
        assert_eq!(json["status"], "planning");
    }

    #[test]
    fn build_task_change_events_emits_task_and_review_when_task_changes() {
        let before = Task {
            task_id: "task_1".into(),
            project_root: "/ws".into(),
            workspace_root: "/ws".into(),
            title: "T1".into(),
            status: TaskStatus::Implementing,
            lead_session_id: None,
            current_coder_session_id: None,
            lead_provider: Provider::Claude,
            coder_provider: Provider::Codex,
            created_at: 100,
            updated_at: 200,
        };
        let after = Task {
            status: TaskStatus::Reviewing,
            ..before.clone()
        };

        let events = build_task_change_events(Some(&before), Some(&after));
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], TaskUiEvent::TaskUpdated(after.clone()));
    }

    fn make_test_task(task_id: &str) -> Task {
        use crate::daemon::task_graph::types::TaskStatus;
        Task {
            task_id: task_id.into(),
            project_root: "/ws".into(),
            workspace_root: "/ws".into(),
            title: "T".into(),
            status: TaskStatus::Reviewing,
            lead_session_id: Some("sess_1".into()),
            current_coder_session_id: None,
            lead_provider: Provider::Claude,
            coder_provider: Provider::Codex,
            created_at: 100,
            updated_at: 200,
        }
    }

    fn make_test_session(task_id: &str) -> SessionHandle {
        use crate::daemon::task_graph::types::*;
        SessionHandle {
            session_id: "sess_1".into(),
            task_id: task_id.into(),
            parent_session_id: None,
            provider: Provider::Claude,
            role: SessionRole::Lead,
            external_session_id: None,
            transcript_path: None,
            agent_id: None,
            status: SessionStatus::Active,
            cwd: "/ws".into(),
            title: "Lead".into(),
            created_at: 100,
            updated_at: 200,
        }
    }

    fn make_test_artifact(task_id: &str) -> Artifact {
        use crate::daemon::task_graph::types::ArtifactKind;
        Artifact {
            artifact_id: "art_1".into(),
            task_id: task_id.into(),
            session_id: "sess_1".into(),
            kind: ArtifactKind::Plan,
            title: "plan".into(),
            content_ref: "artifact://plan".into(),
            created_at: 300,
        }
    }

    #[test]
    fn build_task_context_events_includes_task_and_review_updates() {
        let task = make_test_task("task_1");
        let sessions = vec![make_test_session("task_1")];
        let artifacts = vec![make_test_artifact("task_1")];

        let events =
            build_task_context_events(Some(&task), &task.task_id, &sessions, &artifacts, &[], Some("task_1"));

        assert_eq!(events.len(), 5);
        assert_eq!(events[0], TaskUiEvent::TaskUpdated(task.clone()));
        assert_eq!(
            events[1],
            TaskUiEvent::ActiveTaskChanged {
                task_id: Some("task_1".into()),
            }
        );
        assert_eq!(
            events[2],
            TaskUiEvent::SessionTreeChanged {
                task_id: "task_1".into(),
                sessions,
            }
        );
        assert_eq!(
            events[3],
            TaskUiEvent::ArtifactsChanged {
                task_id: "task_1".into(),
                artifacts,
            }
        );
    }

    #[test]
    fn build_task_context_events_non_active_does_not_emit_active_task_changed() {
        let task_a = make_test_task("task_a");
        let sessions = vec![make_test_session("task_a")];
        let artifacts = vec![make_test_artifact("task_a")];

        // task_a is being refreshed, but the active task is task_b
        let events =
            build_task_context_events(Some(&task_a), &task_a.task_id, &sessions, &artifacts, &[], Some("task_b"));

        assert!(
            !events.iter().any(|event| matches!(
                event,
                TaskUiEvent::ActiveTaskChanged { task_id }
                    if task_id.as_deref() == Some("task_a")
            )),
            "non-active task refresh must not emit ActiveTaskChanged for task_a"
        );
        // Should still emit the other context events
        assert!(events.iter().any(|e| matches!(e, TaskUiEvent::TaskUpdated(_))));
        assert!(events
            .iter()
            .any(|e| matches!(e, TaskUiEvent::SessionTreeChanged { .. })));
        assert!(events
            .iter()
            .any(|e| matches!(e, TaskUiEvent::ArtifactsChanged { .. })));
    }

    #[test]
    fn build_task_context_events_active_task_emits_active_task_changed() {
        let task = make_test_task("task_1");
        let sessions = vec![make_test_session("task_1")];
        let artifacts = vec![make_test_artifact("task_1")];

        let events =
            build_task_context_events(Some(&task), "task_1", &sessions, &artifacts, &[], Some("task_1"));

        assert!(
            events.iter().any(|e| matches!(
                e,
                TaskUiEvent::ActiveTaskChanged { task_id }
                    if task_id.as_deref() == Some("task_1")
            )),
            "active task refresh must emit ActiveTaskChanged"
        );
    }
}
