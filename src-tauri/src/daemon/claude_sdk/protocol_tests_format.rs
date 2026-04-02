use super::super::*;
use serde_json::{json, Value};

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
    assert_eq!(msg.trim().lines().count(), 1);
}

#[test]
fn channel_user_message_wraps_content_in_channel_tag() {
    let msg = format_channel_user_message("coder", "done");
    let v: Value = serde_json::from_str(msg.trim()).unwrap();
    assert_eq!(v["message"]["content"][0]["type"], "text");
    assert_eq!(
        v["message"]["content"][0]["text"],
        "<channel source=\"agentnexus\" from=\"coder\">done</channel>"
    );
}

#[test]
fn channel_user_message_escapes_xml_special_chars() {
    let msg = format_channel_user_message("co\"der", "done </channel> & <next>");
    let v: Value = serde_json::from_str(msg.trim()).unwrap();
    assert_eq!(
        v["message"]["content"][0]["text"],
        "<channel source=\"agentnexus\" from=\"co&quot;der\">done &lt;/channel&gt; &amp; &lt;next&gt;</channel>"
    );
}

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
fn deny_response_has_message_and_updated_input() {
    let msg = format_control_response("req-2", false);
    let v: Value = serde_json::from_str(msg.trim()).unwrap();
    let inner = &v["response"]["response"];
    assert_eq!(inner["behavior"], "deny");
    assert!(inner["message"].is_string(), "knY schema requires message");
    assert!(inner["updatedInput"].is_object(), "deny payload keeps schema shape stable");
}

#[test]
fn control_response_ends_with_newline() {
    assert!(format_control_response("x", true).ends_with('\n'));
    assert!(format_control_response("x", false).ends_with('\n'));
}

#[test]
fn generic_ack_has_empty_success_response() {
    let msg = format_generic_ack("req-generic");
    let v: Value = serde_json::from_str(msg.trim()).unwrap();
    assert_eq!(v["type"], "control_response");
    assert_eq!(v["response"]["subtype"], "success");
    assert_eq!(v["response"]["request_id"], "req-generic");
    assert_eq!(v["response"]["response"], json!({}));
}

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

#[test]
fn keep_alive_format() {
    let msg = format_keep_alive();
    let v: Value = serde_json::from_str(msg.trim()).unwrap();
    assert_eq!(v["type"], "keep_alive");
    assert!(msg.ends_with('\n'));
}
