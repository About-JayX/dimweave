use super::*;

fn preview(raw: &str) -> Option<String> {
    extract_structured_message_preview(raw)
}

#[test]
fn preview_complete_json() {
    assert_eq!(
        preview(r#"{"message":"Hello world","send_to":"none"}"#),
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
    assert_eq!(preview(r#"{"send_to":"lead"}"#), None);
}
#[test]
fn final_empty_message_not_emitted() {
    let parsed = parse_structured_output(r#"{"message":"   ","send_to":"lead"}"#).unwrap();
    assert_eq!(parsed.send_to.as_deref(), Some("lead"));
    assert!(!should_emit_final_message(&parsed.message));
}
#[test]
fn status_defaults_to_done_when_missing() {
    let parsed = parse_structured_output(r#"{"message":"done","send_to":"lead"}"#).unwrap();
    assert_eq!(parsed.status.as_str(), "done");
}
#[test]
fn parses_explicit_in_progress_status() {
    let parsed =
        parse_structured_output(r#"{"message":"working","send_to":"lead","status":"in_progress"}"#)
            .unwrap();
    assert_eq!(parsed.status.as_str(), "in_progress");
}
#[test]
fn invalid_status_returns_error() {
    let err =
        parse_structured_output(r#"{"message":"working","send_to":"lead","status":"waiting"}"#)
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
        r#"","send_to":"lead"}"#
    );
    assert!(
        s.ingest_delta(&rest).is_none(),
        "no new preview after truncation"
    );
    assert!(!s.last_preview.contains("send_to"));
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
        r#"{"message":"done","send_to":"lead","status":"done","report_telegram":true}"#,
    )
    .unwrap();
    assert_eq!(parsed.message, "done");
    assert_eq!(parsed.status.as_str(), "done");
}
