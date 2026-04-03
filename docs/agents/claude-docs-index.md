# Claude 文档索引与状态

> **当前采用方案：** Dimweave 的 Claude 主链路已经统一为 `--sdk-url`。
>
> **当前运行时形态：** `claude --print --sdk-url ... --input-format stream-json --output-format stream-json` + workspace `.mcp.json` / inline `--strict-mcp-config` 注入 `agentnexus` MCP server。  
> Claude 主消息链路走 `WS /claude` + `POST /claude/events`，bridge sidecar 继续保留用于 `reply(to, text, status)` 和 `get_online_agents()`。

## 状态标签

| 标签 | 含义 |
|------|------|
| `Current` | 当前实际采用、应按此实现理解 |
| `Active reference` | 当前实现依赖的协议/验证/对照资料 |
| `Alternatives` | 保留的备选方案分析，不代表当前实现 |
| `Legacy` | 旧链路或旧协议参考，只用于历史追溯/兼容理解 |
| `Archive` | 已完成或已废弃的计划、实验记录 |

## 建议阅读顺序

1. [CLAUDE.md](/Users/jason/floder/agent-bridge/CLAUDE.md)
2. [claude-sdk-url-validation.md](/Users/jason/floder/agent-bridge/docs/agents/claude-sdk-url-validation.md)
3. [channel-vs-sdk-url-diff.md](/Users/jason/floder/agent-bridge/docs/channel-vs-sdk-url-diff.md)
4. [claude-sdk-url-protocol-deep-dive.md](/Users/jason/floder/agent-bridge/docs/agents/claude-sdk-url-protocol-deep-dive.md)

## 文档地图

| 文档 | 状态 | 用途 |
|------|------|------|
| [CLAUDE.md](/Users/jason/floder/agent-bridge/CLAUDE.md) | `Current` | 仓库级 Source of Truth。描述当前 Claude/Codex 架构、运行链路、约束与文档边界。 |
| [docs/agents/claude-sdk-url-validation.md](/Users/jason/floder/agent-bridge/docs/agents/claude-sdk-url-validation.md) | `Current` | 当前 Claude 主链路的协议验证和运行时收口结论。 |
| [docs/channel-vs-sdk-url-diff.md](/Users/jason/floder/agent-bridge/docs/channel-vs-sdk-url-diff.md) | `Active reference` | 对比旧 channel 链路与当前 `--sdk-url` 链路，帮助判断哪些部件迁移了、哪些保留。 |
| [docs/agents/claude-sdk-url-protocol-deep-dive.md](/Users/jason/floder/agent-bridge/docs/agents/claude-sdk-url-protocol-deep-dive.md) | `Active reference` | 逆向出来的 `--sdk-url` 协议细节。用于 host transport、control_request、result 语义核对。 |
| [docs/claude-code-integration-alternatives.md](/Users/jason/floder/agent-bridge/docs/claude-code-integration-alternatives.md) | `Alternatives` | Claude Agent SDK、CLI stdio、Channel、Anthropic API 等路线的系统性比较。保留方案推演，但当前未采用。 |
| [docs/agents/claude-cli-reverse-engineering.md](/Users/jason/floder/agent-bridge/docs/agents/claude-cli-reverse-engineering.md) | `Active reference` | Claude CLI 隐藏参数、prompt 注入位点、`--sdk-url` / `--system-prompt` / `--append-system-prompt` 的逆向记录。 |
| [docs/agents/claude-channel-api.md](/Users/jason/floder/agent-bridge/docs/agents/claude-channel-api.md) | `Legacy` | 官方 channel contract 参考。当前主链路不再使用 channel transport，但保留作旧链路/兼容概念参考。 |
| [docs/agents/claude-chain.md](/Users/jason/floder/agent-bridge/docs/agents/claude-chain.md) | `Legacy` | Claude 修复历史总账。包含大量 PTY/channel 时代记录，也包含后续 SDK 迁移节点；不应单独当作现状说明。 |
| [docs/superpowers/plans/2026-03-27-claude-stream-json.md](/Users/jason/floder/agent-bridge/docs/superpowers/plans/2026-03-27-claude-stream-json.md) | `Archive` | 早期 stream-json 方案实验计划，已被更准确的 `--sdk-url` 路线取代。 |
| [docs/superpowers/plans/2026-04-02-claude-sdk-full-regression.md](/Users/jason/floder/agent-bridge/docs/superpowers/plans/2026-04-02-claude-sdk-full-regression.md) | `Archive` | 已执行的 Claude 全量 `--sdk-url` 回归计划。 |

## 当前方案一句话定义

当前 Claude 不是走 “channel 通知驱动的 PTY 会话”，而是走：

`Claude CLI child` → `WS /claude + POST /claude/events` → `daemon`  
同时保留 `agentnexus` MCP bridge，为 Claude 提供：

- `reply(to, text, status)`
- `get_online_agents()`

也就是说，**当前采用的是 `--sdk-url` transport + MCP tools bridge 的混合方案**，而不是“完全移除 bridge”的纯 SDK 直连形态。

## 阅读时的判断规则

- 看“当前怎么跑”，优先以 [CLAUDE.md](/Users/jason/floder/agent-bridge/CLAUDE.md) 和 [claude-sdk-url-validation.md](/Users/jason/floder/agent-bridge/docs/agents/claude-sdk-url-validation.md) 为准。
- 看 `--sdk-url` wire format、事件字段、control_request 细节，以 [claude-sdk-url-protocol-deep-dive.md](/Users/jason/floder/agent-bridge/docs/agents/claude-sdk-url-protocol-deep-dive.md) 为准。
- 看为什么没有选 Agent SDK / stdio / channel，以 [claude-code-integration-alternatives.md](/Users/jason/floder/agent-bridge/docs/claude-code-integration-alternatives.md) 为准。
- 看旧 channel contract 或历史 bug 演化，只把 [claude-channel-api.md](/Users/jason/floder/agent-bridge/docs/agents/claude-channel-api.md) 和 [claude-chain.md](/Users/jason/floder/agent-bridge/docs/agents/claude-chain.md) 当背景资料，不当现状。
