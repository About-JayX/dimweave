use super::*;
use serde_json::{json, Value};

// ── Serialize: always emits the canonical flat 3-field form ──

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

// ── Deserialize: canonical flat form ──

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

// ── Deserialize: legacy discriminated-union form (backward compat) ──

#[test]
fn deserialize_legacy_user_without_role_or_agent_id() {
    let t: MessageTarget = serde_json::from_value(json!({ "kind": "user" })).unwrap();
    assert_eq!(t, MessageTarget::User);
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

// ── Deserialize: rejection cases ──

#[test]
fn deserialize_rejects_missing_kind() {
    let err = serde_json::from_value::<MessageTarget>(json!({ "role": "lead" })).unwrap_err();
    assert!(err.to_string().contains("kind"));
}

#[test]
fn deserialize_rejects_unknown_kind() {
    let err = serde_json::from_value::<MessageTarget>(json!({ "kind": "broadcast" }))
        .unwrap_err();
    assert!(err.to_string().contains("broadcast"));
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
fn deserialize_rejects_role_with_whitespace_only_role() {
    let err = serde_json::from_value::<MessageTarget>(json!({
        "kind": "role", "role": "   ", "agentId": ""
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

// ── Roundtrip + unknown-field tolerance ──

#[test]
fn roundtrip_preserves_role() {
    let original = MessageTarget::Role { role: "coder".into() };
    let s = serde_json::to_string(&original).unwrap();
    let decoded: MessageTarget = serde_json::from_str(&s).unwrap();
    assert_eq!(decoded, original);
}

#[test]
fn roundtrip_preserves_agent() {
    let original = MessageTarget::Agent {
        agent_id: "agent_99".into(),
    };
    let s = serde_json::to_string(&original).unwrap();
    let decoded: MessageTarget = serde_json::from_str(&s).unwrap();
    assert_eq!(decoded, original);
}

#[test]
fn deserialize_ignores_unknown_fields() {
    let t: MessageTarget = serde_json::from_value(json!({
        "kind": "user", "role": "", "agentId": "",
        "extra": "should be ignored"
    }))
    .unwrap();
    assert_eq!(t, MessageTarget::User);
}

// ── User variant ignores stray role / agentId values ──

#[test]
fn deserialize_user_ignores_populated_role_and_agent_id() {
    // Some clients might send role="something" with kind="user"; we treat it
    // as User since kind is the discriminator.
    let t: MessageTarget = serde_json::from_value(json!({
        "kind": "user", "role": "lead", "agentId": "agent_42"
    }))
    .unwrap();
    assert_eq!(t, MessageTarget::User);
}
