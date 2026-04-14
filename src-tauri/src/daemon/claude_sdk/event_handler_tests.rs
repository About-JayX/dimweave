use super::*;
use crate::daemon::types::MessageStatus;
use serde_json::json;

#[test]
fn in_progress_sdk_text_does_not_create_visible_gui_message() {
    let msg = build_direct_sdk_gui_message("lead", "partial reply", MessageStatus::InProgress, "agent-1", "claude");
    assert!(msg.is_none());
}

#[test]
fn terminal_sdk_text_creates_visible_gui_message() {
    let msg = build_direct_sdk_gui_message("lead", "final reply", MessageStatus::Done, "agent-1", "claude")
        .expect("done messages should be visible");

    assert_eq!(msg.from, "lead");
    assert_eq!(msg.display_source.as_deref(), Some("claude"));
    assert_eq!(msg.to, "user");
    assert_eq!(msg.content, "final reply");
    assert_eq!(msg.status, Some(MessageStatus::Done));
    assert_eq!(msg.sender_agent_id.as_deref(), Some("agent-1"));
}

#[test]
fn extract_text_from_content_array() {
    let event = json!({
        "message": {
            "content": [
                {"type": "text", "text": "Hello "},
                {"type": "tool_use", "name": "Bash"},
                {"type": "text", "text": "world"}
            ]
        }
    });
    assert_eq!(extract_assistant_text(&event), "Hello world");
}

#[test]
fn extract_text_from_string_content() {
    let event = json!({"message": {"content": "plain text"}});
    assert_eq!(extract_assistant_text(&event), "plain text");
}

#[test]
fn extract_text_returns_empty_for_missing_content() {
    let event = json!({"message": {}});
    assert_eq!(extract_assistant_text(&event), "");
}

#[test]
fn extract_text_returns_empty_for_only_tool_use() {
    let event = json!({
        "message": {
            "content": [{"type": "tool_use", "name": "Edit"}]
        }
    });
    assert_eq!(extract_assistant_text(&event), "");
}

#[test]
fn stream_event_text_delta_extracts_text() {
    let event = json!({
        "type": "stream_event",
        "event": {
            "type": "content_block_delta",
            "delta": {"type": "text_delta", "text": "Hello"}
        }
    });
    let inner = &event["event"];
    let delta_type = inner["delta"]["type"].as_str().unwrap();
    let text = inner["delta"]["text"].as_str().unwrap();
    assert_eq!(delta_type, "text_delta");
    assert_eq!(text, "Hello");
}

#[test]
fn stream_event_non_text_delta_has_no_text() {
    let event = json!({
        "type": "stream_event",
        "event": {
            "type": "content_block_delta",
            "delta": {"type": "input_json_delta", "partial_json": "{\"cmd\""}
        }
    });
    let text = event["event"]["delta"]["text"].as_str();
    assert!(text.is_none());
}

#[test]
fn stream_event_content_block_start_text_type() {
    let event = json!({
        "type": "stream_event",
        "event": {
            "type": "content_block_start",
            "content_block": {"type": "text"}
        }
    });
    let block_type = event["event"]["content_block"]["type"].as_str().unwrap();
    assert_eq!(block_type, "text");
}

#[test]
fn stream_event_content_block_start_tool_use_type() {
    let event = json!({
        "type": "stream_event",
        "event": {
            "type": "content_block_start",
            "content_block": {"type": "tool_use", "name": "Bash"}
        }
    });
    let block_type = event["event"]["content_block"]["type"].as_str().unwrap();
    assert_eq!(block_type, "tool_use");
}
