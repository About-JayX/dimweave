use serde_json::Value;
use std::fmt;

use crate::daemon::types::{MessageStatus, MessageTarget};

/// Max bytes in raw delta buffer; bounds Rust-side memory for long responses.
const RAW_DELTA_CAP: usize = 512_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ParsedOutput {
    pub(super) message: String,
    pub(super) target: Option<MessageTarget>,
    pub(super) reply_target: Option<MessageTarget>,
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
    reasoning: String,
    /// Set when a durable message (delivered/buffered/ToGui) was produced this turn.
    had_durable_output: bool,
    /// Set when any transient activity (reasoning, deltas, tool calls) occurred this turn.
    had_transient_content: bool,
}

const REASONING_CAP: usize = 8_000;

impl StreamPreviewState {
    pub(super) fn reset(&mut self) {
        self.raw_delta.clear();
        self.last_preview.clear();
        self.truncated = false;
        self.reasoning.clear();
        self.had_durable_output = false;
        self.had_transient_content = false;
    }

    pub(super) fn append_reasoning(&mut self, delta: &str) {
        self.had_transient_content = true;
        self.reasoning.push_str(delta);
        if self.reasoning.len() > REASONING_CAP {
            let drop = self.reasoning.len() - REASONING_CAP;
            let mut b = drop;
            while b < self.reasoning.len() && !self.reasoning.is_char_boundary(b) {
                b += 1;
            }
            self.reasoning.drain(..b);
        }
    }

    pub(super) fn append_reasoning_boundary(&mut self) {
        if self.reasoning.is_empty() || self.reasoning.ends_with("\n\n") {
            return;
        }
        if self.reasoning.ends_with('\n') {
            self.reasoning.push('\n');
        } else {
            self.reasoning.push_str("\n\n");
        }
    }

    pub(super) fn reasoning_text(&self) -> &str {
        &self.reasoning
    }

    pub(super) fn ingest_delta(&mut self, text: &str) -> Option<String> {
        self.had_transient_content = true;
        self.raw_delta.push_str(text);
        if self.raw_delta.len() > RAW_DELTA_CAP {
            let drop = self.raw_delta.len() - RAW_DELTA_CAP;
            let mut b = drop;
            while b < self.raw_delta.len() && !self.raw_delta.is_char_boundary(b) {
                b += 1;
            }
            self.raw_delta.drain(..b);
            self.truncated = true;
        }
        if self.truncated {
            return None;
        }
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

    pub(super) fn mark_durable_output(&mut self) {
        self.had_durable_output = true;
    }

    pub(super) fn mark_transient_content(&mut self) {
        self.had_transient_content = true;
    }

    pub(super) fn had_durable_output(&self) -> bool {
        self.had_durable_output
    }

    pub(super) fn had_transient_content(&self) -> bool {
        self.had_transient_content
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
            target: parse_target_field(&v),
            reply_target: parse_reply_target_field(&v),
            status,
        })
    } else {
        Ok(ParsedOutput {
            message: raw.to_string(),
            target: None,
            reply_target: None,
            status: MessageStatus::Done,
        })
    }
}

fn parse_target_field(v: &Value) -> Option<MessageTarget> {
    let obj = v.get("target")?;
    serde_json::from_value::<MessageTarget>(obj.clone()).ok()
}

fn parse_reply_target_field(v: &Value) -> Option<MessageTarget> {
    let obj = v.get("replyTarget").or_else(|| v.get("reply_target"))?;
    serde_json::from_value::<MessageTarget>(obj.clone()).ok()
}

pub(super) fn should_emit_final_message(text: &str) -> bool {
    !text.trim().is_empty()
}

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
#[path = "structured_output_tests.rs"]
mod tests;
