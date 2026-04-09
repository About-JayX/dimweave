use super::*;

#[test]
fn parse_initialize_request() {
    let raw = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"claude-code","version":"1.0"}}}"#;
    let msg: RpcMessage = serde_json::from_str(raw).unwrap();
    assert_eq!(msg.method.as_deref(), Some("initialize"));
    assert!(matches!(msg.id, Some(RpcId::Number(1))));
}

#[test]
fn parse_tools_list_request() {
    let raw = r#"{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}"#;
    let msg: RpcMessage = serde_json::from_str(raw).unwrap();
    assert_eq!(msg.method.as_deref(), Some("tools/list"));
}

#[test]
fn initialize_result_includes_instructions_and_permission_capability() {
    let result = initialize_result("lead", true);
    assert_eq!(
        result["capabilities"]["experimental"]["claude/channel"],
        serde_json::json!({})
    );
    assert_eq!(
        result["capabilities"]["experimental"]["claude/channel/permission"],
        serde_json::json!({})
    );
    assert!(result["instructions"]
        .as_str()
        .unwrap_or_default()
        .contains("<channel source=\"agentnexus\""));
}

#[test]
fn initialize_result_includes_silence_rules() {
    let result = initialize_result("coder", true);
    let instructions = result["instructions"].as_str().unwrap_or_default();
    assert!(
        instructions.contains("Stay completely silent"),
        "channel instructions must include strict silence rule"
    );
    assert!(
        instructions.contains("Do NOT call reply()"),
        "channel instructions must prohibit reply() for non-addressed messages"
    );
    assert!(
        !instructions.contains("Proactively report progress"),
        "channel instructions must NOT contain loose 'proactively report' directive"
    );
}

#[test]
fn initialize_result_mentions_reply_status_contract() {
    let result = initialize_result("lead", true);
    let instructions = result["instructions"].as_str().unwrap_or_default();
    assert!(instructions.contains("reply(to, text, status, report_telegram?)"));
    assert!(instructions.contains("in_progress"));
    assert!(instructions.contains("done"));
    assert!(instructions.contains("error"));
    assert!(instructions.contains("lead is your default recipient"));
}

#[test]
fn serialize_channel_notification() {
    let n = channel_notification("hello", "coder");
    let s = serde_json::to_string(&n).unwrap();
    assert!(s.contains("notifications/claude/channel"));
    assert!(s.contains("hello"));
    assert!(s.contains("coder"));
}

#[test]
fn instructions_document_online_agents_query() {
    let result = initialize_result("lead", true);
    let instructions = result["instructions"].as_str().unwrap_or_default();
    assert!(
        instructions.contains("get_online_agents"),
        "instructions must mention get_online_agents tool"
    );
    assert!(
        instructions.contains("agent_id"),
        "instructions must describe agent_id field in online_agents response"
    );
    assert!(
        instructions.contains("role"),
        "instructions must describe role field in online_agents response"
    );
    assert!(
        instructions.contains("model_source"),
        "instructions must describe model_source field in online_agents response"
    );
    assert!(
        instructions.contains("transport layer does NOT automatically select"),
        "instructions must state that lead must choose the target agent"
    );
}

#[test]
fn channel_instructions_mention_report_telegram_in_reply() {
    let result = initialize_result("lead", true);
    let instructions = result["instructions"].as_str().unwrap_or_default();
    assert!(instructions.contains("report_telegram"));
}

#[test]
fn initialize_result_omits_permission_capability_in_sdk_mode() {
    let result = initialize_result("lead", false);
    assert_eq!(
        result["capabilities"]["experimental"]["claude/channel"],
        serde_json::json!({})
    );
    assert!(result["capabilities"]["experimental"]["claude/channel/permission"].is_null());
}
