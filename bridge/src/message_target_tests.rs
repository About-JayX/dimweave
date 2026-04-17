use super::*;
use serde_json::{json, Value};

#[test]
fn serialize_user_emits_empty_role_and_agent_id() {
    let json: Value = serde_json::to_value(MessageTarget::User).unwrap();
    assert_eq!(
        json,
        json!({ "kind": "user", "role": "", "agentId": "" })
    );
}

#[test]
fn serialize_role_emits_empty_agent_id() {
    let json: Value =
        serde_json::to_value(MessageTarget::Role { role: "lead".into() }).unwrap();
    assert_eq!(
        json,
        json!({ "kind": "role", "role": "lead", "agentId": "" })
    );
}

#[test]
fn serialize_agent_emits_empty_role() {
    let json: Value = serde_json::to_value(MessageTarget::Agent {
        agent_id: "agent_42".into(),
    })
    .unwrap();
    assert_eq!(
        json,
        json!({ "kind": "agent", "role": "", "agentId": "agent_42" })
    );
}

#[test]
fn deserialize_flat_user_form() {
    let t: MessageTarget =
        serde_json::from_value(json!({ "kind": "user", "role": "", "agentId": "" })).unwrap();
    assert_eq!(t, MessageTarget::User);
}

#[test]
fn deserialize_flat_role_form() {
    let t: MessageTarget = serde_json::from_value(json!({
        "kind": "role", "role": "lead", "agentId": ""
    }))
    .unwrap();
    assert_eq!(t, MessageTarget::Role { role: "lead".into() });
}

#[test]
fn deserialize_flat_agent_form() {
    let t: MessageTarget = serde_json::from_value(json!({
        "kind": "agent", "role": "", "agentId": "agent_42"
    }))
    .unwrap();
    assert_eq!(
        t,
        MessageTarget::Agent {
            agent_id: "agent_42".into(),
        }
    );
}

#[test]
fn deserialize_legacy_role_without_agent_id() {
    let t: MessageTarget =
        serde_json::from_value(json!({ "kind": "role", "role": "lead" })).unwrap();
    assert_eq!(t, MessageTarget::Role { role: "lead".into() });
}

#[test]
fn deserialize_legacy_agent_without_role() {
    let t: MessageTarget =
        serde_json::from_value(json!({ "kind": "agent", "agentId": "x" })).unwrap();
    assert_eq!(t, MessageTarget::Agent { agent_id: "x".into() });
}

#[test]
fn deserialize_rejects_role_with_empty_role_field() {
    let err = serde_json::from_value::<MessageTarget>(json!({
        "kind": "role", "role": "", "agentId": ""
    }))
    .unwrap_err();
    assert!(err.to_string().contains("non-empty"));
}

#[test]
fn deserialize_rejects_agent_with_empty_agent_id() {
    let err = serde_json::from_value::<MessageTarget>(json!({
        "kind": "agent", "role": "", "agentId": ""
    }))
    .unwrap_err();
    assert!(err.to_string().contains("non-empty"));
}

#[test]
fn deserialize_rejects_unknown_kind() {
    let err = serde_json::from_value::<MessageTarget>(json!({ "kind": "broadcast" }))
        .unwrap_err();
    assert!(err.to_string().contains("broadcast"));
}

#[test]
fn roundtrip_preserves_role() {
    let original = MessageTarget::Role { role: "coder".into() };
    let s = serde_json::to_string(&original).unwrap();
    let decoded: MessageTarget = serde_json::from_str(&s).unwrap();
    assert_eq!(decoded, original);
}
