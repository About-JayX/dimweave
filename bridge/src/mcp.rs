use crate::tools::handle_tool_call;
use crate::types::BridgeMessage;
use serde::Deserialize;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum RpcId {
    Number(i64),
    Str(String),
}

#[derive(Debug, Deserialize)]
pub struct RpcMessage {
    pub id: Option<RpcId>,
    pub method: Option<String>,
    pub params: Option<serde_json::Value>,
}

fn id_to_value(id: &Option<RpcId>) -> serde_json::Value {
    match id {
        Some(RpcId::Number(n)) => serde_json::json!(n),
        Some(RpcId::Str(s)) => serde_json::json!(s),
        None => serde_json::Value::Null,
    }
}

pub fn channel_notification(content: &str, chat_id: &str, from: &str) -> serde_json::Value {
    serde_json::json!({
        "jsonrpc": "2.0",
        "method": "notifications/claude/channel",
        "params": {
            "content": content,
            "meta": { "from": from, "chat_id": chat_id }
        }
    })
}

pub async fn run(
    agent_id: String,
    mut push_rx: tokio::sync::mpsc::Receiver<BridgeMessage>,
    reply_tx: tokio::sync::mpsc::Sender<BridgeMessage>,
) {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let mut reader = BufReader::new(stdin);
    let mut writer = tokio::io::BufWriter::new(stdout);
    let mut initialized = false;

    loop {
        let mut line = String::new();
        tokio::select! {
            n = reader.read_line(&mut line) => {
                if n.unwrap_or(0usize) == 0 { break; }
                let trimmed = line.trim();
                if trimmed.is_empty() { continue; }
                let Ok(msg) = serde_json::from_str::<RpcMessage>(trimmed) else { continue };

                match msg.method.as_deref() {
                    Some("initialize") => {
                        initialized = true;
                        let resp = serde_json::json!({
                            "jsonrpc": "2.0",
                            "id": id_to_value(&msg.id),
                            "result": {
                                "protocolVersion": "2024-11-05",
                                "capabilities": {
                                    "tools": {},
                                    "experimental": { "claude/channel": {} }
                                },
                                "serverInfo": { "name": "agentbridge", "version": "0.1.0" }
                            }
                        });
                        write_line(&mut writer, &resp).await;
                    }
                    Some("tools/list") => {
                        let resp = serde_json::json!({
                            "jsonrpc": "2.0",
                            "id": id_to_value(&msg.id),
                            "result": { "tools": [crate::tools::reply_tool_schema()] }
                        });
                        write_line(&mut writer, &resp).await;
                    }
                    Some("tools/call") => {
                        if let Some(params) = &msg.params {
                            if let Some(bridge_msg) = handle_tool_call(params, &agent_id) {
                                let _ = reply_tx.send(bridge_msg).await;
                                let resp = serde_json::json!({
                                    "jsonrpc": "2.0",
                                    "id": id_to_value(&msg.id),
                                    "result": { "content": [{ "type": "text", "text": "sent" }] }
                                });
                                write_line(&mut writer, &resp).await;
                            }
                        }
                    }
                    Some("notifications/initialized") | None => {}
                    _ => {}
                }
            }
            Some(msg) = push_rx.recv() => {
                if initialized {
                    let notif = channel_notification(&msg.content, &msg.id, &msg.from);
                    write_line(&mut writer, &notif).await;
                }
            }
        }
    }
}

async fn write_line(w: &mut tokio::io::BufWriter<tokio::io::Stdout>, val: &serde_json::Value) {
    let mut line = serde_json::to_string(val).unwrap();
    line.push('\n');
    let _ = w.write_all(line.as_bytes()).await;
    let _ = w.flush().await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_initialize_request() {
        let raw = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"claude-code","version":"1.0"}}}"#;
        let msg: RpcMessage = serde_json::from_str(raw).unwrap();
        assert_eq!(msg.method.as_deref(), Some("initialize"));
        assert!(matches!(msg.id, Some(RpcId::Number(1))));
    }

    #[test]
    fn parse_tools_list_request() {
        let raw = r#"{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}"#;
        let msg: RpcMessage = serde_json::from_str(raw).unwrap();
        assert_eq!(msg.method.as_deref(), Some("tools/list"));
    }

    #[test]
    fn serialize_channel_notification() {
        let n = channel_notification("hello", "msg-1", "coder");
        let s = serde_json::to_string(&n).unwrap();
        assert!(s.contains("notifications/claude/channel"));
        assert!(s.contains("hello"));
        assert!(s.contains("coder"));
    }
}
