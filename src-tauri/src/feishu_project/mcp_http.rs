//! Direct HTTP MCP transport for Feishu Project.
//!
//! Sends JSON-RPC 2.0 requests to `{domain}/mcp_server/v1`.
//! The `X-Meego-MCP-Connection-Type` header uses value `"stdio"` — this is
//! the value the official npm proxy sends and the remote server recognises.
//! We reuse it intentionally even though our transport is direct HTTP.

use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;

const MCP_SERVER_PATH: &str = "/mcp_server/v1";
const HEADER_MCP_TOKEN: &str = "X-Mcp-Token";
const HEADER_CONNECTION_TYPE: &str = "X-Meego-MCP-Connection-Type";
const CONNECTION_TYPE_VALUE: &str = "stdio";

#[derive(Debug, Clone)]
pub struct McpHttpTransport {
    client: Client,
    endpoint: String,
    token: String,
}

#[derive(Debug, Serialize)]
struct JsonRpcRequest {
    jsonrpc: &'static str,
    id: u64,
    method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<Value>,
}

#[derive(Debug, Deserialize)]
pub struct JsonRpcResponse {
    pub id: Option<u64>,
    pub result: Option<Value>,
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Deserialize)]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
    #[serde(default)]
    pub data: Option<Value>,
}

impl std::fmt::Display for JsonRpcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "JSON-RPC error {}: {}", self.code, self.message)
    }
}

impl McpHttpTransport {
    pub fn new(domain: &str, token: &str) -> Self {
        let domain = domain.trim_end_matches('/');
        Self {
            client: Client::new(),
            endpoint: format!("{domain}{MCP_SERVER_PATH}"),
            token: token.to_string(),
        }
    }

    pub async fn send(
        &self,
        id: u64,
        method: &str,
        params: Option<Value>,
    ) -> Result<JsonRpcResponse, McpTransportError> {
        let req = JsonRpcRequest {
            jsonrpc: "2.0",
            id,
            method: method.into(),
            params,
        };
        let resp = self
            .client
            .post(&self.endpoint)
            .header(HEADER_MCP_TOKEN, &self.token)
            .header(HEADER_CONNECTION_TYPE, CONNECTION_TYPE_VALUE)
            .header("Content-Type", "application/json")
            .json(&req)
            .send()
            .await
            .map_err(McpTransportError::Http)?;
        let status = resp.status();
        let body = resp.text().await.map_err(McpTransportError::Http)?;
        if status == reqwest::StatusCode::UNAUTHORIZED
            || body.contains("\"unauthorized\"")
        {
            return Err(McpTransportError::Unauthorized);
        }
        if !status.is_success() {
            return Err(McpTransportError::ServerError(status.as_u16(), body));
        }
        serde_json::from_str(&body).map_err(McpTransportError::Parse)
    }
}

#[derive(Debug)]
pub enum McpTransportError {
    Http(reqwest::Error),
    Unauthorized,
    ServerError(u16, String),
    Parse(serde_json::Error),
}

impl std::fmt::Display for McpTransportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Http(e) => write!(f, "HTTP error: {e}"),
            Self::Unauthorized => write!(f, "unauthorized (invalid MCP token)"),
            Self::ServerError(code, body) => write!(f, "server error {code}: {body}"),
            Self::Parse(e) => write!(f, "response parse error: {e}"),
        }
    }
}

impl std::error::Error for McpTransportError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transport_builds_correct_endpoint() {
        let t = McpHttpTransport::new("https://project.feishu.cn", "tok");
        assert_eq!(t.endpoint, "https://project.feishu.cn/mcp_server/v1");
    }

    #[test]
    fn transport_strips_trailing_slash() {
        let t = McpHttpTransport::new("https://project.feishu.cn/", "tok");
        assert_eq!(t.endpoint, "https://project.feishu.cn/mcp_server/v1");
    }

    #[test]
    fn json_rpc_request_serializes() {
        let req = JsonRpcRequest {
            jsonrpc: "2.0",
            id: 1,
            method: "initialize".into(),
            params: Some(serde_json::json!({"protocolVersion": "2024-11-05"})),
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"jsonrpc\":\"2.0\""));
        assert!(json.contains("\"method\":\"initialize\""));
    }

    #[test]
    fn json_rpc_response_parses_result() {
        let body = r#"{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"2024-11-05"}}"#;
        let resp: JsonRpcResponse = serde_json::from_str(body).unwrap();
        assert_eq!(resp.id, Some(1));
        assert!(resp.result.is_some());
        assert!(resp.error.is_none());
    }

    #[test]
    fn json_rpc_response_parses_error() {
        let body = r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32600,"message":"bad request"}}"#;
        let resp: JsonRpcResponse = serde_json::from_str(body).unwrap();
        let err = resp.error.unwrap();
        assert_eq!(err.code, -32600);
        assert_eq!(err.message, "bad request");
    }
}
