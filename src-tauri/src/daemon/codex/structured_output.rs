use serde_json::Value;
use std::fmt;

use crate::daemon::types::MessageStatus;

/// Max bytes in raw delta buffer; bounds Rust-side memory for long responses.
const RAW_DELTA_CAP: usize = 512_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ParsedOutput {
    pub(super) message: String,
    pub(super) send_to: Option<String>,
    pub(super) status: MessageStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum StructuredOutputError {
    InvalidStatus(String),
}

impl fmt::Display for StructuredOutputError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidStatus(value) => write!(
                f,
                "Invalid status: \"{value}\". Expected \"in_progress\", \"done\", or \"error\"."
            ),
        }
    }
}

#[derive(Default)]
pub(super) struct StreamPreviewState {
    raw_delta: String,
    last_preview: String,
    /// Once truncation destroys the JSON prefix, stop re-parsing.
    truncated: bool,
}

impl StreamPreviewState {
    pub(super) fn reset(&mut self) {
        self.raw_delta.clear();
        self.last_preview.clear();
        self.truncated = false;
    }

    pub(super) fn ingest_delta(&mut self, text: &str) -> Option<String> {
        self.raw_delta.push_str(text);
        if self.raw_delta.len() > RAW_DELTA_CAP {
            let drop = self.raw_delta.len() - RAW_DELTA_CAP;
            let mut b = drop;
            while b < self.raw_delta.len() && !self.raw_delta.is_char_boundary(b) { b += 1; }
            self.raw_delta.drain(..b);
            self.truncated = true;
        }
        if self.truncated { return None; }
        let preview = extract_structured_message_preview(&self.raw_delta)?;
        if preview == self.last_preview {
            return None;
        }
        self.last_preview = preview.clone();
        Some(preview)
    }

    pub(super) fn sync_final_raw(&mut self, raw: &str) {
        self.raw_delta.clear();
        self.raw_delta.push_str(raw);
    }
}

pub(super) fn parse_structured_output(raw: &str) -> Result<ParsedOutput, StructuredOutputError> {
    if let Ok(v) = serde_json::from_str::<Value>(raw) {
        let status = match v.get("status") {
            Some(value) => {
                let raw = value.as_str().unwrap_or_default();
                MessageStatus::parse(raw).ok_or_else(|| {
                    StructuredOutputError::InvalidStatus(if raw.is_empty() {
                        value.to_string()
                    } else {
                        raw.to_string()
                    })
                })?
            }
            None => MessageStatus::Done,
        };
        Ok(ParsedOutput {
            message: v["message"].as_str().unwrap_or(raw).to_string(),
            send_to: v["send_to"].as_str().map(str::to_string),
            status,
        })
    } else {
        Ok(ParsedOutput {
            message: raw.to_string(),
            send_to: None,
            status: MessageStatus::Done,
        })
    }
}

pub(super) fn should_emit_final_message(text: &str) -> bool { !text.trim().is_empty() }

fn extract_structured_message_preview(raw: &str) -> Option<String> {
    if let Ok(v) = serde_json::from_str::<Value>(raw) {
        let msg = v["message"].as_str().unwrap_or("").to_string();
        return should_emit_final_message(&msg).then_some(msg);
    }
    if !raw.trim_start().starts_with('{') {
        return should_emit_final_message(raw).then_some(raw.to_string());
    }
    let start = find_message_value_start(raw)?;
    let preview = decode_partial_json_string(&raw[start..]);
    should_emit_final_message(&preview).then_some(preview)
}

fn find_message_value_start(raw: &str) -> Option<usize> {
    let key_idx = raw.find("\"message\"")?;
    let mut idx = key_idx + "\"message\"".len();
    while let Some(ch) = raw[idx..].chars().next() {
        if ch.is_whitespace() {
            idx += ch.len_utf8();
            continue;
        }
        if ch == ':' {
            idx += ch.len_utf8();
            break;
        }
        return None;
    }
    while let Some(ch) = raw[idx..].chars().next() {
        if ch.is_whitespace() {
            idx += ch.len_utf8();
            continue;
        }
        if ch == '"' {
            return Some(idx + ch.len_utf8());
        }
        return None;
    }
    None
}

fn decode_partial_json_string(raw: &str) -> String {
    let mut out = String::new();
    let mut chars = raw.chars();
    let mut escaping = false;

    while let Some(ch) = chars.next() {
        if escaping {
            match ch {
                '"' => out.push('"'),
                '\\' => out.push('\\'),
                '/' => out.push('/'),
                'b' => out.push('\u{0008}'),
                'f' => out.push('\u{000c}'),
                'n' => out.push('\n'),
                'r' => out.push('\r'),
                't' => out.push('\t'),
                'u' => {
                    let mut hex = String::new();
                    for _ in 0..4 {
                        let Some(next) = chars.next() else {
                            return out;
                        };
                        hex.push(next);
                    }
                    if let Ok(code) = u32::from_str_radix(&hex, 16) {
                        if let Some(decoded) = char::from_u32(code) {
                            out.push(decoded);
                        }
                    }
                }
                _ => out.push(ch),
            }
            escaping = false;
            continue;
        }

        match ch {
            '\\' => escaping = true,
            '"' => break,
            _ => out.push(ch),
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn preview(raw: &str) -> Option<String> { extract_structured_message_preview(raw) }

    #[test]
    fn preview_complete_json() {
        assert_eq!(preview(r#"{"message":"Hello world","send_to":"none"}"#), Some("Hello world".into()));
    }
    #[test]
    fn preview_partial_message() {
        assert_eq!(preview(r#"{"message":"Hello wor"#), Some("Hello wor".into()));
    }
    #[test]
    fn preview_decodes_escapes() {
        assert_eq!(preview(r#"{"message":"line 1\nline 2\tok"#), Some("line 1\nline 2\tok".into()));
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
        let parsed = parse_structured_output(
            r#"{"message":"working","send_to":"lead","status":"in_progress"}"#,
        )
        .unwrap();
        assert_eq!(parsed.status.as_str(), "in_progress");
    }
    #[test]
    fn invalid_status_returns_error() {
        let err = parse_structured_output(
            r#"{"message":"working","send_to":"lead","status":"waiting"}"#,
        )
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
        let rest = format!("{}{}","A".repeat(RAW_DELTA_CAP + 200), r#"","send_to":"lead"}"#);
        assert!(s.ingest_delta(&rest).is_none(), "no new preview after truncation");
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
}
