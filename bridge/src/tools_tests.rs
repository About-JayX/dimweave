use super::*;

#[test]
fn reply_schema_uses_to_field() {
    let schema = reply_tool_schema();
    assert!(schema["inputSchema"]["properties"]["to"].is_object());
    assert_eq!(
        schema["inputSchema"]["required"],
        serde_json::json!(["to", "text", "status"])
    );
    assert!(schema["inputSchema"]["properties"]["chat_id"].is_null());
}

#[test]
fn handle_reply_tool() {
    let params = serde_json::json!({
        "name": "reply",
        "arguments": { "to": "lead", "text": "hello", "status": "done" }
    });
    let msg = handle_tool_call(&params, "coder").unwrap().unwrap();
    assert_eq!(msg.to, "lead");
    assert_eq!(msg.content, "hello");
    assert_eq!(msg.from, "coder");
    assert_eq!(msg.status.unwrap().as_str(), "done");
}

#[test]
fn handle_reply_defaults_missing_status_to_done() {
    let params = serde_json::json!({
        "name": "reply",
        "arguments": { "to": "lead", "text": "hello" }
    });
    let msg = handle_tool_call(&params, "coder").unwrap().unwrap();
    assert_eq!(msg.status.unwrap().as_str(), "done");
}

#[test]
fn invalid_status_returns_explicit_error() {
    let params = serde_json::json!({
        "name": "reply",
        "arguments": { "to": "lead", "text": "hello", "status": "waiting" }
    });
    let err = handle_tool_call(&params, "coder").unwrap_err();
    assert!(
        err.to_string().contains("Invalid status: \"waiting\""),
        "unexpected error: {err}"
    );
}

#[test]
fn unknown_tool_returns_none() {
    let params = serde_json::json!({ "name": "unknown", "arguments": {} });
    assert!(handle_tool_call(&params, "claude").unwrap().is_none());
}

#[test]
fn invalid_target_rejected() {
    let params = serde_json::json!({
        "name": "reply",
        "arguments": { "to": "admin", "text": "hello", "status": "done" }
    });
    assert!(handle_tool_call(&params, "coder").unwrap().is_none());
}

#[test]
fn empty_reply_text_rejected() {
    let params = serde_json::json!({
        "name": "reply",
        "arguments": { "to": "lead", "text": "", "status": "done" }
    });
    assert!(handle_tool_call(&params, "coder").unwrap().is_none());
}

#[test]
fn whitespace_only_reply_text_rejected() {
    let params = serde_json::json!({
        "name": "reply",
        "arguments": { "to": "lead", "text": " \n\t ", "status": "done" }
    });
    assert!(handle_tool_call(&params, "coder").unwrap().is_none());
}

#[test]
fn reply_schema_has_enum_constraint() {
    let schema = reply_tool_schema();
    let to_enum = &schema["inputSchema"]["properties"]["to"]["enum"];
    assert!(to_enum.is_array());
    let targets: Vec<&str> = to_enum
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert_eq!(targets, vec!["user", "lead", "coder"]);
    assert!(!targets.contains(&"tester"));
    assert!(!targets.contains(&"admin"));
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
    assert!(schema["inputSchema"]["properties"]
        .as_object()
        .unwrap()
        .is_empty());
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

#[test]
fn reply_schema_exposes_optional_report_telegram() {
    let schema = reply_tool_schema();
    assert_eq!(
        schema["inputSchema"]["properties"]["report_telegram"]["type"],
        "boolean"
    );
    let required = schema["inputSchema"]["required"]
        .as_array()
        .unwrap();
    assert!(!required.iter().any(|v| v == "report_telegram"));
}

#[test]
fn report_telegram_preserved_for_lead() {
    let params = serde_json::json!({
        "name": "reply",
        "arguments": {
            "to": "user",
            "text": "final summary",
            "status": "done",
            "report_telegram": true
        }
    });
    let msg = handle_tool_call(&params, "lead").unwrap().unwrap();
    assert_eq!(msg.report_telegram, Some(true));
}

#[test]
fn report_telegram_stripped_for_coder() {
    let params = serde_json::json!({
        "name": "reply",
        "arguments": {
            "to": "lead",
            "text": "hello",
            "status": "done",
            "report_telegram": true
        }
    });
    let msg = handle_tool_call(&params, "coder").unwrap().unwrap();
    assert_eq!(msg.report_telegram, None);
}

#[test]
fn handle_reply_defaults_report_telegram_to_none() {
    let params = serde_json::json!({
        "name": "reply",
        "arguments": { "to": "lead", "text": "hello", "status": "done" }
    });
    let msg = handle_tool_call(&params, "lead").unwrap().unwrap();
    assert_eq!(msg.report_telegram, None);
}
