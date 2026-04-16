use super::*;
use crate::daemon::types::MessageTarget;

fn preview(raw: &str) -> Option<String> {
    extract_structured_message_preview(raw)
}

#[test]
fn preview_complete_json() {
    assert_eq!(
        preview(r#"{"message":"Hello world","status":"done"}"#),
        Some("Hello world".into())
    );
}
#[test]
fn preview_partial_message() {
    assert_eq!(
        preview(r#"{"message":"Hello wor"#),
        Some("Hello wor".into())
    );
}
#[test]
fn preview_decodes_escapes() {
    assert_eq!(
        preview(r#"{"message":"line 1\nline 2\tok"#),
        Some("line 1\nline 2\tok".into())
    );
}
#[test]
fn preview_none_without_message_field() {
    assert_eq!(preview(r#"{"target":{"kind":"role","role":"lead"}}"#), None);
}
#[test]
fn final_empty_message_not_emitted() {
    let parsed = parse_structured_output(r#"{"message":"   ","target":{"kind":"role","role":"lead"}}"#).unwrap();
    assert_eq!(parsed.target, Some(MessageTarget::Role { role: "lead".into() }));
    assert!(!should_emit_final_message(&parsed.message));
}
#[test]
fn status_defaults_to_done_when_missing() {
    let parsed = parse_structured_output(r#"{"message":"done","target":{"kind":"role","role":"lead"}}"#).unwrap();
    assert_eq!(parsed.status.as_str(), "done");
}
#[test]
fn parses_explicit_in_progress_status() {
    let parsed =
        parse_structured_output(r#"{"message":"working","target":{"kind":"role","role":"lead"},"status":"in_progress"}"#)
            .unwrap();
    assert_eq!(parsed.status.as_str(), "in_progress");
}
#[test]
fn invalid_status_returns_error() {
    let err =
        parse_structured_output(r#"{"message":"working","target":{"kind":"role","role":"lead"},"status":"waiting"}"#)
            .unwrap_err();
    assert!(
        err.to_string().contains("Invalid status: \"waiting\""),
        "unexpected error: {err}"
    );
}
#[test]
fn raw_delta_cap_enforced() {
    let mut s = StreamPreviewState::default();
    s.ingest_delta(&"x".repeat(RAW_DELTA_CAP + 100));
    assert!(s.raw_delta.len() <= RAW_DELTA_CAP);
}
#[test]
fn truncation_does_not_leak_json_wrapper() {
    let mut s = StreamPreviewState::default();
    s.ingest_delta(r#"{"message":"Hello preview"#);
    assert_eq!(s.last_preview, "Hello preview");
    let rest = format!(
        "{}{}",
        "A".repeat(RAW_DELTA_CAP + 200),
        r#"","target":{"kind":"role","role":"lead"}}"#
    );
    assert!(
        s.ingest_delta(&rest).is_none(),
        "no new preview after truncation"
    );
    assert!(!s.last_preview.contains("target"));
    assert_eq!(s.last_preview, "Hello preview");
    assert!(s.truncated);
}
#[test]
fn truncated_flag_resets_on_new_turn() {
    let mut s = StreamPreviewState::default();
    s.ingest_delta(&"x".repeat(RAW_DELTA_CAP + 100));
    assert!(s.truncated);
    s.reset();
    assert!(!s.truncated);
    assert!(s.raw_delta.is_empty());
}

#[test]
fn reasoning_accumulates_and_resets_per_turn() {
    let mut s = StreamPreviewState::default();
    s.append_reasoning("first ");
    s.append_reasoning("second");
    assert_eq!(s.reasoning_text(), "first second");
    s.reset();
    assert_eq!(s.reasoning_text(), "");
}

#[test]
fn reasoning_cap_keeps_latest_text() {
    let mut s = StreamPreviewState::default();
    s.append_reasoning(&"a".repeat(8_100));
    assert!(s.reasoning_text().len() <= 8_000);
    assert!(s.reasoning_text().chars().all(|ch| ch == 'a'));
}

#[test]
fn reasoning_boundary_separates_sections() {
    let mut s = StreamPreviewState::default();
    s.append_reasoning("First section");
    s.append_reasoning_boundary();
    s.append_reasoning("Second section");
    assert_eq!(s.reasoning_text(), "First section\n\nSecond section");
}

#[test]
fn ignores_unknown_report_telegram_field_gracefully() {
    let parsed = parse_structured_output(
        r#"{"message":"done","target":{"kind":"role","role":"lead"},"status":"done","report_telegram":true}"#,
    )
    .unwrap();
    assert_eq!(parsed.message, "done");
    assert_eq!(parsed.target, Some(MessageTarget::Role { role: "lead".into() }));
    assert_eq!(parsed.status.as_str(), "done");
}

// ── Structured target tests ──────────────────────────────────

#[test]
fn send_to_field_is_ignored_after_hard_cut() {
    let parsed = parse_structured_output(r#"{"message":"hi","send_to":"coder"}"#).unwrap();
    assert_eq!(parsed.target, None, "send_to must no longer produce a target");
}

#[test]
fn structured_role_target_object() {
    let raw = r#"{"message":"hi","target":{"kind":"role","role":"lead"}}"#;
    let parsed = parse_structured_output(raw).unwrap();
    assert_eq!(parsed.target, Some(MessageTarget::Role { role: "lead".into() }));
}

#[test]
fn structured_user_target_object() {
    let raw = r#"{"message":"hi","target":{"kind":"user"}}"#;
    let parsed = parse_structured_output(raw).unwrap();
    assert_eq!(parsed.target, Some(MessageTarget::User));
}

#[test]
fn structured_user_target_with_empty_extra_fields() {
    // Codex strict schema requires all three fields; serde should ignore empty extras
    let raw = r#"{"message":"hi","target":{"kind":"user","role":"","agentId":""}}"#;
    let parsed = parse_structured_output(raw).unwrap();
    assert_eq!(parsed.target, Some(MessageTarget::User));
}

#[test]
fn structured_role_target_with_empty_agent_id() {
    let raw = r#"{"message":"hi","target":{"kind":"role","role":"coder","agentId":""}}"#;
    let parsed = parse_structured_output(raw).unwrap();
    assert_eq!(parsed.target, Some(MessageTarget::Role { role: "coder".into() }));
}

#[test]
fn structured_agent_target_with_empty_role() {
    let raw = r#"{"message":"hi","target":{"kind":"agent","role":"","agentId":"claude-1"}}"#;
    let parsed = parse_structured_output(raw).unwrap();
    assert_eq!(parsed.target, Some(MessageTarget::Agent { agent_id: "claude-1".into() }));
}

#[test]
fn structured_agent_target_object() {
    let raw = r#"{"message":"hi","target":{"kind":"agent","agentId":"claude"}}"#;
    let parsed = parse_structured_output(raw).unwrap();
    assert_eq!(parsed.target, Some(MessageTarget::Agent { agent_id: "claude".into() }));
}

#[test]
fn structured_target_ignores_stale_send_to() {
    let raw = r#"{"message":"hi","send_to":"user","target":{"kind":"role","role":"lead"}}"#;
    let parsed = parse_structured_output(raw).unwrap();
    assert_eq!(parsed.target, Some(MessageTarget::Role { role: "lead".into() }));
}

#[test]
fn missing_target_and_send_to_yields_none() {
    let parsed = parse_structured_output(r#"{"message":"hi"}"#).unwrap();
    assert_eq!(parsed.target, None);
}

#[test]
fn reply_target_parsed_from_camel_case() {
    let raw = r#"{"message":"hi","target":{"kind":"role","role":"lead"},"replyTarget":{"kind":"agent","agentId":"codex"}}"#;
    let parsed = parse_structured_output(raw).unwrap();
    assert_eq!(parsed.reply_target, Some(MessageTarget::Agent { agent_id: "codex".into() }));
}

#[test]
fn reply_target_parsed_from_snake_case() {
    let raw = r#"{"message":"hi","reply_target":{"kind":"user"}}"#;
    let parsed = parse_structured_output(raw).unwrap();
    assert_eq!(parsed.reply_target, Some(MessageTarget::User));
}

#[test]
fn reply_target_absent_yields_none() {
    let parsed = parse_structured_output(r#"{"message":"hi","target":{"kind":"role","role":"lead"}}"#).unwrap();
    assert_eq!(parsed.reply_target, None);
}

#[test]
fn raw_text_fallback_has_no_target() {
    let parsed = parse_structured_output("plain text output").unwrap();
    assert_eq!(parsed.target, None);
    assert_eq!(parsed.reply_target, None);
    assert_eq!(parsed.message, "plain text output");
}

// ── Durable/transient tracking tests ────────────────────────

#[test]
fn tracking_flags_default_to_false() {
    let s = StreamPreviewState::default();
    assert!(!s.had_durable_output());
    assert!(!s.had_transient_content());
}

#[test]
fn ingest_delta_sets_transient_content() {
    let mut s = StreamPreviewState::default();
    s.ingest_delta("hello");
    assert!(s.had_transient_content());
    assert!(!s.had_durable_output());
}

#[test]
fn append_reasoning_sets_transient_content() {
    let mut s = StreamPreviewState::default();
    s.append_reasoning("thinking...");
    assert!(s.had_transient_content());
    assert!(!s.had_durable_output());
}

#[test]
fn mark_durable_output_sets_flag() {
    let mut s = StreamPreviewState::default();
    s.mark_durable_output();
    assert!(s.had_durable_output());
}

#[test]
fn mark_transient_content_sets_flag() {
    let mut s = StreamPreviewState::default();
    s.mark_transient_content();
    assert!(s.had_transient_content());
}

#[test]
fn reset_clears_tracking_flags() {
    let mut s = StreamPreviewState::default();
    s.mark_durable_output();
    s.mark_transient_content();
    s.ingest_delta("data");
    s.append_reasoning("think");
    assert!(s.had_durable_output());
    assert!(s.had_transient_content());
    s.reset();
    assert!(!s.had_durable_output());
    assert!(!s.had_transient_content());
}

#[test]
fn silent_turn_detected_when_transient_only() {
    let mut s = StreamPreviewState::default();
    s.ingest_delta("partial");
    s.append_reasoning("thinking");
    s.mark_transient_content();
    // No mark_durable_output — simulates a silent turn
    assert!(!s.had_durable_output() && s.had_transient_content());
}

#[test]
fn durable_turn_not_flagged_as_silent() {
    let mut s = StreamPreviewState::default();
    s.ingest_delta("partial");
    s.mark_transient_content();
    s.mark_durable_output();
    // Both flags set — not a silent turn
    assert!(s.had_durable_output());
}
