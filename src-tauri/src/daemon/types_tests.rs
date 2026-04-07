use super::*;

#[test]
fn serialize_permission_verdict_to_agent() {
    let outbound = ToAgent::PermissionVerdict {
        verdict: PermissionVerdict {
            request_id: "req-1".into(),
            behavior: PermissionBehavior::Allow,
        },
    };
    let json = serde_json::to_value(outbound).unwrap();
    assert_eq!(json["type"], "permission_verdict");
    assert_eq!(json["verdict"]["request_id"], "req-1");
    assert_eq!(json["verdict"]["behavior"], "allow");
}

#[test]
fn deserialize_permission_request_from_agent() {
    let raw = r#"{"type":"permission_request","request":{"request_id":"req-2","tool_name":"Bash","description":"run pwd","input_preview":"pwd"}}"#;
    let inbound: FromAgent = serde_json::from_str(raw).unwrap();
    match inbound {
        FromAgent::PermissionRequest { request } => {
            assert_eq!(request.request_id, "req-2");
            assert_eq!(request.tool_name, "Bash");
        }
        other => panic!("unexpected inbound payload: {other:?}"),
    }
}

#[test]
fn online_agent_info_camel_case_fields() {
    let info = OnlineAgentInfo {
        agent_id: "bridge-1".into(),
        role: "coder".into(),
        model_source: "claude".into(),
    };
    let json = serde_json::to_value(&info).unwrap();
    assert_eq!(json["agentId"], "bridge-1");
    assert_eq!(json["role"], "coder");
    assert_eq!(json["modelSource"], "claude");
}

#[test]
fn online_agent_info_roundtrip() {
    let raw = r#"{"agentId":"bridge-1","role":"coder","modelSource":"claude"}"#;
    let info: OnlineAgentInfo = serde_json::from_str(raw).unwrap();
    assert_eq!(info.agent_id, "bridge-1");
    assert_eq!(info.role, "coder");
    assert_eq!(info.model_source, "claude");
}

#[test]
fn deserialize_get_online_agents_from_agent() {
    let raw = r#"{"type":"get_online_agents"}"#;
    let inbound: FromAgent = serde_json::from_str(raw).unwrap();
    assert!(matches!(inbound, FromAgent::GetOnlineAgents));
}

#[test]
fn task_snapshot_serializes_camel_case() {
    use crate::daemon::task_graph::types::*;
    let snap = TaskSnapshot {
        task: Task {
            task_id: "task_1".into(),
            workspace_root: "/ws".into(),
            title: "Test".into(),
            status: TaskStatus::Implementing,
            lead_session_id: Some("s1".into()),
            current_coder_session_id: Some("s2".into()),
            created_at: 1000,
            updated_at: 2000,
        },
        sessions: vec![SessionHandle {
            session_id: "s1".into(),
            task_id: "task_1".into(),
            parent_session_id: None,
            provider: Provider::Claude,
            role: SessionRole::Lead,
            external_session_id: None,
            transcript_path: None,
            status: SessionStatus::Active,
            cwd: "/ws".into(),
            title: "Lead".into(),
            created_at: 1000,
            updated_at: 2000,
        }],
        artifacts: vec![],
    };
    let json = serde_json::to_value(&snap).unwrap();
    assert_eq!(json["task"]["taskId"], "task_1");
    assert_eq!(json["task"]["status"], "implementing");
    assert!(json["task"]["reviewStatus"].is_null());
    assert_eq!(json["sessions"][0]["sessionId"], "s1");
    assert_eq!(json["sessions"][0]["provider"], "claude");
    assert!(json["artifacts"].as_array().unwrap().is_empty());
}

#[test]
fn task_snapshot_roundtrip() {
    use crate::daemon::task_graph::types::*;
    let snap = TaskSnapshot {
        task: Task {
            task_id: "t1".into(),
            workspace_root: "/ws".into(),
            title: "T".into(),
            status: TaskStatus::Draft,
            lead_session_id: None,
            current_coder_session_id: None,
            created_at: 100,
            updated_at: 200,
        },
        sessions: vec![],
        artifacts: vec![],
    };
    let json_str = serde_json::to_string(&snap).unwrap();
    let decoded: TaskSnapshot = serde_json::from_str(&json_str).unwrap();
    assert_eq!(decoded.task.task_id, "t1");
    assert_eq!(decoded.task.status, TaskStatus::Draft);
}

#[test]
fn session_tree_snapshot_serializes_camel_case() {
    use crate::daemon::task_graph::types::*;
    let snap = SessionTreeSnapshot {
        task_id: "t1".into(),
        sessions: vec![SessionHandle {
            session_id: "s1".into(),
            task_id: "t1".into(),
            parent_session_id: None,
            provider: Provider::Claude,
            role: SessionRole::Lead,
            external_session_id: None,
            transcript_path: None,
            status: SessionStatus::Active,
            cwd: "/ws".into(),
            title: "Lead".into(),
            created_at: 100,
            updated_at: 200,
        }],
    };
    let json = serde_json::to_value(&snap).unwrap();
    assert_eq!(json["taskId"], "t1");
    assert_eq!(json["sessions"][0]["sessionId"], "s1");
    assert_eq!(json["sessions"][0]["role"], "lead");
}

#[test]
fn history_entry_serializes_camel_case() {
    use crate::daemon::task_graph::types::*;
    let entry = HistoryEntry {
        task: Task {
            task_id: "t1".into(),
            workspace_root: "/ws".into(),
            title: "T".into(),
            status: TaskStatus::Done,
            lead_session_id: None,
            current_coder_session_id: None,
            created_at: 100,
            updated_at: 200,
        },
        session_count: 3,
        artifact_count: 1,
    };
    let json = serde_json::to_value(&entry).unwrap();
    assert_eq!(json["task"]["taskId"], "t1");
    assert_eq!(json["sessionCount"], 3);
    assert_eq!(json["artifactCount"], 1);
}

#[test]
fn serialize_online_agents_response_to_agent() {
    let agents = serde_json::json!([
        {"agentId": "claude", "role": "lead", "modelSource": "claude"}
    ]);
    let outbound = ToAgent::OnlineAgentsResponse {
        online_agents: agents,
    };
    let json = serde_json::to_value(outbound).unwrap();
    assert_eq!(json["type"], "online_agents_response");
    assert!(json["online_agents"].is_array());
    assert_eq!(json["online_agents"][0]["agentId"], "claude");
}
