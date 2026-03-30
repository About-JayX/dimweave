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
fn serialize_online_agents_response_to_agent() {
    let agents = serde_json::json!([
        {"agentId": "claude", "role": "lead", "modelSource": "claude"}
    ]);
    let outbound = ToAgent::OnlineAgentsResponse { online_agents: agents };
    let json = serde_json::to_value(outbound).unwrap();
    assert_eq!(json["type"], "online_agents_response");
    assert!(json["online_agents"].is_array());
    assert_eq!(json["online_agents"][0]["agentId"], "claude");
}
