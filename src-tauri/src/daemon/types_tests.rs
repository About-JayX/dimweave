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
            project_root: "/ws".into(),
            task_worktree_root: "/ws".into(),
            title: "Test".into(),
            status: TaskStatus::Implementing,
            lead_session_id: Some("s1".into()),
            current_coder_session_id: Some("s2".into()),
            lead_provider: Provider::Claude,
            coder_provider: Provider::Codex,
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
            agent_id: None,
            status: SessionStatus::Active,
            cwd: "/ws".into(),
            title: "Lead".into(),
            created_at: 1000,
            updated_at: 2000,
        }],
        artifacts: vec![],
        task_agents: vec![TaskAgent {
            agent_id: "agent_1".into(),
            task_id: "task_1".into(),
            provider: Provider::Claude,
            role: "lead".into(),
            display_name: None,
            model: None,
            effort: None,
            order: 0,
            created_at: 1000,
            updated_at: 2000,
        }],
        provider_summary: None,
        agent_runtime_statuses: vec![],
    };
    let json = serde_json::to_value(&snap).unwrap();
    assert_eq!(json["task"]["taskId"], "task_1");
    assert_eq!(json["task"]["status"], "implementing");
    assert!(json["task"]["reviewStatus"].is_null());
    assert_eq!(json["sessions"][0]["sessionId"], "s1");
    assert_eq!(json["sessions"][0]["provider"], "claude");
    assert!(json["artifacts"].as_array().unwrap().is_empty());
    assert_eq!(json["taskAgents"][0]["agentId"], "agent_1");
    assert_eq!(json["taskAgents"][0]["role"], "lead");
}

#[test]
fn task_snapshot_roundtrip() {
    use crate::daemon::task_graph::types::*;
    let snap = TaskSnapshot {
        task: Task {
            task_id: "t1".into(),
            project_root: "/ws".into(),
            task_worktree_root: "/ws".into(),
            title: "T".into(),
            status: TaskStatus::Draft,
            lead_session_id: None,
            current_coder_session_id: None,
            lead_provider: Provider::Claude,
            coder_provider: Provider::Codex,
            created_at: 100,
            updated_at: 200,
        },
        sessions: vec![],
        artifacts: vec![],
        task_agents: vec![],
        provider_summary: None,
        agent_runtime_statuses: vec![],
    };
    let json_str = serde_json::to_string(&snap).unwrap();
    let decoded: TaskSnapshot = serde_json::from_str(&json_str).unwrap();
    assert_eq!(decoded.task.task_id, "t1");
    assert_eq!(decoded.task.status, TaskStatus::Draft);
    assert_eq!(decoded.task.lead_provider, Provider::Claude);
    assert!(decoded.task_agents.is_empty());
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
            agent_id: None,
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
            project_root: "/ws".into(),
            task_worktree_root: "/ws".into(),
            title: "T".into(),
            status: TaskStatus::Done,
            lead_session_id: None,
            current_coder_session_id: None,
            lead_provider: Provider::Claude,
            coder_provider: Provider::Codex,
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

// ── BridgeMessage serialization tests ───────────────────

#[test]
fn directed_msg_user_to_role_target() {
    let msg = BridgeMessage {
        id: "msg_1".into(),
        source: MessageSource::User,
        target: MessageTarget::Role { role: "coder".into() },
        reply_target: None,
        message: "Implement this.".into(),
        timestamp: 1770000000000,
        reply_to: None,
        priority: None,
        status: None,
        task_id: Some("task_1".into()),
        session_id: None,
        attachments: None,
    };
    let json = serde_json::to_value(&msg).unwrap();
    assert_eq!(json["source"]["kind"], "user");
    assert_eq!(json["target"]["kind"], "role");
    assert_eq!(json["target"]["role"], "coder");
    assert_eq!(json["taskId"], "task_1");
    // Optional fields that are None must not appear
    assert!(json.get("replyTarget").is_none());
    assert!(json.get("sessionId").is_none());
    assert!(json.get("priority").is_none());
}

#[test]
fn directed_msg_agent_to_agent_with_reply_target() {
    use crate::daemon::task_graph::types::Provider;
    let msg = BridgeMessage {
        id: "msg_2".into(),
        source: MessageSource::Agent {
            agent_id: "agent_lead_1".into(),
            role: "lead".into(),
            provider: Provider::Claude,
            display_source: Some("claude".into()),
        },
        target: MessageTarget::Agent { agent_id: "agent_coder_2".into() },
        reply_target: Some(MessageTarget::Agent { agent_id: "agent_lead_1".into() }),
        message: "Review and implement the daemon fix.".into(),
        timestamp: 1770000000100,
        reply_to: None,
        priority: None,
        status: Some(MessageStatus::InProgress),
        task_id: Some("task_1".into()),
        session_id: Some("session_9".into()),
        attachments: None,
    };
    let json = serde_json::to_value(&msg).unwrap();
    // Source fields
    assert_eq!(json["source"]["kind"], "agent");
    assert_eq!(json["source"]["agentId"], "agent_lead_1");
    assert_eq!(json["source"]["role"], "lead");
    assert_eq!(json["source"]["provider"], "claude");
    assert_eq!(json["source"]["displaySource"], "claude");
    // Target fields
    assert_eq!(json["target"]["kind"], "agent");
    assert_eq!(json["target"]["agentId"], "agent_coder_2");
    // Reply target
    assert_eq!(json["replyTarget"]["kind"], "agent");
    assert_eq!(json["replyTarget"]["agentId"], "agent_lead_1");
    // Retained fields
    assert_eq!(json["status"], "in_progress");
    assert_eq!(json["sessionId"], "session_9");
}

#[test]
fn directed_msg_roundtrip() {
    use crate::daemon::task_graph::types::Provider;
    let msg = BridgeMessage {
        id: "msg_3".into(),
        source: MessageSource::Agent {
            agent_id: "agent_coder_2".into(),
            role: "coder".into(),
            provider: Provider::Codex,
            display_source: None,
        },
        target: MessageTarget::User,
        reply_target: None,
        message: "Task is done.".into(),
        timestamp: 1770000000200,
        reply_to: Some("msg_2".into()),
        priority: None,
        status: Some(MessageStatus::Done),
        task_id: None,
        session_id: None,
        attachments: None,
    };
    let json_str = serde_json::to_string(&msg).unwrap();
    let decoded: BridgeMessage = serde_json::from_str(&json_str).unwrap();
    assert_eq!(decoded.id, "msg_3");
    assert_eq!(decoded.source, MessageSource::Agent {
        agent_id: "agent_coder_2".into(),
        role: "coder".into(),
        provider: Provider::Codex,
        display_source: None,
    });
    assert_eq!(decoded.target, MessageTarget::User);
    assert_eq!(decoded.reply_target, None);
    assert_eq!(decoded.status, Some(MessageStatus::Done));
    assert_eq!(decoded.reply_to, Some("msg_2".into()));
    // displaySource must not appear when None
    let json = serde_json::to_value(&decoded).unwrap();
    assert!(json["source"].get("displaySource").is_none());
}

#[test]
fn bridge_message_accepts_legacy_content_field_on_deserialize() {
    // Lock the `#[serde(alias = "content")]` behavior. Persisted buffered
    // messages in `task_graph.sqlite` written before the message/content
    // rename must still deserialize — otherwise load_buffered_messages
    // silently drops them.
    let legacy_json = serde_json::json!({
        "id": "legacy_1",
        "source": { "kind": "user" },
        "target": { "kind": "user", "role": "", "agentId": "" },
        "content": "hello from old-schema buffered message",
        "timestamp": 123
    });
    let decoded: BridgeMessage =
        serde_json::from_value(legacy_json).expect("legacy content alias must deserialize");
    assert_eq!(decoded.message, "hello from old-schema buffered message");
    // Re-serialize must emit the new canonical `message` field.
    let reserialized = serde_json::to_value(&decoded).unwrap();
    assert!(
        reserialized.get("message").is_some(),
        "canonical serialization must emit `message`"
    );
    assert!(
        reserialized.get("content").is_none(),
        "canonical serialization must NOT emit legacy `content`"
    );
}
