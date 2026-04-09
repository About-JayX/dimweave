//! MCP tool catalog — parsed from `tools/list` response.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A single MCP tool discovered from the remote server.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpToolInfo {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub input_schema: Option<Value>,
}

/// Parsed tool catalog from a `tools/list` response.
#[derive(Debug, Clone, Default)]
pub struct McpToolCatalog {
    pub tools: Vec<McpToolInfo>,
}

impl McpToolCatalog {
    /// Parse from the JSON-RPC result value of `tools/list`.
    pub fn from_tools_list_result(result: &Value) -> Self {
        let tools = result
            .get("tools")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| serde_json::from_value::<McpToolInfo>(v.clone()).ok())
                    .collect()
            })
            .unwrap_or_default();
        Self { tools }
    }

    pub fn tool_count(&self) -> usize {
        self.tools.len()
    }

    pub fn tool_names(&self) -> Vec<&str> {
        self.tools.iter().map(|t| t.name.as_str()).collect()
    }

    pub fn find_tool(&self, name: &str) -> Option<&McpToolInfo> {
        self.tools.iter().find(|t| t.name == name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_empty_catalog() {
        let result = serde_json::json!({"tools": []});
        let cat = McpToolCatalog::from_tools_list_result(&result);
        assert_eq!(cat.tool_count(), 0);
    }

    #[test]
    fn parse_catalog_with_tools() {
        let result = serde_json::json!({
            "tools": [
                {
                    "name": "search_work_items",
                    "description": "Search work items in a space",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "space_id": {"type": "string"}
                        }
                    }
                },
                {
                    "name": "get_work_item_detail",
                    "description": "Get work item by ID"
                }
            ]
        });
        let cat = McpToolCatalog::from_tools_list_result(&result);
        assert_eq!(cat.tool_count(), 2);
        assert_eq!(cat.tool_names(), vec!["search_work_items", "get_work_item_detail"]);
        let tool = cat.find_tool("search_work_items").unwrap();
        assert!(tool.input_schema.is_some());
    }

    #[test]
    fn find_tool_returns_none_for_missing() {
        let cat = McpToolCatalog::default();
        assert!(cat.find_tool("nonexistent").is_none());
    }

    #[test]
    fn malformed_tools_entry_skipped() {
        let result = serde_json::json!({
            "tools": [
                {"name": "valid_tool"},
                "not_an_object",
                42
            ]
        });
        let cat = McpToolCatalog::from_tools_list_result(&result);
        assert_eq!(cat.tool_count(), 1);
        assert_eq!(cat.tool_names(), vec!["valid_tool"]);
    }
}
