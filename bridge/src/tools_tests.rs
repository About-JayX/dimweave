use super::*;
use crate::types::MessageTarget;

fn call_params(args: serde_json::Value) -> serde_json::Value {
    serde_json::json!({ "name": "reply", "arguments": args })
}

// ── Structured target: valid cases ──────────────────────────────

#[test]
fn reply_with_user_target() {
    let params = call_params(serde_json::json!({
        "target": { "kind": "user" },
        "text": "hello",
        "status": "done"
    }));
    let result = handle_tool_call(&params).unwrap().unwrap();
    assert_eq!(result.target, MessageTarget::User);
    assert_eq!(result.content, "hello");
    assert_eq!(result.status, MessageStatus::Done);
}

#[test]
fn reply_with_role_target() {
    let params = call_params(serde_json::json!({
        "target": { "kind": "role", "role": "coder" },
        "text": "implement this",
        "status": "in_progress"
    }));
    let result = handle_tool_call(&params).unwrap().unwrap();
    assert_eq!(result.target, MessageTarget::Role { role: "coder".into() });
    assert_eq!(result.status, MessageStatus::InProgress);
}

#[test]
fn reply_with_agent_target() {
    let params = call_params(serde_json::json!({
        "target": { "kind": "agent", "agentId": "agent_coder_2" },
        "text": "fix the bug",
        "status": "done"
    }));
    let result = handle_tool_call(&params).unwrap().unwrap();
    assert_eq!(
        result.target,
        MessageTarget::Agent { agent_id: "agent_coder_2".into() }
    );
}

#[test]
fn reply_preserves_arbitrary_role() {
    let params = call_params(serde_json::json!({
        "target": { "kind": "role", "role": "reviewer" },
        "text": "check this",
        "status": "done"
    }));
    let result = handle_tool_call(&params).unwrap().unwrap();
    assert_eq!(result.target, MessageTarget::Role { role: "reviewer".into() });
}

#[test]
fn reply_defaults_missing_status_to_done() {
    let params = call_params(serde_json::json!({
        "target": { "kind": "user" },
        "text": "hello"
    }));
    let result = handle_tool_call(&params).unwrap().unwrap();
    assert_eq!(result.status, MessageStatus::Done);
}

// ── Structured target: rejection cases ──────────────────────────

#[test]
fn reply_rejects_missing_target() {
    let params = call_params(serde_json::json!({
        "text": "hello", "status": "done"
    }));
    assert!(matches!(
        handle_tool_call(&params).unwrap_err(),
        ToolCallError::InvalidTarget(_)
    ));
}

#[test]
fn reply_rejects_unknown_kind() {
    let params = call_params(serde_json::json!({
        "target": { "kind": "broadcast" },
        "text": "hello", "status": "done"
    }));
    match handle_tool_call(&params).unwrap_err() {
        ToolCallError::InvalidTarget(msg) => assert!(msg.contains("broadcast")),
        other => panic!("expected InvalidTarget, got {other:?}"),
    }
}

#[test]
fn reply_rejects_role_without_role_field() {
    let params = call_params(serde_json::json!({
        "target": { "kind": "role" },
        "text": "hello", "status": "done"
    }));
    assert!(matches!(
        handle_tool_call(&params).unwrap_err(),
        ToolCallError::InvalidTarget(_)
    ));
}

#[test]
fn reply_rejects_agent_without_agent_id() {
    let params = call_params(serde_json::json!({
        "target": { "kind": "agent" },
        "text": "hello", "status": "done"
    }));
    assert!(matches!(
        handle_tool_call(&params).unwrap_err(),
        ToolCallError::InvalidTarget(_)
    ));
}

#[test]
fn reply_rejects_empty_role() {
    let params = call_params(serde_json::json!({
        "target": { "kind": "role", "role": "  " },
        "text": "hello", "status": "done"
    }));
    assert!(matches!(
        handle_tool_call(&params).unwrap_err(),
        ToolCallError::InvalidTarget(_)
    ));
}

#[test]
fn reply_rejects_old_to_field_without_target() {
    // Old `to` field without structured `target` must be rejected
    let params = call_params(serde_json::json!({
        "to": "lead", "text": "hello", "status": "done"
    }));
    assert!(matches!(
        handle_tool_call(&params).unwrap_err(),
        ToolCallError::InvalidTarget(_)
    ));
}

#[test]
fn reply_invalid_status_returns_error() {
    let params = call_params(serde_json::json!({
        "target": { "kind": "user" },
        "text": "hello",
        "status": "waiting"
    }));
    let err = handle_tool_call(&params).unwrap_err();
    assert!(err.to_string().contains("Invalid status: \"waiting\""));
}

// ── Non-reply tool behaviour (unchanged) ────────────────────────

#[test]
fn unknown_tool_returns_none() {
    let params = serde_json::json!({ "name": "unknown", "arguments": {} });
    assert!(handle_tool_call(&params).unwrap().is_none());
}

#[test]
fn empty_reply_text_rejected() {
    let params = call_params(serde_json::json!({
        "target": { "kind": "user" }, "text": "", "status": "done"
    }));
    assert!(handle_tool_call(&params).unwrap().is_none());
}

#[test]
fn whitespace_only_reply_text_rejected() {
    let params = call_params(serde_json::json!({
        "target": { "kind": "user" }, "text": " \n\t ", "status": "done"
    }));
    assert!(handle_tool_call(&params).unwrap().is_none());
}

// ── Schema shape ────────────────────────────────────────────────

#[test]
fn reply_schema_uses_structured_target() {
    let schema = reply_tool_schema();
    let target = &schema["inputSchema"]["properties"]["target"];
    assert!(target.is_object());
    assert_eq!(
        target["properties"]["kind"]["enum"],
        serde_json::json!(["user", "role", "agent"])
    );
    // Old `to` field must be absent
    assert!(schema["inputSchema"]["properties"]["to"].is_null());
    assert_eq!(
        schema["inputSchema"]["required"],
        serde_json::json!(["target", "text", "status"])
    );
}

#[test]
fn tool_list_contains_both_tools() {
    let list = tool_list();
    assert_eq!(list.len(), 2);
    assert_eq!(list[0]["name"], "reply");
    assert_eq!(list[1]["name"], "get_online_agents");
}

#[test]
fn get_online_agents_schema_has_no_required_params() {
    let schema = get_online_agents_schema();
    assert_eq!(schema["name"], "get_online_agents");
    assert!(schema["inputSchema"]["properties"].as_object().unwrap().is_empty());
}

#[test]
fn is_get_online_agents_detects_tool() {
    let params = serde_json::json!({ "name": "get_online_agents", "arguments": {} });
    assert!(is_get_online_agents(&params));
}

#[test]
fn is_get_online_agents_rejects_other_tools() {
    let params = serde_json::json!({ "name": "reply", "arguments": {} });
    assert!(!is_get_online_agents(&params));
}
