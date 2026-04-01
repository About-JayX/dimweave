//! NDJSON protocol types for Claude Code `--sdk-url` communication.
//!
//! Inbound (daemon → Claude): WS text frames, one NDJSON object per line.
//! Outbound (Claude → daemon): HTTP POST body `{ "events": [...] }`.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Top-level envelope for events POSTed by Claude to `/claude/events`.
#[derive(Debug, Deserialize)]
pub struct PostEventsBody {
    pub events: Vec<Value>,
}

/// Inbound NDJSON event types that Claude sends to the daemon.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum NdjsonEvent {
    /// Claude system/init event with session metadata.
    System {
        #[serde(flatten)]
        payload: Value,
    },
    /// User message echo (replayed).
    User {
        #[serde(flatten)]
        payload: Value,
    },
    /// Assistant response containing `message.content`.
    Assistant {
        #[serde(default)]
        message: Value,
        #[serde(flatten)]
        rest: Value,
    },
    /// Final result event when the turn completes.
    Result {
        #[serde(default)]
        result: Value,
        #[serde(flatten)]
        rest: Value,
    },
    /// Permission/tool-use approval request from Claude.
    #[serde(rename = "control_request")]
    ControlRequest {
        request_id: String,
        #[serde(default)]
        request: ControlRequestInner,
    },
    /// Permission verdict sent back to Claude.
    #[serde(rename = "control_response")]
    ControlResponse {
        #[serde(flatten)]
        payload: Value,
    },
    /// Keep-alive ping.
    #[serde(rename = "keep_alive")]
    KeepAlive {
        #[serde(flatten)]
        payload: Value,
    },
    /// Rate-limit notification.
    #[serde(rename = "rate_limit_event")]
    RateLimitEvent {
        #[serde(flatten)]
        payload: Value,
    },
}

/// Inner body of a `control_request` event.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ControlRequestInner {
    #[serde(default)]
    pub subtype: String,
    #[serde(default)]
    pub tool_name: Option<String>,
    #[serde(default)]
    pub input: Option<Value>,
    #[serde(default)]
    pub description: Option<String>,
}

/// NDJSON user message sent from daemon → Claude over WS.
#[derive(Serialize)]
pub struct OutboundUserMessage {
    #[serde(rename = "type")]
    pub msg_type: &'static str,
    pub message: OutboundMessageBody,
}

#[derive(Serialize)]
pub struct OutboundMessageBody {
    pub role: &'static str,
    pub content: String,
}

/// NDJSON control_response sent from daemon → Claude over WS.
#[derive(Serialize)]
pub struct OutboundControlResponse {
    #[serde(rename = "type")]
    pub msg_type: &'static str,
    pub response: ControlResponseBody,
}

#[derive(Serialize)]
pub struct ControlResponseBody {
    pub subtype: &'static str,
    pub request_id: String,
    pub response: ControlVerdictBody,
}

#[derive(Serialize)]
pub struct ControlVerdictBody {
    pub behavior: String,
}

/// Format a user message as an NDJSON line (with trailing newline).
pub fn format_user_message(content: &str) -> String {
    let msg = OutboundUserMessage {
        msg_type: "user",
        message: OutboundMessageBody {
            role: "user",
            content: content.to_string(),
        },
    };
    let mut line = serde_json::to_string(&msg).unwrap_or_default();
    line.push('\n');
    line
}

/// Format a control_response verdict as an NDJSON line.
pub fn format_control_response(request_id: &str, allow: bool) -> String {
    let msg = OutboundControlResponse {
        msg_type: "control_response",
        response: ControlResponseBody {
            subtype: "success",
            request_id: request_id.to_string(),
            response: ControlVerdictBody {
                behavior: if allow { "allow" } else { "deny" }.into(),
            },
        },
    };
    let mut line = serde_json::to_string(&msg).unwrap_or_default();
    line.push('\n');
    line
}
