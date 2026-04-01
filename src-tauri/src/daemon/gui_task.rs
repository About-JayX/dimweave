use crate::daemon::task_graph::types::{Artifact, ReviewStatus, SessionHandle, Task};
use serde::Serialize;
use tauri::{AppHandle, Emitter};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskUiEvent {
    TaskUpdated(Task),
    ActiveTaskChanged {
        task_id: Option<String>,
    },
    ReviewGateChanged {
        task_id: String,
        review_status: Option<ReviewStatus>,
    },
    SessionTreeChanged {
        task_id: String,
        sessions: Vec<SessionHandle>,
    },
    ArtifactsChanged {
        task_id: String,
        artifacts: Vec<Artifact>,
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
            TaskUiEvent::ReviewGateChanged {
                task_id,
                review_status,
            } => {
                #[derive(Serialize, Clone)]
                #[serde(rename_all = "camelCase")]
                struct Payload {
                    task_id: String,
                    review_status: Option<ReviewStatus>,
                }
                let _ = app.emit(
                    "review_gate_changed",
                    Payload {
                        task_id: task_id.clone(),
                        review_status: *review_status,
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
        }
    }
}

pub fn build_task_state_events(task: &Task) -> Vec<TaskUiEvent> {
    vec![
        TaskUiEvent::TaskUpdated(task.clone()),
        TaskUiEvent::ReviewGateChanged {
            task_id: task.task_id.clone(),
            review_status: task.review_status,
        },
    ]
}

pub fn build_task_change_events(before: Option<&Task>, after: Option<&Task>) -> Vec<TaskUiEvent> {
    if before == after {
        Vec::new()
    } else {
        after.map(build_task_state_events).unwrap_or_default()
    }
}

pub fn build_task_context_events(
    task: Option<&Task>,
    task_id: &str,
    sessions: &[SessionHandle],
    artifacts: &[Artifact],
) -> Vec<TaskUiEvent> {
    let mut events = task.map(build_task_state_events).unwrap_or_default();
    events.push(TaskUiEvent::ActiveTaskChanged {
        task_id: Some(task_id.to_string()),
    });
    events.push(TaskUiEvent::SessionTreeChanged {
        task_id: task_id.to_string(),
        sessions: sessions.to_vec(),
    });
    events.push(TaskUiEvent::ArtifactsChanged {
        task_id: task_id.to_string(),
        artifacts: artifacts.to_vec(),
    });
    events
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
    fn review_gate_payload_serializes_correctly() {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Payload {
            task_id: String,
            review_status: Option<ReviewStatus>,
        }
        let json = serde_json::to_value(Payload {
            task_id: "t1".into(),
            review_status: Some(ReviewStatus::PendingLeadApproval),
        })
        .unwrap();
        assert_eq!(json["taskId"], "t1");
        assert_eq!(json["reviewStatus"], "pending_lead_approval");
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
            workspace_root: "/ws".into(),
            title: "T1".into(),
            status: TaskStatus::Planning,
            review_status: None,
            lead_session_id: None,
            current_coder_session_id: None,
            created_at: 100,
            updated_at: 200,
        };
        let json = serde_json::to_value(&task).unwrap();
        assert_eq!(json["taskId"], "task_1");
        assert_eq!(json["status"], "planning");
        assert!(json["reviewStatus"].is_null());
    }

    #[test]
    fn build_task_change_events_emits_task_and_review_when_task_changes() {
        let before = Task {
            task_id: "task_1".into(),
            workspace_root: "/ws".into(),
            title: "T1".into(),
            status: TaskStatus::Implementing,
            review_status: None,
            lead_session_id: None,
            current_coder_session_id: None,
            created_at: 100,
            updated_at: 200,
        };
        let after = Task {
            status: TaskStatus::Reviewing,
            review_status: Some(ReviewStatus::PendingLeadReview),
            ..before.clone()
        };

        let events = build_task_change_events(Some(&before), Some(&after));
        assert_eq!(events.len(), 2);
        assert_eq!(events[0], TaskUiEvent::TaskUpdated(after.clone()));
        assert_eq!(
            events[1],
            TaskUiEvent::ReviewGateChanged {
                task_id: "task_1".into(),
                review_status: Some(ReviewStatus::PendingLeadReview),
            }
        );
    }

    #[test]
    fn build_task_context_events_includes_task_and_review_updates() {
        use crate::daemon::task_graph::types::*;

        let task = Task {
            task_id: "task_1".into(),
            workspace_root: "/ws".into(),
            title: "T1".into(),
            status: TaskStatus::Reviewing,
            review_status: Some(ReviewStatus::PendingLeadApproval),
            lead_session_id: Some("sess_1".into()),
            current_coder_session_id: None,
            created_at: 100,
            updated_at: 200,
        };
        let sessions = vec![SessionHandle {
            session_id: "sess_1".into(),
            task_id: "task_1".into(),
            parent_session_id: None,
            provider: Provider::Claude,
            role: SessionRole::Lead,
            external_session_id: None,
            status: SessionStatus::Active,
            cwd: "/ws".into(),
            title: "Lead".into(),
            created_at: 100,
            updated_at: 200,
        }];
        let artifacts = vec![Artifact {
            artifact_id: "art_1".into(),
            task_id: "task_1".into(),
            session_id: "sess_1".into(),
            kind: ArtifactKind::Plan,
            title: "plan".into(),
            content_ref: "artifact://plan".into(),
            created_at: 300,
        }];

        let events = build_task_context_events(Some(&task), &task.task_id, &sessions, &artifacts);

        assert_eq!(events.len(), 5);
        assert_eq!(events[0], TaskUiEvent::TaskUpdated(task.clone()));
        assert_eq!(
            events[1],
            TaskUiEvent::ReviewGateChanged {
                task_id: "task_1".into(),
                review_status: Some(ReviewStatus::PendingLeadApproval),
            }
        );
        assert_eq!(
            events[2],
            TaskUiEvent::ActiveTaskChanged {
                task_id: Some("task_1".into()),
            }
        );
        assert_eq!(
            events[3],
            TaskUiEvent::SessionTreeChanged {
                task_id: "task_1".into(),
                sessions,
            }
        );
        assert_eq!(
            events[4],
            TaskUiEvent::ArtifactsChanged {
                task_id: "task_1".into(),
                artifacts,
            }
        );
    }
}
