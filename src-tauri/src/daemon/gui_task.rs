use crate::daemon::task_graph::types::{Artifact, ReviewStatus, SessionHandle, Task};
use serde::Serialize;
use tauri::{AppHandle, Emitter};

/// Emitted when any task field changes (status, review_status, sessions, etc.).
pub fn emit_task_updated(app: &AppHandle, task: &Task) {
    let _ = app.emit("task_updated", task.clone());
}

/// Emitted when the daemon's active task pointer changes.
pub fn emit_active_task_changed(app: &AppHandle, task_id: Option<&str>) {
    #[derive(Serialize, Clone)]
    #[serde(rename_all = "camelCase")]
    struct Payload {
        task_id: Option<String>,
    }
    let _ = app.emit(
        "active_task_changed",
        Payload { task_id: task_id.map(String::from) },
    );
}

/// Emitted when a task's review gate status changes.
pub fn emit_review_gate_changed(app: &AppHandle, task: &Task) {
    #[derive(Serialize, Clone)]
    #[serde(rename_all = "camelCase")]
    struct Payload {
        task_id: String,
        review_status: Option<ReviewStatus>,
    }
    let _ = app.emit(
        "review_gate_changed",
        Payload {
            task_id: task.task_id.clone(),
            review_status: task.review_status,
        },
    );
}

/// Emitted when the session tree for a task changes (session added/removed/status).
pub fn emit_session_tree_changed(app: &AppHandle, task_id: &str, sessions: &[SessionHandle]) {
    #[derive(Serialize, Clone)]
    #[serde(rename_all = "camelCase")]
    struct Payload {
        task_id: String,
        sessions: Vec<SessionHandle>,
    }
    let _ = app.emit(
        "session_tree_changed",
        Payload { task_id: task_id.into(), sessions: sessions.to_vec() },
    );
}

/// Emitted when the artifact list for a task changes.
pub fn emit_artifacts_changed(app: &AppHandle, task_id: &str, artifacts: &[Artifact]) {
    #[derive(Serialize, Clone)]
    #[serde(rename_all = "camelCase")]
    struct Payload {
        task_id: String,
        artifacts: Vec<Artifact>,
    }
    let _ = app.emit(
        "artifacts_changed",
        Payload { task_id: task_id.into(), artifacts: artifacts.to_vec() },
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::daemon::task_graph::types::TaskStatus;

    #[test]
    fn active_task_changed_payload_serializes_camel_case() {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Payload { task_id: Option<String> }
        let json = serde_json::to_value(Payload { task_id: Some("t1".into()) }).unwrap();
        assert_eq!(json["taskId"], "t1");
    }

    #[test]
    fn review_gate_payload_serializes_correctly() {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Payload { task_id: String, review_status: Option<ReviewStatus> }
        let json = serde_json::to_value(Payload {
            task_id: "t1".into(),
            review_status: Some(ReviewStatus::PendingLeadApproval),
        }).unwrap();
        assert_eq!(json["taskId"], "t1");
        assert_eq!(json["reviewStatus"], "pending_lead_approval");
    }

    #[test]
    fn session_tree_payload_serializes_correctly() {
        use crate::daemon::task_graph::types::*;
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Payload { task_id: String, sessions: Vec<SessionHandle> }
        let json = serde_json::to_value(Payload {
            task_id: "t1".into(),
            sessions: vec![SessionHandle {
                session_id: "s1".into(), task_id: "t1".into(),
                parent_session_id: None, provider: Provider::Claude,
                role: SessionRole::Lead, external_session_id: None,
                status: SessionStatus::Active, cwd: "/ws".into(),
                title: "Lead".into(), created_at: 100, updated_at: 200,
            }],
        }).unwrap();
        assert_eq!(json["taskId"], "t1");
        assert_eq!(json["sessions"][0]["sessionId"], "s1");
    }

    #[test]
    fn artifacts_payload_serializes_correctly() {
        use crate::daemon::task_graph::types::*;
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Payload { task_id: String, artifacts: Vec<Artifact> }
        let json = serde_json::to_value(Payload {
            task_id: "t1".into(),
            artifacts: vec![Artifact {
                artifact_id: "a1".into(), task_id: "t1".into(),
                session_id: "s1".into(), kind: ArtifactKind::Diff,
                title: "patch".into(), content_ref: "ref".into(), created_at: 300,
            }],
        }).unwrap();
        assert_eq!(json["taskId"], "t1");
        assert_eq!(json["artifacts"][0]["artifactId"], "a1");
        assert_eq!(json["artifacts"][0]["kind"], "diff");
    }

    #[test]
    fn task_updated_payload_matches_task_shape() {
        let task = Task {
            task_id: "task_1".into(), workspace_root: "/ws".into(),
            title: "T1".into(), status: TaskStatus::Planning,
            review_status: None, lead_session_id: None,
            current_coder_session_id: None, created_at: 100, updated_at: 200,
        };
        let json = serde_json::to_value(&task).unwrap();
        assert_eq!(json["taskId"], "task_1");
        assert_eq!(json["status"], "planning");
        assert!(json["reviewStatus"].is_null());
    }
}
