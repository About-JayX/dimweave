//! MCP client API wrapping the HTTP transport.
//!
//! Provides `initialize`, `list_tools`, and `call_tool` on top of
//! `McpHttpTransport`. Manages request ID sequencing and connection state.

use super::mcp_http::{JsonRpcResponse, McpHttpTransport, McpTransportError};
use super::tool_catalog::{McpToolCatalog, McpToolInfo};
use super::types::McpConnectionStatus;
use serde_json::Value;
use std::sync::atomic::{AtomicU64, Ordering};

const PROTOCOL_VERSION: &str = "2024-11-05";
const CLIENT_NAME: &str = "dimweave";
const CLIENT_VERSION: &str = "0.1.0";

pub struct McpClient {
    transport: McpHttpTransport,
    next_id: AtomicU64,
    pub status: McpConnectionStatus,
    pub catalog: McpToolCatalog,
    pub last_error: Option<String>,
}

impl McpClient {
    pub fn new(domain: &str, token: &str) -> Self {
        Self {
            transport: McpHttpTransport::new(domain, token),
            next_id: AtomicU64::new(1),
            status: McpConnectionStatus::Disconnected,
            catalog: McpToolCatalog::default(),
            last_error: None,
        }
    }

    fn next_id(&self) -> u64 {
        self.next_id.fetch_add(1, Ordering::SeqCst)
    }

    /// Connect: send `initialize` then `tools/list`.
    pub async fn connect(&mut self) -> Result<(), String> {
        self.status = McpConnectionStatus::Connecting;
        self.last_error = None;
        if let Err(e) = self.do_initialize().await {
            self.set_error(e);
            return Err(self.last_error.clone().unwrap());
        }
        if let Err(e) = self.do_list_tools().await {
            self.set_error(e);
            return Err(self.last_error.clone().unwrap());
        }
        self.status = McpConnectionStatus::Connected;
        Ok(())
    }

    pub fn disconnect(&mut self) {
        self.status = McpConnectionStatus::Disconnected;
        self.catalog = McpToolCatalog::default();
        self.last_error = None;
    }

    async fn do_initialize(&mut self) -> Result<Value, McpTransportError> {
        let params = serde_json::json!({
            "protocolVersion": PROTOCOL_VERSION,
            "capabilities": {},
            "clientInfo": {
                "name": CLIENT_NAME,
                "version": CLIENT_VERSION
            }
        });
        let resp = self
            .transport
            .send(self.next_id(), "initialize", Some(params))
            .await?;
        extract_result(resp)
    }

    async fn do_list_tools(&mut self) -> Result<(), McpTransportError> {
        let resp = self
            .transport
            .send(self.next_id(), "tools/list", None)
            .await?;
        let result = extract_result(resp)?;
        self.catalog = McpToolCatalog::from_tools_list_result(&result);
        Ok(())
    }

    /// Call a single MCP tool by name.
    pub async fn call_tool(
        &self,
        name: &str,
        arguments: Value,
    ) -> Result<Value, String> {
        let params = serde_json::json!({
            "name": name,
            "arguments": arguments,
        });
        let resp = self
            .transport
            .send(self.next_id(), "tools/call", Some(params))
            .await
            .map_err(|e| e.to_string())?;
        extract_result(resp).map_err(|e| e.to_string())
    }

    fn set_error(&mut self, err: McpTransportError) {
        match &err {
            McpTransportError::Unauthorized => {
                self.status = McpConnectionStatus::Unauthorized;
            }
            _ => {
                self.status = McpConnectionStatus::Error;
            }
        }
        self.last_error = Some(err.to_string());
    }
}

fn extract_result(resp: JsonRpcResponse) -> Result<Value, McpTransportError> {
    if let Some(err) = resp.error {
        return Err(McpTransportError::ServerError(
            err.code as u16,
            err.message,
        ));
    }
    resp.result.ok_or_else(|| {
        McpTransportError::Parse(serde_json::from_str::<Value>("null").unwrap_err())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_starts_disconnected() {
        let c = McpClient::new("https://project.feishu.cn", "tok");
        assert_eq!(c.status, McpConnectionStatus::Disconnected);
        assert_eq!(c.catalog.tools.len(), 0);
    }

    #[test]
    fn disconnect_resets_state() {
        let mut c = McpClient::new("https://project.feishu.cn", "tok");
        c.status = McpConnectionStatus::Connected;
        c.last_error = Some("old error".into());
        c.disconnect();
        assert_eq!(c.status, McpConnectionStatus::Disconnected);
        assert!(c.last_error.is_none());
    }

    #[test]
    fn next_id_increments() {
        let c = McpClient::new("https://x.com", "tok");
        assert_eq!(c.next_id(), 1);
        assert_eq!(c.next_id(), 2);
        assert_eq!(c.next_id(), 3);
    }

    #[test]
    fn extract_result_returns_result_value() {
        let resp = JsonRpcResponse {
            id: Some(1),
            result: Some(serde_json::json!({"tools": []})),
            error: None,
        };
        let val = extract_result(resp).unwrap();
        assert!(val.get("tools").is_some());
    }

    #[test]
    fn extract_result_returns_error_on_rpc_error() {
        let resp = JsonRpcResponse {
            id: Some(1),
            result: None,
            error: Some(super::super::mcp_http::JsonRpcError {
                code: -32600,
                message: "bad".into(),
                data: None,
            }),
        };
        let err = extract_result(resp).unwrap_err();
        assert!(err.to_string().contains("bad"));
    }
}
