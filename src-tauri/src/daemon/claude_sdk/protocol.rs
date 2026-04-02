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

// ── Outbound: daemon → Claude (via WS) ─────────────────────

/// Format a user message as NDJSON matching the verified protocol.
/// Format: `{"type":"user","session_id":"","message":{"role":"user","content":[{"type":"text","text":"..."}]},"parent_tool_use_id":null}`
pub fn format_user_message(content: &str) -> String {
    let msg = serde_json::json!({
        "type": "user",
        "session_id": "",
        "message": {
            "role": "user",
            "content": [{"type": "text", "text": content}]
        },
        "parent_tool_use_id": null
    });
    format!("{msg}\n")
}

/// Format a control_response (allow) with required `updatedInput` field.
/// Spec: TnY requires `{ behavior: "allow", updatedInput: {} }`.
pub fn format_control_response(request_id: &str, allow: bool) -> String {
    let inner = if allow {
        serde_json::json!({
            "behavior": "allow",
            "updatedInput": {}
        })
    } else {
        serde_json::json!({
            "behavior": "deny",
            "message": "Denied by AgentNexus daemon"
        })
    };
    let msg = serde_json::json!({
        "type": "control_response",
        "response": {
            "subtype": "success",
            "request_id": request_id,
            "response": inner
        }
    });
    format!("{msg}\n")
}

#[cfg(test)]
#[path = "protocol_tests.rs"]
mod tests;

/// Format a keep_alive message.
pub fn format_keep_alive() -> String {
    let msg = serde_json::json!({"type": "keep_alive"});
    format!("{msg}\n")
}

/// Format an initialize control_response.
pub fn format_initialize_response(request_id: &str) -> String {
    let msg = serde_json::json!({
        "type": "control_response",
        "response": {
            "subtype": "success",
            "request_id": request_id,
            "response": {
                "commands": [],
                "agents": [],
                "output_style": "default",
                "available_output_styles": ["default"],
                "models": [],
                "account": {}
            }
        }
    });
    format!("{msg}\n")
}
