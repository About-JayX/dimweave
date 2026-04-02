use super::*;
use serde_json::json;

// ── format_user_message ────────────────────────────────

#[test]
fn user_message_has_correct_type() {
    let msg = format_user_message("hello");
    let v: Value = serde_json::from_str(msg.trim()).unwrap();
    assert_eq!(v["type"], "user");
}

#[test]
fn user_message_has_content_array_format() {
    let msg = format_user_message("test content");
    let v: Value = serde_json::from_str(msg.trim()).unwrap();
    let content = &v["message"]["content"];
    assert!(content.is_array());
    assert_eq!(content[0]["type"], "text");
    assert_eq!(content[0]["text"], "test content");
}

#[test]
fn user_message_has_required_fields() {
    let msg = format_user_message("x");
    let v: Value = serde_json::from_str(msg.trim()).unwrap();
    assert_eq!(v["session_id"], "");
    assert_eq!(v["message"]["role"], "user");
    assert!(v["parent_tool_use_id"].is_null());
}

#[test]
fn user_message_ends_with_newline() {
    let msg = format_user_message("x");
    assert!(msg.ends_with('\n'));
    // Single line (no embedded newlines in JSON)
    assert_eq!(msg.trim().lines().count(), 1);
}

// ── format_control_response ────────────────────────────

#[test]
fn allow_response_has_updated_input() {
    let msg = format_control_response("req-1", true);
    let v: Value = serde_json::from_str(msg.trim()).unwrap();
    assert_eq!(v["type"], "control_response");
    assert_eq!(v["response"]["subtype"], "success");
    assert_eq!(v["response"]["request_id"], "req-1");
    let inner = &v["response"]["response"];
    assert_eq!(inner["behavior"], "allow");
    assert!(inner["updatedInput"].is_object(), "TnY schema requires updatedInput");
}

#[test]
fn deny_response_has_message() {
    let msg = format_control_response("req-2", false);
    let v: Value = serde_json::from_str(msg.trim()).unwrap();
    let inner = &v["response"]["response"];
    assert_eq!(inner["behavior"], "deny");
    assert!(inner["message"].is_string(), "knY schema requires message");
}

#[test]
fn control_response_ends_with_newline() {
    assert!(format_control_response("x", true).ends_with('\n'));
    assert!(format_control_response("x", false).ends_with('\n'));
}

// ── format_initialize_response ─────────────────────────

#[test]
fn initialize_response_has_required_fields() {
    let msg = format_initialize_response("init-1");
    let v: Value = serde_json::from_str(msg.trim()).unwrap();
    assert_eq!(v["type"], "control_response");
    assert_eq!(v["response"]["subtype"], "success");
    assert_eq!(v["response"]["request_id"], "init-1");
    let inner = &v["response"]["response"];
    assert!(inner["commands"].is_array());
    assert!(inner["agents"].is_array());
    assert!(inner["output_style"].is_string());
    assert!(inner["models"].is_array());
    assert!(inner["account"].is_object());
}

// ── format_keep_alive ──────────────────────────────────

#[test]
fn keep_alive_format() {
    let msg = format_keep_alive();
    let v: Value = serde_json::from_str(msg.trim()).unwrap();
    assert_eq!(v["type"], "keep_alive");
    assert!(msg.ends_with('\n'));
}

// ── NdjsonEvent deserialization ────────────────────────

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

// ── Stream event structure (used by event_handler) ─────

#[test]
fn stream_event_content_block_delta_structure() {
    // Verify the JSON structure our handle_stream_event expects
    let raw = json!({
        "type": "stream_event",
        "event": {
            "type": "content_block_delta",
            "delta": {
                "type": "text_delta",
                "text": "Hello world"
            }
        }
    });
    let event_type = raw["type"].as_str().unwrap();
    assert_eq!(event_type, "stream_event");
    let inner_type = raw["event"]["type"].as_str().unwrap();
    assert_eq!(inner_type, "content_block_delta");
    let delta_text = raw["event"]["delta"]["text"].as_str().unwrap();
    assert_eq!(delta_text, "Hello world");
}

#[test]
fn stream_event_content_block_start_structure() {
    let raw = json!({
        "type": "stream_event",
        "event": {
            "type": "content_block_start",
            "content_block": {"type": "text"}
        }
    });
    let block_type = raw["event"]["content_block"]["type"].as_str().unwrap();
    assert_eq!(block_type, "text");
}
