use super::super::*;
use serde_json::json;

#[test]
fn parse_assistant_event() {
    let raw = json!({
        "type": "assistant",
        "session_id": "sess-1",
        "message": {
            "content": [{"type": "text", "text": "hello"}],
            "model": "claude-opus-4-6"
        }
    });
    let event: NdjsonEvent = serde_json::from_value(raw).unwrap();
    assert!(matches!(event, NdjsonEvent::Assistant { .. }));
}

#[test]
fn parse_control_request_can_use_tool() {
    let raw = json!({
        "type": "control_request",
        "request_id": "req-abc",
        "request": {
            "subtype": "can_use_tool",
            "tool_name": "Bash",
            "input": {"command": "ls"},
            "description": "List files"
        }
    });
    let event: NdjsonEvent = serde_json::from_value(raw).unwrap();
    match event {
        NdjsonEvent::ControlRequest { request_id, request } => {
            assert_eq!(request_id, "req-abc");
            assert_eq!(request.subtype, "can_use_tool");
            assert_eq!(request.tool_name.as_deref(), Some("Bash"));
            assert_eq!(request.description.as_deref(), Some("List files"));
        }
        _ => panic!("expected ControlRequest"),
    }
}

#[test]
fn parse_control_request_initialize() {
    let raw = json!({
        "type": "control_request",
        "request_id": "req-init",
        "request": { "subtype": "initialize" }
    });
    let event: NdjsonEvent = serde_json::from_value(raw).unwrap();
    match event {
        NdjsonEvent::ControlRequest { request, .. } => {
            assert_eq!(request.subtype, "initialize");
        }
        _ => panic!("expected ControlRequest"),
    }
}

#[test]
fn parse_result_success() {
    let raw = json!({
        "type": "result",
        "subtype": "success",
        "result": "done text",
        "session_id": "sess-1",
        "total_cost_usd": 0.05
    });
    let event: NdjsonEvent = serde_json::from_value(raw).unwrap();
    match event {
        NdjsonEvent::Result { result, .. } => {
            assert_eq!(result.as_str(), Some("done text"));
        }
        _ => panic!("expected Result"),
    }
}

#[test]
fn parse_system_init() {
    let raw = json!({
        "type": "system",
        "subtype": "init",
        "session_id": "sess-1",
        "model": "claude-opus-4-6",
        "tools": ["Bash", "Read"]
    });
    let event: NdjsonEvent = serde_json::from_value(raw).unwrap();
    assert!(matches!(event, NdjsonEvent::System { .. }));
}

#[test]
fn parse_keep_alive() {
    let raw = json!({"type": "keep_alive"});
    let event: NdjsonEvent = serde_json::from_value(raw).unwrap();
    assert!(matches!(event, NdjsonEvent::KeepAlive { .. }));
}

#[test]
fn parse_rate_limit_event() {
    let raw = json!({
        "type": "rate_limit_event",
        "rate_limit_info": {"status": "allowed"}
    });
    let event: NdjsonEvent = serde_json::from_value(raw).unwrap();
    assert!(matches!(event, NdjsonEvent::RateLimitEvent { .. }));
}

#[test]
fn parse_post_events_body() {
    let raw = json!({
        "events": [
            {"type": "system", "subtype": "init"},
            {"type": "assistant", "message": {"content": "hi"}}
        ]
    });
    let body: PostEventsBody = serde_json::from_value(raw).unwrap();
    assert_eq!(body.events.len(), 2);
}
