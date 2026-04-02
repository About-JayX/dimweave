use crate::daemon::codex::session::SessionOpts;
use serde_json::{json, Value};

/// Build the JSON params for Codex `thread/start`.
pub(super) fn build_thread_start_params(opts: &SessionOpts) -> Value {
    let mut params = json!({
        "dynamicTools": [
            { "name": "check_messages",
              "description": "Check for new incoming messages from other agents.",
              "inputSchema": {"type":"object","properties":{}} },
            { "name": "get_status",
              "description": "Get Dimweave status as structured JSON: {\"online_agents\": [{\"agentId\", \"role\", \"modelSource\"}]}.",
              "inputSchema": {"type":"object","properties":{}} }
        ]
    });
    if let Some(cwd) = (!opts.cwd.is_empty()).then_some(opts.cwd.as_str()) {
        params["cwd"] = json!(cwd);
    }
    if let Some(m) = &opts.model {
        if !m.is_empty() {
            params["model"] = json!(m);
        }
    }
    if let Some(effort) = &opts.effort {
        if !effort.is_empty() {
            params["effort"] = json!(effort);
        }
    }
    if let Some(sb) = &opts.sandbox_mode {
        if opts.network_access {
            params["sandboxPolicy"] = json!({
                "type": sb,
                "networkAccess": true
            });
        } else {
            params["sandbox"] = json!(sb);
        }
    }
    if let Some(bi) = opts.base_instructions.as_deref().filter(|s| !s.is_empty()) {
        params["baseInstructions"] = json!(bi);
    }
    params
}

#[cfg(test)]
#[path = "handshake_tests.rs"]
mod tests;
