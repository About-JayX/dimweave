Codex App Server
通过 app-server 协议将 Codex 嵌入到你的产品中

Codex app-server 是 Codex 用来驱动富客户端的接口（例如 Codex VS Code 扩展）。当你希望在自己的产品中进行深度集成时，可以使用它：身份验证、会话历史、审批，以及流式代理事件。app-server 的实现已在 Codex GitHub 仓库（openai/codex/codex-rs/app-server）中开源。完整的开源 Codex 组件列表请参阅 Open Source 页面。

如果你是在自动化任务或在 CI 中运行 Codex，请改用 Codex SDK。

协议

与 MCP 类似，codex app-server 支持使用 JSON-RPC 2.0 消息进行双向通信（线上传输时省略 "jsonrpc":"2.0" 头）。

支持的传输方式：

stdio（--listen stdio://，默认）：使用换行分隔的 JSON（JSONL）。
websocket（--listen ws://IP:PORT，实验性）：每个 WebSocket 文本帧承载一条 JSON-RPC 消息。
在 WebSocket 模式下，app-server 使用有界队列。当请求入口已满时，服务器会使用 JSON-RPC 错误码 -32001 和消息 "Server overloaded; retry later." 拒绝新请求。客户端应使用指数递增延迟并加入抖动后重试。

消息结构

请求包含 method、params 和 id：

{ "method": "thread/start", "id": 10, "params": { "model": "gpt-5.4" } }

响应会回显 id，并包含 result 或 error：

{ "id": 10, "result": { "thread": { "id": "thr_123" } } }

{ "id": 10, "error": { "code": 123, "message": "Something went wrong" } }

通知不包含 id，只使用 method 和 params：

{ "method": "turn/started", "params": { "turn": { "id": "turn_456" } } }

你可以通过 CLI 生成 TypeScript schema 或 JSON Schema bundle。每种输出都对应你运行时的 Codex 版本，因此生成的产物会与该版本精确匹配：

codex app-server generate-ts --out ./schemas
codex app-server generate-json-schema --out ./schemas

快速开始

使用 codex app-server 启动服务器（默认 stdio 传输），或者使用 codex app-server --listen ws://127.0.0.1:4500 启动（实验性 WebSocket 传输）。
通过所选传输方式连接客户端，然后先发送 initialize，再发送 initialized 通知。
启动一个 thread 和一个 turn，然后持续从当前传输流中读取通知。
示例（Node.js / TypeScript）：

import { spawn } from "node:child_process";
import readline from "node:readline";

const proc = spawn("codex", ["app-server"], {
  stdio: ["pipe", "pipe", "inherit"],
});
const rl = readline.createInterface({ input: proc.stdout });

const send = (message: unknown) => {
  proc.stdin.write(`${JSON.stringify(message)}\n`);
};

let threadId: string | null = null;

rl.on("line", (line) => {
  const msg = JSON.parse(line) as any;
  console.log("server:", msg);

  if (msg.id === 1 && msg.result?.thread?.id && !threadId) {
    threadId = msg.result.thread.id;
    send({
      method: "turn/start",
      id: 2,
      params: {
        threadId,
        input: [{ type: "text", text: "Summarize this repo." }],
      },
    });
  }
});

send({
  method: "initialize",
  id: 0,
  params: {
    clientInfo: {
      name: "my_product",
      title: "My Product",
      version: "0.1.0",
    },
  },
});
send({ method: "initialized", params: {} });
send({ method: "thread/start", id: 1, params: { model: "gpt-5.4" } });

核心原语

线程（Thread）：用户与 Codex 代理之间的一段对话。线程包含多个 turn。
轮次（Turn）：一次用户请求，以及随后代理执行的工作。turn 包含多个 item，并流式输出增量更新。
条目（Item）：输入或输出的一个单元（用户消息、代理消息、命令执行、文件变更、工具调用等等）。
使用线程 API 来创建、列出或归档会话。使用 turn API 来驱动对话，并通过 turn 通知流式获取进度。

生命周期概览

每个连接只初始化一次：在打开传输连接后，立即发送带有客户端元数据的 initialize 请求，然后发出 initialized。服务器会拒绝该连接在此握手之前发送的任何请求。
启动（或恢复）线程：调用 thread/start 创建新会话，调用 thread/resume 继续现有会话，或调用 thread/fork 将历史分叉到一个新的 thread id。
开始一个 turn：调用 turn/start，提供目标 threadId 和用户输入。可选字段可覆盖 model、personality、cwd、sandbox policy 等。
引导一个活跃中的 turn：调用 turn/steer 将用户输入追加到当前正在进行的 turn，而不是创建新 turn。
流式读取事件：在 turn/start 之后，持续从 stdout 读取通知：thread/archived、thread/unarchived、item/started、item/completed、item/agentMessage/delta、工具进度以及其他更新。
结束该 turn：当模型完成时，或在执行 turn/interrupt 取消后，服务器会发出包含最终状态的 turn/completed。

初始化

客户端在某个传输连接上调用任何其他方法之前，必须先发送一次 initialize 请求，然后再以 initialized 通知进行确认。在初始化之前发送的请求会收到 Not initialized 错误；在同一连接上重复调用 initialize 会返回 Already initialized。

服务器会返回其向上游服务呈现的 user agent 字符串，以及描述运行目标平台的 platformFamily 和 platformOs 值。请设置 clientInfo 来标识你的集成。

initialize.params.capabilities 还支持按连接选择性关闭通知，可通过 optOutNotificationMethods 指定要在该连接上抑制的精确方法名列表。匹配为精确匹配（不支持通配符或前缀）。未知的方法名会被接受并忽略。

重要：请使用 clientInfo.name 来在 OpenAI Compliance Logs Platform 中标识你的客户端。如果你正在开发一个面向企业使用的新 Codex 集成，请联系 OpenAI，将其加入已知客户端列表。更多背景信息请参阅 Codex logs reference。

示例（来自 Codex VS Code 扩展）：

{
  "method": "initialize",
  "id": 0,
  "params": {
    "clientInfo": {
      "name": "codex_vscode",
      "title": "Codex VS Code Extension",
      "version": "0.1.0"
    }
  }
}

带通知退出配置的示例：

{
  "method": "initialize",
  "id": 1,
  "params": {
    "clientInfo": {
      "name": "my_client",
      "title": "My Client",
      "version": "0.1.0"
    },
    "capabilities": {
      "experimentalApi": true,
      "optOutNotificationMethods": ["thread/started", "item/agentMessage/delta"]
    }
  }
}

实验性 API 选择加入

某些 app-server 方法和字段被有意置于 experimentalApi capability 的保护之下。

省略 capabilities（或将 experimentalApi 设为 false），即可停留在稳定 API 范围内，服务器会拒绝实验性方法或字段。
将 capabilities.experimentalApi 设为 true，即可启用实验性方法和字段。
{
  "method": "initialize",
  "id": 1,
  "params": {
    "clientInfo": {
      "name": "my_client",
      "title": "My Client",
      "version": "0.1.0"
    },
    "capabilities": {
      "experimentalApi": true
    }
  }
}

如果客户端在未选择加入的情况下发送实验性方法或字段，app-server 会以如下错误拒绝：

<descriptor> requires experimentalApi capability

API 概览

thread/start - 创建一个新线程；会发出 thread/started，并自动将你订阅到该线程的 turn/item 事件。
thread/resume - 根据 id 重新打开一个已有线程，以便后续 turn/start 调用追加到该线程。
thread/fork - 通过复制已存储历史，将一个线程分叉为新的 thread id；会为新线程发出 thread/started。
thread/read - 按 id 读取已存储线程，但不恢复它；设置 includeTurns 可返回完整的 turn 历史。返回的 thread 对象包含运行时状态。
thread/list - 以分页方式遍历已存储线程日志；支持基于游标的分页，以及 modelProviders、sourceKinds、archived 和 cwd 过滤。返回的 thread 对象包含运行时状态。
thread/loaded/list - 列出当前已加载到内存中的线程 id。
thread/name/set - 为已加载线程或已持久化 rollout 设置或更新面向用户的线程名称；会发出 thread/name/updated。
thread/archive - 将线程日志文件移动到 archived 目录；成功时返回 {}，并发出 thread/archived。
thread/unsubscribe - 取消当前连接对线程 turn/item 事件的订阅。如果这是最后一个订阅者，服务器会卸载该线程并发出 thread/closed。
thread/unarchive - 将已归档的线程 rollout 恢复回 active sessions 目录；返回恢复后的线程，并发出 thread/unarchived。
thread/status/changed - 当某个已加载线程的运行时状态发生变化时发出的通知。
thread/compact/start - 触发某个线程的会话历史压缩；立即返回 {}，实际进度通过 turn/* 和 item/* 通知流式发送。
thread/rollback - 从内存上下文中删除最后 N 个 turn，并持久化一个回滚标记；返回更新后的线程。
turn/start - 向线程添加用户输入并开始 Codex 生成；响应中返回初始 turn，并流式发送事件。对于 collaborationMode，settings.developer_instructions: null 的含义是“对所选模式使用内置指令”。
turn/steer - 向某个线程当前正在进行的活跃 turn 追加用户输入；返回已接受的 turnId。
turn/interrupt - 请求取消一个正在进行中的 turn；成功时返回 {}，且 turn 将以 status: "interrupted" 结束。
review/start - 为线程启动 Codex reviewer；会发出 enteredReviewMode 和 exitedReviewMode item。
command/exec - 在不启动 thread/turn 的情况下，在服务器沙箱内运行单个命令。
command/exec/write - 向正在运行的 command/exec 会话写入 stdin 字节，或关闭 stdin。
command/exec/resize - 调整一个基于 PTY 的 command/exec 会话大小。
command/exec/terminate - 终止一个正在运行的 command/exec 会话。
model/list - 列出可用模型（设置 includeHidden: true 可以包含 hidden: true 的条目），以及 effort 选项、可选升级项和 inputModalities。
experimentalFeature/list - 列出功能开关及其生命周期阶段元数据，并支持游标分页。
collaborationMode/list - 列出协作模式预设（实验性，不分页）。
skills/list - 针对一个或多个 cwd 列出技能（支持 forceReload 和可选的 perCwdExtraUserRoots）。
plugin/list - 列出已发现的插件市场及插件状态，包括安装/鉴权策略元数据。
plugin/read - 根据 marketplace path 和 plugin name 读取单个插件，包括其打包的技能、应用和 MCP server 名称。
app/list - 列出可用应用（连接器），包含分页以及 accessibility/enabled 元数据。
skills/config/write - 按路径启用或禁用技能。
mcpServer/oauth/login - 为已配置的 MCP server 启动 OAuth 登录；返回授权 URL，并在完成时发出 mcpServer/oauthLogin/completed。
tool/requestUserInput - 在工具调用中提示用户回答 1 到 3 个简短问题（实验性）；问题选项可设置 isOther 以支持自由输入。
config/mcpServer/reload - 从磁盘重新加载 MCP server 配置，并为已加载线程排队刷新。
mcpServerStatus/list - 列出 MCP server、工具、资源以及认证状态（基于 cursor + limit 分页）。
windowsSandbox/setupStart - 启动 Windows 沙箱设置，可选择提权或非提权模式；该调用快速返回，并稍后发出 windowsSandbox/setupCompleted。
feedback/upload - 提交一份反馈报告（分类 + 可选 reason/logs + conversation id，也可附带额外 extraLogFiles 附件）。
config/read - 在解析配置分层后，获取磁盘上的最终生效配置。
externalAgentConfig/detect - 使用 includeHome 和可选 cwds 检测可迁移的 external-agent 制品；每个检测项都包含 cwd（home 级项为 null）。
externalAgentConfig/import - 通过传入带 cwd（home 级为 null）的显式 migrationItems，应用选定的 external-agent 迁移项。
config/value/write - 向用户磁盘上的 config.toml 写入一个单独的配置键值。
config/batchWrite - 以原子方式对用户磁盘上的 config.toml 应用一组配置修改。
configRequirements/read - 从 requirements.toml 和/或 MDM 获取需求，包括 allow-list、固定的 featureRequirements，以及地域/网络需求（如果尚未配置则返回 null）。
fs/readFile、fs/writeFile、fs/createDirectory、fs/getMetadata、fs/readDirectory、fs/remove 和 fs/copy - 通过 app-server v2 文件系统 API 操作绝对文件系统路径。

模型

列出模型（model/list）

在渲染模型或 personality 选择器之前，调用 model/list 来发现可用模型及其能力。

{ "method": "model/list", "id": 6, "params": { "limit": 20, "includeHidden": false } }
{ "id": 6, "result": {
  "data": [{
    "id": "gpt-5.4",
    "model": "gpt-5.4",
    "displayName": "GPT-5.4",
    "hidden": false,
    "defaultReasoningEffort": "medium",
    "supportedReasoningEfforts": [{
      "reasoningEffort": "low",
      "description": "Lower latency"
    }],
    "inputModalities": ["text", "image"],
    "supportsPersonality": true,
    "isDefault": true
  }],
  "nextCursor": null
} }

每个模型条目可以包含：

supportedReasoningEfforts - 该模型支持的 effort 选项。
defaultReasoningEffort - 面向客户端建议的默认 effort。
upgrade - 可选的推荐升级模型 id，用于客户端中的迁移提示。
upgradeInfo - 可选的升级元数据，用于客户端中的迁移提示。
hidden - 该模型是否在默认选择器列表中隐藏。
inputModalities - 模型支持的输入类型（例如 text、image）。
supportsPersonality - 该模型是否支持类似 /personality 这样的个性化指令。
isDefault - 该模型是否为推荐默认值。
默认情况下，model/list 只返回选择器可见的模型。如果你需要完整列表并希望在客户端侧根据 hidden 进行过滤，请设置 includeHidden: true。

当 inputModalities 缺失时（旧版模型目录），为了向后兼容，应将其视为 ["text", "image"]。

列出实验性功能（experimentalFeature/list）

使用此端点发现带元数据和生命周期阶段的功能开关：

{ "method": "experimentalFeature/list", "id": 7, "params": { "limit": 20 } }
{ "id": 7, "result": {
  "data": [{
    "name": "unified_exec",
    "stage": "beta",
    "displayName": "Unified exec",
    "description": "Use the unified PTY-backed execution tool.",
    "announcement": "Beta rollout for improved command execution reliability.",
    "enabled": false,
    "defaultEnabled": false
  }],
  "nextCursor": null
} }

stage 可以是 beta、underDevelopment、stable、deprecated 或 removed。对于非 beta 的功能开关，displayName、description 和 announcement 可能为 null。

线程

thread/read 会读取已存储线程，但不会订阅它；设置 includeTurns 可以把 turns 一并包含进来。
thread/list 支持游标分页，以及 modelProviders、sourceKinds、archived 和 cwd 过滤。
thread/loaded/list 返回当前内存中的线程 ID。
thread/archive 会把线程持久化 JSONL 日志移动到 archived 目录。
thread/unsubscribe 会取消当前连接对某个已加载线程的订阅，并且可能触发 thread/closed。
thread/unarchive 会把已归档线程 rollout 恢复回 active sessions 目录。
thread/compact/start 会触发压缩并立即返回 {}。
thread/rollback 会从内存上下文中删除最后 N 个 turns，并在该线程持久化的 JSONL 日志中记录回滚标记。
启动或恢复线程

当你需要一段新的 Codex 对话时，启动一个全新的线程。

{ "method": "thread/start", "id": 10, "params": {
  "model": "gpt-5.4",
  "cwd": "/Users/me/project",
  "approvalPolicy": "never",
  "sandbox": "workspaceWrite",
  "personality": "friendly",
  "serviceName": "my_app_server_client"
} }
{ "id": 10, "result": {
  "thread": {
    "id": "thr_123",
    "preview": "",
    "ephemeral": false,
    "modelProvider": "openai",
    "createdAt": 1730910000
  }
} }
{ "method": "thread/started", "params": { "thread": { "id": "thr_123" } } }

serviceName 是可选项。当你希望 app-server 用你的集成服务名对线程级指标打标时，可以设置它。

源码补充：`baseInstructions` / `developerInstructions`

官方 app-server 页面当前没有把这两个字段单独展开写在 `thread/start` 示例里，但从公开源码可以确认，它们确实属于 `ThreadStartParams` 的一部分。

在 `codex-app-server-protocol` 的 `ThreadStartParams` 结构体中，存在以下字段：

```rust
pub struct ThreadStartParams {
    pub base_instructions: Option<String>,
    pub developer_instructions: Option<String>,
}
```

由于该结构体使用了 `#[serde(rename_all = "camelCase")]`，因此在线上传输时，对应 JSON 字段名分别是：

- `baseInstructions`
- `developerInstructions`

从 `codex-rs/core/src/codex.rs` 的优先级逻辑可以确认，`baseInstructions` 对应的 `base_instructions` 会按如下顺序参与解析：

1. `thread/start` 传入的 `baseInstructions`
2. 已恢复会话历史中保存的 `base_instructions`
3. 当前模型的默认内置 instructions

这意味着 `baseInstructions` 会覆盖 Codex 当前回合使用的 base instructions 层。

进一步看 `codex-rs/core/src/client.rs`，Codex 在构造 `ResponsesApiRequest` 时，会把 `prompt.base_instructions.text` 写入请求的 `instructions` 字段。因此：

- `baseInstructions` 会映射到 OpenAI Responses API 的 `instructions`
- `developerInstructions` 的语义则不同，它用于补充一条 `developer` 角色消息，而不是替换 `instructions`

可以把两者理解为：

| 字段 | 作用层 | 典型语义 |
| --- | --- | --- |
| `baseInstructions` | Base instructions / `instructions` | 替换 Codex 本次使用的基础指令层 |
| `developerInstructions` | `developer` role message | 在对话输入中追加开发者指令 |

说明：这里更严谨的说法是“替换 Codex 传入 Responses API 的基础 instructions 层”，而不是“替换 OpenAI 服务端保留的全部 system prompt”。

参考来源：

- <https://docs.rs/codex-app-server-protocol/latest/codex_app_server_protocol/struct.ThreadStartParams.html>
- <https://docs.rs/codex-app-server-protocol/latest/src/codex_app_server_protocol/protocol/v2.rs.html>
- <https://github.com/openai/codex/blob/main/codex-rs/core/src/codex.rs>
- <https://github.com/openai/codex/blob/main/codex-rs/core/src/client.rs>

如果要继续某个已存储会话，请使用你之前记录下来的 thread.id 调用 thread/resume。其响应结构与 thread/start 一致。你也可以传入 thread/start 支持的同类配置覆盖项，例如 personality：

{ "method": "thread/resume", "id": 11, "params": {
  "threadId": "thr_123",
  "personality": "friendly"
} }
{ "id": 11, "result": { "thread": { "id": "thr_123", "name": "Bug bash notes", "ephemeral": false } } }

恢复某个线程本身不会更新 thread.updatedAt（或 rollout 文件的修改时间）。时间戳会在你启动一个 turn 时更新。

如果你在配置中把某个已启用的 MCP server 标记为 required，而该 server 初始化失败，那么 thread/start 和 thread/resume 会直接失败，而不是在缺少该 server 的情况下继续。

thread/start 上的 dynamicTools 是实验性字段（要求 capabilities.experimentalApi = true）。Codex 会把这些动态工具持久化到线程 rollout 元数据中，并在你未提供新的 dynamic tools 时，在 thread/resume 时恢复它们。

如果你恢复线程时使用的模型与 rollout 中记录的模型不同，Codex 会发出警告，并在下一次 turn 中应用一次性模型切换指令。

如果你要基于已存储会话创建分支，请使用该 thread.id 调用 thread/fork。这样会创建一个新的 thread id，并为它发出 thread/started 通知：

{ "method": "thread/fork", "id": 12, "params": { "threadId": "thr_123" } }
{ "id": 12, "result": { "thread": { "id": "thr_456" } } }
{ "method": "thread/started", "params": { "thread": { "id": "thr_456" } } }

当已经设置了面向用户的线程标题时，app-server 会在 thread/list、thread/read、thread/resume、thread/unarchive 和 thread/rollback 响应中填充 thread.name。thread/start 和 thread/fork 在标题稍后设置之前，可能省略 name（或返回 null）。

读取已存储线程（但不恢复）

当你想获取已存储的线程数据，但又不想恢复该线程或订阅其事件时，请使用 thread/read。

includeTurns - 为 true 时，响应会包含该线程的 turns；为 false 或省略时，只返回线程摘要。
返回的 thread 对象包含运行时状态（notLoaded、idle、systemError，或者带 activeFlags 的 active）。
{ "method": "thread/read", "id": 19, "params": { "threadId": "thr_123", "includeTurns": true } }
{ "id": 19, "result": { "thread": { "id": "thr_123", "name": "Bug bash notes", "ephemeral": false, "status": { "type": "notLoaded" }, "turns": [] } } }

不同于 thread/resume，thread/read 不会把线程加载到内存中，也不会发出 thread/started。

列出线程（带分页和过滤）

thread/list 可以让你渲染历史记录 UI。结果默认按 createdAt 从新到旧排序。过滤会先于分页执行。你可以组合传入以下任意参数：

cursor - 来自上一次响应的不透明字符串；首页时省略。
limit - 未设置时，服务器会使用一个合理的默认分页大小。
sortKey - created_at（默认）或 updated_at。
modelProviders - 将结果限制为特定 provider；未设置、null 或空数组表示包含所有 provider。
sourceKinds - 将结果限制为特定线程来源。省略或传 [] 时，服务器默认仅包含交互式来源：cli 和 vscode。
archived - 为 true 时仅列出已归档线程。为 false 或省略时，列出未归档线程（默认）。
cwd - 将结果限制为 session 当前工作目录与该路径完全一致的线程。
sourceKinds 接受以下值：

cli
vscode
exec
appServer
subAgent
subAgentReview
subAgentCompact
subAgentThreadSpawn
subAgentOther
unknown
示例：

{ "method": "thread/list", "id": 20, "params": {
  "cursor": null,
  "limit": 25,
  "sortKey": "created_at"
} }
{ "id": 20, "result": {
  "data": [
    { "id": "thr_a", "preview": "Create a TUI", "ephemeral": false, "modelProvider": "openai", "createdAt": 1730831111, "updatedAt": 1730831111, "name": "TUI prototype", "status": { "type": "notLoaded" } },
    { "id": "thr_b", "preview": "Fix tests", "ephemeral": true, "modelProvider": "openai", "createdAt": 1730750000, "updatedAt": 1730750000, "status": { "type": "notLoaded" } }
  ],
  "nextCursor": "opaque-token-or-null"
} }

当 nextCursor 为 null 时，表示你已经到达最后一页。

跟踪线程状态变化

每当某个已加载线程的运行时状态发生变化时，都会发出 thread/status/changed。其载荷包含 threadId 和新的状态。

{
  "method": "thread/status/changed",
  "params": {
    "threadId": "thr_123",
    "status": { "type": "active", "activeFlags": ["waitingOnApproval"] }
  }
}

列出已加载线程

thread/loaded/list 返回当前加载在内存中的线程 ID。

{ "method": "thread/loaded/list", "id": 21 }
{ "id": 21, "result": { "data": ["thr_123", "thr_456"] } }

取消订阅某个已加载线程

thread/unsubscribe 会移除当前连接对某个线程的订阅。响应中的 status 可能是以下之一：

unsubscribed：当前连接原本已订阅，现在已移除。
notSubscribed：当前连接并未订阅该线程。
notLoaded：该线程当前未加载。
如果这是最后一个订阅者，服务器会卸载该线程，并发出状态变为 notLoaded 的 thread/status/changed，随后再发出 thread/closed。

{ "method": "thread/unsubscribe", "id": 22, "params": { "threadId": "thr_123" } }
{ "id": 22, "result": { "status": "unsubscribed" } }
{ "method": "thread/status/changed", "params": {
    "threadId": "thr_123",
    "status": { "type": "notLoaded" }
} }
{ "method": "thread/closed", "params": { "threadId": "thr_123" } }

归档线程

使用 thread/archive 将线程的持久化日志（磁盘上的 JSONL 文件）移动到 archived sessions 目录。

{ "method": "thread/archive", "id": 22, "params": { "threadId": "thr_b" } }
{ "id": 22, "result": {} }
{ "method": "thread/archived", "params": { "threadId": "thr_b" } }

除非你传入 archived: true，否则已归档线程不会出现在后续的 thread/list 调用中。

取消归档线程

使用 thread/unarchive 将已归档的线程 rollout 移回 active sessions 目录。

{ "method": "thread/unarchive", "id": 24, "params": { "threadId": "thr_b" } }
{ "id": 24, "result": { "thread": { "id": "thr_b", "name": "Bug bash notes" } } }
{ "method": "thread/unarchived", "params": { "threadId": "thr_b" } }

触发线程压缩

使用 thread/compact/start 手动触发某个线程的历史压缩。该请求会立即返回 {}。

app-server 会在相同 threadId 上以标准 turn/* 和 item/* 通知的方式流式发送进度，其中包括 contextCompaction item 的生命周期（item/started 随后 item/completed）。

{ "method": "thread/compact/start", "id": 25, "params": { "threadId": "thr_b" } }
{ "id": 25, "result": {} }

回滚最近的 turns

使用 thread/rollback 从内存上下文中删除最后 numTurns 个条目，并在 rollout 日志中持久化一个回滚标记。返回的线程会包含回滚后的 turns。

{ "method": "thread/rollback", "id": 26, "params": { "threadId": "thr_b", "numTurns": 1 } }
{ "id": 26, "result": { "thread": { "id": "thr_b", "name": "Bug bash notes", "ephemeral": false } } }

Turns（轮次）

input 字段接受一个 item 列表：

{ "type": "text", "text": "Explain this diff" }
{ "type": "image", "url": "https://.../design.png" }
{ "type": "localImage", "path": "/tmp/screenshot.png" }
你可以为每个 turn 覆盖配置设置（model、effort、personality、cwd、sandbox policy、summary）。一旦指定，这些设置会成为同一线程后续 turns 的默认值。outputSchema 仅应用于当前 turn。对于 sandboxPolicy.type = "externalSandbox"，请将 networkAccess 设为 restricted 或 enabled；对于 workspaceWrite，networkAccess 仍是布尔值。

对于 turn/start.collaborationMode，settings.developer_instructions: null 的含义是“对所选模式使用内置指令”，而不是清空该模式的指令。

沙箱读取权限（ReadOnlyAccess）

sandboxPolicy 支持显式读取权限控制：

readOnly：可选 access（默认是 { "type": "fullAccess" }，也可以是受限根目录）。
workspaceWrite：可选 readOnlyAccess（默认是 { "type": "fullAccess" }，也可以是受限根目录）。
受限读取权限的结构如下：

{
  "type": "restricted",
  "includePlatformDefaults": true,
  "readableRoots": ["/Users/me/shared-read-only"]
}

在 macOS 上，includePlatformDefaults: true 会为受限只读会话附加一套精心挑选的平台默认 Seatbelt 策略。这样可以提升工具兼容性，而不会宽泛地开放整个 /System。

示例：

{ "type": "readOnly", "access": { "type": "fullAccess" } }

{
  "type": "workspaceWrite",
  "writableRoots": ["/Users/me/project"],
  "readOnlyAccess": {
    "type": "restricted",
    "includePlatformDefaults": true,
    "readableRoots": ["/Users/me/shared-read-only"]
  },
  "networkAccess": false
}

启动一个 turn

{ "method": "turn/start", "id": 30, "params": {
  "threadId": "thr_123",
  "input": [ { "type": "text", "text": "Run tests" } ],
  "cwd": "/Users/me/project",
  "approvalPolicy": "unlessTrusted",
  "sandboxPolicy": {
    "type": "workspaceWrite",
    "writableRoots": ["/Users/me/project"],
    "networkAccess": true
  },
  "model": "gpt-5.4",
  "effort": "medium",
  "summary": "concise",
  "personality": "friendly",
  "outputSchema": {
    "type": "object",
    "properties": { "answer": { "type": "string" } },
    "required": ["answer"],
    "additionalProperties": false
  }
} }
{ "id": 30, "result": { "turn": { "id": "turn_456", "status": "inProgress", "items": [], "error": null } } }

引导一个活跃中的 turn

使用 turn/steer 可以向当前正在进行的活跃 turn 追加更多用户输入。

请包含 expectedTurnId；它必须与当前活跃 turn 的 id 匹配。
如果该线程上没有活跃中的 turn，请求会失败。
turn/steer 不会发出新的 turn/started 通知。
turn/steer 不接受 turn 级覆盖项（model、cwd、sandboxPolicy 或 outputSchema）。
{ "method": "turn/steer", "id": 32, "params": {
  "threadId": "thr_123",
  "input": [ { "type": "text", "text": "Actually focus on failing tests first." } ],
  "expectedTurnId": "turn_456"
} }
{ "id": 32, "result": { "turnId": "turn_456" } }

启动一个 turn（调用技能）

若要显式调用某个技能，请在文本输入中包含 $<skill-name>，并同时附带一个 skill 输入项。

{ "method": "turn/start", "id": 33, "params": {
  "threadId": "thr_123",
  "input": [
    { "type": "text", "text": "$skill-creator Add a new skill for triaging flaky CI and include step-by-step usage." },
    { "type": "skill", "name": "skill-creator", "path": "/Users/me/.codex/skills/skill-creator/SKILL.md" }
  ]
} }
{ "id": 33, "result": { "turn": { "id": "turn_457", "status": "inProgress", "items": [], "error": null } } }

中断一个 turn

{ "method": "turn/interrupt", "id": 31, "params": { "threadId": "thr_123", "turnId": "turn_456" } }
{ "id": 31, "result": {} }

成功时，该 turn 会以 status: "interrupted" 结束。

Review（审查）

review/start 会为某个线程运行 Codex reviewer，并流式返回 review items。目标可以包括：

uncommittedChanges
baseBranch（与某个分支进行 diff）
commit（review 一个特定提交）
custom（自由形式指令）
使用 delivery: "inline"（默认）可以在现有线程中执行 review；使用 delivery: "detached" 则会 fork 出一个新的 review 线程。

请求/响应示例：

{ "method": "review/start", "id": 40, "params": {
  "threadId": "thr_123",
  "delivery": "inline",
  "target": { "type": "commit", "sha": "1234567deadbeef", "title": "Polish tui colors" }
} }
{ "id": 40, "result": {
  "turn": {
    "id": "turn_900",
    "status": "inProgress",
    "items": [
      { "type": "userMessage", "id": "turn_900", "content": [ { "type": "text", "text": "Review commit 1234567: Polish tui colors" } ] }
    ],
    "error": null
  },
  "reviewThreadId": "thr_123"
} }

如果是 detached review，请使用 "delivery": "detached"。响应结构相同，但 reviewThreadId 会是新 review 线程的 id（与原始 threadId 不同）。服务器也会在流式发送该 review turn 之前，为新线程发出一个 thread/started 通知。

Codex 会先流式发出常规的 turn/started 通知，然后发出一个携带 enteredReviewMode item 的 item/started：

{
  "method": "item/started",
  "params": {
    "item": {
      "type": "enteredReviewMode",
      "id": "turn_900",
      "review": "current changes"
    }
  }
}

当 reviewer 完成时，服务器会发出 item/started 和 item/completed，其中包含一个带最终 review 文本的 exitedReviewMode item：

{
  "method": "item/completed",
  "params": {
    "item": {
      "type": "exitedReviewMode",
      "id": "turn_900",
      "review": "Looks solid overall..."
    }
  }
}

请使用此通知在你的客户端中渲染 reviewer 输出。

命令执行

command/exec 会在不创建线程的情况下，在服务器沙箱中执行一个单独命令（argv 数组）。

{ "method": "command/exec", "id": 50, "params": {
  "command": ["ls", "-la"],
  "cwd": "/Users/me/project",
  "sandboxPolicy": { "type": "workspaceWrite" },
  "timeoutMs": 10000
} }
{ "id": 50, "result": { "exitCode": 0, "stdout": "...", "stderr": "" } }

如果你已经对服务器进程进行了沙箱隔离，并希望 Codex 跳过自身的沙箱强制措施，请使用 sandboxPolicy.type = "externalSandbox"。在 external sandbox 模式下，请将 networkAccess 设为 restricted（默认）或 enabled。对于 readOnly 和 workspaceWrite，请使用上文展示过的同一套可选 access / readOnlyAccess 结构。

说明：

服务器会拒绝空命令数组。
sandboxPolicy 接受与 turn/start 相同的结构（例如 dangerFullAccess、readOnly、workspaceWrite、externalSandbox）。
若省略 timeoutMs，则回退到服务器默认值。
当你计划后续使用 command/exec/write、command/exec/resize 或 command/exec/terminate 时，请设置 tty: true，并使用 processId。
将 streamStdoutStderr 设为 true 后，命令运行期间会收到 command/exec/outputDelta 通知。
读取管理员要求（configRequirements/read）

使用 configRequirements/read 检查从 requirements.toml 和/或 MDM 加载得到的最终生效管理员要求。

{ "method": "configRequirements/read", "id": 52, "params": {} }
{ "id": 52, "result": {
  "requirements": {
    "allowedApprovalPolicies": ["onRequest", "unlessTrusted"],
    "allowedSandboxModes": ["readOnly", "workspaceWrite"],
    "featureRequirements": {
      "personality": true,
      "unified_exec": false
    },
    "network": {
      "enabled": true,
      "allowedDomains": ["api.openai.com"],
      "allowUnixSockets": ["/tmp/example.sock"],
      "dangerouslyAllowAllUnixSockets": false
    }
  }
} }

当未配置任何 requirements 时，result.requirements 为 null。支持的键和值详情，请参阅 requirements.toml 的相关文档。

Windows 沙箱设置（windowsSandbox/setupStart）

自定义 Windows 客户端可以异步触发沙箱设置，而不是在启动检查时阻塞。

{ "method": "windowsSandbox/setupStart", "id": 53, "params": { "mode": "elevated" } }
{ "id": 53, "result": { "started": true } }

app-server 会在后台启动设置，并在稍后发出完成通知：

{
  "method": "windowsSandbox/setupCompleted",
  "params": { "mode": "elevated", "success": true, "error": null }
}

模式：

elevated - 运行提权的 Windows 沙箱设置路径。
unelevated - 运行旧版设置/预检路径。

事件

事件通知是由服务器主动发起的流，用于描述线程生命周期、turn 生命周期以及其中的各类 item。在你启动或恢复一个线程之后，请持续从当前传输流中读取 thread/started、thread/archived、thread/unarchived、thread/closed、thread/status/changed、turn/*、item/* 和 serverRequest/resolved 通知。

通知退出订阅

客户端可以通过在 initialize.params.capabilities.optOutNotificationMethods 中发送精确的方法名，来按连接抑制特定通知。

仅支持精确匹配：item/agentMessage/delta 只会抑制该方法本身。
未知方法名会被忽略。
适用于当前线程的 thread/*、turn/*、item/* 以及相关 v2 通知。
不适用于请求、响应或错误。
模糊文件搜索事件（实验性）

fuzzy file search 会话 API 会为每个查询发出通知：

fuzzyFileSearch/sessionUpdated - { sessionId, query, files }，包含当前活跃查询的匹配结果。
fuzzyFileSearch/sessionCompleted - { sessionId }，表示该查询的索引和匹配已经完成。
Windows 沙箱设置事件

windowsSandbox/setupCompleted - { mode, success, error }，在 windowsSandbox/setupStart 请求完成后发出。
Turn 事件

turn/started - { turn }，包含 turn id、空 items，以及 status: "inProgress"。
turn/completed - { turn }，其中 turn.status 为 completed、interrupted 或 failed；失败时会携带 { error: { message, codexErrorInfo?, additionalDetails? } }。
turn/diff/updated - { threadId, turnId, diff }，包含该 turn 中所有文件变更聚合后的最新 unified diff。
turn/plan/updated - { turnId, explanation?, plan }，每当代理共享或更改其计划时发出；每个 plan 项为 { step, status }，其中 status 取值为 pending、inProgress 或 completed。
thread/tokenUsage/updated - 当前活跃线程的用量更新。
即使 item 事件在流式发送，turn/diff/updated 和 turn/plan/updated 目前仍会包含空的 items 数组。请以 item/* 通知作为 turn items 的事实来源。

Items（条目）

ThreadItem 是 turn 响应和 item/* 通知中携带的带标签联合类型。常见 item 类型包括：

userMessage - {id, content}，其中 content 是用户输入列表（text、image 或 localImage）。
agentMessage - {id, text, phase?}，包含已累计的代理回复。存在 phase 时，其值使用 Responses API 的线上取值（commentary、final_answer）。
plan - {id, text}，包含 plan 模式下提出的计划文本。请以 item/completed 中最终的 plan item 为准。
reasoning - {id, summary, content}，其中 summary 保存流式推理摘要，content 保存原始推理块。
commandExecution - {id, command, cwd, status, commandActions, aggregatedOutput?, exitCode?, durationMs?}。
fileChange - {id, changes, status}，描述建议的编辑；changes 列表项为 {path, kind, diff}。
mcpToolCall - {id, server, tool, status, arguments, result?, error?}。
dynamicToolCall - {id, tool, arguments, status, contentItems?, success?, durationMs?}，用于客户端执行的动态工具调用。
collabToolCall - {id, tool, status, senderThreadId, receiverThreadId?, newThreadId?, prompt?, agentStatus?}。
webSearch - {id, query, action?}，用于代理发起的网页搜索请求。
imageView - {id, path}，在代理调用图片查看工具时发出。
enteredReviewMode - {id, review}，在 reviewer 开始时发送。
exitedReviewMode - {id, review}，在 reviewer 完成时发出。
contextCompaction - {id}，在 Codex 压缩会话历史时发出。
对于 webSearch.action，action 类型可以是 search（query?、queries?）、openPage（url?）或 findInPage（url?、pattern?）。

app server 已弃用旧版 thread/compacted 通知；请改用 contextCompaction item。

所有 item 都会发出两个共享的生命周期事件：

item/started - 当一个新的工作单元开始时，发出完整 item；item.id 与 deltas 中使用的 itemId 对应。
item/completed - 当工作完成时，发送最终 item；请将其视为权威状态。
Item 增量

item/agentMessage/delta - 为代理消息追加流式文本。
item/plan/delta - 流式发送计划文本。最终的 plan item 不一定与所有 delta 拼接后的结果完全一致。
item/reasoning/summaryTextDelta - 流式发送可读的推理摘要；当开启新的摘要分段时，summaryIndex 会递增。
item/reasoning/summaryPartAdded - 标记推理摘要分段之间的边界。
item/reasoning/textDelta - 流式发送原始推理文本（模型支持时）。
item/commandExecution/outputDelta - 流式发送某个命令的 stdout/stderr；请按顺序追加 delta。
item/fileChange/outputDelta - 包含底层 apply_patch 工具调用的工具响应。
错误

如果某个 turn 失败，服务器会发出错误事件 { error: { message, codexErrorInfo?, additionalDetails? } }，然后以 status: "failed" 结束该 turn。当上游 HTTP 状态码可用时，它会出现在 codexErrorInfo.httpStatusCode 中。

常见的 codexErrorInfo 值包括：

ContextWindowExceeded
UsageLimitExceeded
HttpConnectionFailed（上游 4xx/5xx 错误）
ResponseStreamConnectionFailed
ResponseStreamDisconnected
ResponseTooManyFailedAttempts
BadRequest、Unauthorized、SandboxError、InternalServerError、Other
当上游 HTTP 状态码可用时，服务器会在相应的 codexErrorInfo 变体中的 httpStatusCode 字段转发该状态码。

审批

根据用户的 Codex 设置，命令执行和文件变更可能需要审批。app-server 会向客户端发送一个由服务器主动发起的 JSON-RPC 请求，而客户端则返回一个决策载荷。

命令执行决策包括：accept、acceptForSession、decline、cancel，或 { "acceptWithExecpolicyAmendment": { "execpolicy_amendment": ["cmd", "..."] } }。

文件变更决策包括：accept、acceptForSession、decline、cancel。

请求中会包含 threadId 和 turnId，请使用它们将 UI 状态限定到当前活跃会话。

服务器会恢复或拒绝该工作，并通过 item/completed 结束相应 item。

命令执行审批

消息顺序如下：

item/started 会显示待处理的 commandExecution item，其中包含 command、cwd 以及其他字段。
item/commandExecution/requestApproval 包含 itemId、threadId、turnId、可选的 reason、可选的 command、可选的 cwd、可选的 commandActions、可选的 proposedExecpolicyAmendment、可选的 networkApprovalContext，以及可选的 availableDecisions。当 initialize.params.capabilities.experimentalApi = true 时，该载荷还可能包含实验性的 additionalPermissions，用于描述按命令请求的沙箱访问权限。additionalPermissions 中的任何文件系统路径在线上传输时都是绝对路径。
客户端以上述命令执行审批决策之一进行响应。
serverRequest/resolved 用来确认待处理请求已经得到响应或被清除。
item/completed 返回最终的 commandExecution item，其 status 为 completed | failed | declined。
当存在 networkApprovalContext 时，该提示针对的是受管网络访问，而不是普通的 shell 命令审批。当前 v2 schema 会暴露目标主机和协议；客户端应渲染网络专用提示，而不要依赖 command 作为对用户有意义的 shell 命令预览。

Codex 会按目标地址（host、protocol 和 port）对并发的网络审批提示进行分组。因此，app-server 可能发送一个提示，一次性解除多个指向同一目标地址的排队请求；而同一 host 上不同 port 则会被分别对待。

文件变更审批

消息顺序如下：

item/started 会发出一个 fileChange item，其中包含建议变更，并带有 status: "inProgress"。
item/fileChange/requestApproval 包含 itemId、threadId、turnId、可选的 reason 和可选的 grantRoot。
客户端以上述文件变更审批决策之一进行响应。
serverRequest/resolved 用来确认待处理请求已经得到响应或被清除。
item/completed 返回最终的 fileChange item，其 status 为 completed | failed | declined。
tool/requestUserInput

当客户端对 item/tool/requestUserInput 做出响应时，app-server 会发出 serverRequest/resolved，并包含 { threadId, requestId }。如果客户端尚未响应时，该待处理请求因为 turn start、turn completion 或 turn interruption 而被清除，服务器也会为这次清理发出相同通知。

动态工具调用（实验性）

thread/start 上的 dynamicTools 以及对应的 item/tool/call 请求或响应流程，都是实验性 API。

当某个动态工具在 turn 期间被调用时，app-server 会发出：

item/started，其中 item.type = "dynamicToolCall"，status = "inProgress"，并带有 tool 和 arguments。
item/tool/call，作为发给客户端的服务器请求。
客户端响应时返回的 content items 载荷。
item/completed，其中 item.type = "dynamicToolCall"，包含最终状态，以及任何返回的 contentItems 或 success 值。
MCP 工具调用审批（apps）

应用（连接器）工具调用也可能需要审批。当某个 app 工具调用带有副作用时，服务器可能通过 tool/requestUserInput 发起审批，并提供如 Accept、Decline、Cancel 等选项。带有 destructive 标注的工具即使同时声明了较低权限提示，也始终会触发审批。如果用户拒绝或取消，相关的 mcpToolCall item 会以错误结束，而不会执行该工具。

技能

通过在用户文本输入中包含 $<skill-name> 来调用技能。建议再附带一个 skill 输入项，这样服务器就会注入完整技能说明，而不是依赖模型自行解析该名称。

{
  "method": "turn/start",
  "id": 101,
  "params": {
    "threadId": "thread-1",
    "input": [
      {
        "type": "text",
        "text": "$skill-creator Add a new skill for triaging flaky CI."
      },
      {
        "type": "skill",
        "name": "skill-creator",
        "path": "/Users/me/.codex/skills/skill-creator/SKILL.md"
      }
    ]
  }
}

如果省略 skill item，模型仍会解析 $<skill-name> 标记并尝试定位该技能，但这可能增加延迟。

示例：

$skill-creator Add a new skill for triaging flaky CI and include step-by-step usage.

使用 skills/list 获取可用技能（可按 cwds 进行作用域限制，并支持 forceReload）。你还可以包含 perCwdExtraUserRoots，以便针对特定 cwd 扫描额外的绝对路径作为用户作用域。app-server 会忽略那些 cwd 不在 cwds 中的条目。skills/list 可能按 cwd 复用缓存结果；设置 forceReload: true 可以从磁盘刷新。若存在，服务器会从 SKILL.json 读取 interface 和 dependencies。

{ "method": "skills/list", "id": 25, "params": {
  "cwds": ["/Users/me/project", "/Users/me/other-project"],
  "forceReload": true,
  "perCwdExtraUserRoots": [
    {
      "cwd": "/Users/me/project",
      "extraUserRoots": ["/Users/me/shared-skills"]
    }
  ]
} }
{ "id": 25, "result": {
  "data": [{
    "cwd": "/Users/me/project",
    "skills": [
      {
        "name": "skill-creator",
        "description": "Create or update a Codex skill",
        "enabled": true,
        "interface": {
          "displayName": "Skill Creator",
          "shortDescription": "Create or update a Codex skill"
        },
        "dependencies": {
          "tools": [
            {
              "type": "env_var",
              "value": "GITHUB_TOKEN",
              "description": "GitHub API token"
            },
            {
              "type": "mcp",
              "value": "github",
              "transport": "streamable_http",
              "url": "https://example.com/mcp"
            }
          ]
        }
      }
    ],
    "errors": []
  }]
} }

按路径启用或禁用某个技能：

{
  "method": "skills/config/write",
  "id": 26,
  "params": {
    "path": "/Users/me/.codex/skills/skill-creator/SKILL.md",
    "enabled": false
  }
}

Apps（连接器）

使用 app/list 获取可用 apps。在 CLI/TUI 中，/apps 是面向用户的选择器；在自定义客户端中，请直接调用 app/list。每个条目同时包含 isAccessible（用户可访问）和 isEnabled（在 config.toml 中已启用），以便客户端区分安装/访问能力与本地启用状态。app 条目还可以包含可选的 branding、appMetadata 和 labels 字段。

{ "method": "app/list", "id": 50, "params": {
  "cursor": null,
  "limit": 50,
  "threadId": "thread-1",
  "forceRefetch": false
} }
{ "id": 50, "result": {
  "data": [
    {
      "id": "demo-app",
      "name": "Demo App",
      "description": "Example connector for documentation.",
      "logoUrl": "https://example.com/demo-app.png",
      "logoUrlDark": null,
      "distributionChannel": null,
      "branding": null,
      "appMetadata": null,
      "labels": null,
      "installUrl": "https://chatgpt.com/apps/demo-app/demo-app",
      "isAccessible": true,
      "isEnabled": true
    }
  ],
  "nextCursor": null
} }

如果你提供了 threadId，那么 app 功能门控（features.apps）会使用该线程的配置快照。若省略，则 app-server 使用最新的全局配置。

app/list 会在 accessible apps 和 directory apps 都加载完成后返回。设置 forceRefetch: true 可以绕过 app 缓存并获取最新数据。缓存条目只有在刷新成功时才会被替换。

每当任一来源（accessible apps 或 directory apps）完成加载时，服务器还会发出 app/list/updated 通知。每条通知都包含当前最新的合并后 app 列表。

{
  "method": "app/list/updated",
  "params": {
    "data": [
      {
        "id": "demo-app",
        "name": "Demo App",
        "description": "Example connector for documentation.",
        "logoUrl": "https://example.com/demo-app.png",
        "logoUrlDark": null,
        "distributionChannel": null,
        "branding": null,
        "appMetadata": null,
        "labels": null,
        "installUrl": "https://chatgpt.com/apps/demo-app/demo-app",
        "isAccessible": true,
        "isEnabled": true
      }
    ]
  }
}

要调用某个 app，请在文本输入中插入 $<app-slug>，并附带一个 path 为 app://<id> 的 mention 输入项（推荐）。

{
  "method": "turn/start",
  "id": 51,
  "params": {
    "threadId": "thread-1",
    "input": [
      {
        "type": "text",
        "text": "$demo-app Pull the latest updates from the team."
      },
      {
        "type": "mention",
        "name": "Demo App",
        "path": "app://demo-app"
      }
    ]
  }
}

应用设置的 Config RPC 示例

使用 config/read、config/value/write 和 config/batchWrite 来检查或更新 config.toml 中的 app 控制项。

读取生效后的 app 配置结构（包括 _default 和按工具覆盖）：

{ "method": "config/read", "id": 60, "params": { "includeLayers": false } }
{ "id": 60, "result": {
  "config": {
    "apps": {
      "_default": {
        "enabled": true,
        "destructive_enabled": true,
        "open_world_enabled": true
      },
      "google_drive": {
        "enabled": true,
        "destructive_enabled": false,
        "default_tools_approval_mode": "prompt",
        "tools": {
          "files/delete": { "enabled": false, "approval_mode": "approve" }
        }
      }
    }
  }
} }

更新单个 app 设置：

{
  "method": "config/value/write",
  "id": 61,
  "params": {
    "keyPath": "apps.google_drive.default_tools_approval_mode",
    "value": "prompt",
    "mergeStrategy": "replace"
  }
}

以原子方式应用多个 app 编辑：

{
  "method": "config/batchWrite",
  "id": 62,
  "params": {
    "edits": [
      {
        "keyPath": "apps._default.destructive_enabled",
        "value": false,
        "mergeStrategy": "upsert"
      },
      {
        "keyPath": "apps.google_drive.tools.files/delete.approval_mode",
        "value": "approve",
        "mergeStrategy": "upsert"
      }
    ]
  }
}

检测并导入 external agent 配置

使用 externalAgentConfig/detect 发现可迁移的 external-agent 制品，然后将选中的条目传给 externalAgentConfig/import。

检测示例：

{ "method": "externalAgentConfig/detect", "id": 63, "params": {
  "includeHome": true,
  "cwds": ["/Users/me/project"]
} }
{ "id": 63, "result": {
  "items": [
    {
      "itemType": "AGENTS_MD",
      "description": "Import /Users/me/project/CLAUDE.md to /Users/me/project/AGENTS.md.",
      "cwd": "/Users/me/project"
    },
    {
      "itemType": "SKILLS",
      "description": "Copy skill folders from /Users/me/.claude/skills to /Users/me/.agents/skills.",
      "cwd": null
    }
  ]
} }

导入示例：

{ "method": "externalAgentConfig/import", "id": 64, "params": {
  "migrationItems": [
    {
      "itemType": "AGENTS_MD",
      "description": "Import /Users/me/project/CLAUDE.md to /Users/me/project/AGENTS.md.",
      "cwd": "/Users/me/project"
    }
  ]
} }
{ "id": 64, "result": {} }

支持的 itemType 值包括 AGENTS_MD、CONFIG、SKILLS 和 MCP_SERVER_CONFIG。检测结果只会返回仍有工作需要执行的项。例如，当 AGENTS.md 已经存在且非空时，会跳过 AGENTS 迁移；技能导入也不会覆盖已有的技能目录。

认证端点

JSON-RPC 的 auth/account 接口既暴露请求/响应方法，也暴露服务器主动发起的通知（无 id）。使用这些接口可以判断认证状态、启动或取消登录、登出，以及检查 ChatGPT 速率限制。

认证模式

Codex 支持三种认证模式。account/updated.authMode 显示当前活跃模式，account/read 也会返回该信息。

API key（apikey）- 调用方提供 OpenAI API key，Codex 会保存它以供 API 请求使用。
ChatGPT managed（chatgpt）- Codex 自行管理 ChatGPT OAuth 流程，持久化 token，并自动刷新。
ChatGPT external tokens（chatgptAuthTokens）- 宿主应用直接提供 idToken 和 accessToken。Codex 只在内存中保存这些 token，当被请求时需要由宿主应用负责刷新。
API 概览

account/read - 获取当前账号信息；可选地刷新 token。
account/login/start - 开始登录（apiKey、chatgpt 或 chatgptAuthTokens）。
account/login/completed（notify）- 当登录尝试结束时发出（成功或出错）。
account/login/cancel - 通过 loginId 取消一个待处理的 ChatGPT 登录。
account/logout - 退出登录；会触发 account/updated。
account/updated（notify）- 每当认证模式变化时发出（authMode: apikey、chatgpt、chatgptAuthTokens 或 null）。
account/chatgptAuthTokens/refresh（server request）- 在发生授权错误后，请求刷新由外部管理的 ChatGPT token。
account/rateLimits/read - 获取 ChatGPT 速率限制。
account/rateLimits/updated（notify）- 每当用户的 ChatGPT 速率限制变化时发出。
mcpServer/oauthLogin/completed（notify）- 在 mcpServer/oauth/login 流程结束后发出；载荷包含 { name, success, error? }。
1）检查认证状态

请求：

{ "method": "account/read", "id": 1, "params": { "refreshToken": false } }

响应示例：

{ "id": 1, "result": { "account": null, "requiresOpenaiAuth": false } }

{ "id": 1, "result": { "account": null, "requiresOpenaiAuth": true } }

{
  "id": 1,
  "result": { "account": { "type": "apiKey" }, "requiresOpenaiAuth": true }
}

{
  "id": 1,
  "result": {
    "account": {
      "type": "chatgpt",
      "email": "user@example.com",
      "planType": "pro"
    },
    "requiresOpenaiAuth": true
  }
}

字段说明：

refreshToken（boolean）：设为 true 时，会在 managed ChatGPT 模式下强制刷新 token。在 external token 模式（chatgptAuthTokens）下，app-server 会忽略此标志。
requiresOpenaiAuth 反映当前 provider；为 false 时，Codex 可以在没有 OpenAI 凭证的情况下运行。
2）使用 API key 登录

发送：

{
  "method": "account/login/start",
  "id": 2,
  "params": { "type": "apiKey", "apiKey": "sk-..." }
}

预期返回：

{ "id": 2, "result": { "type": "apiKey" } }

通知：

{
  "method": "account/login/completed",
  "params": { "loginId": null, "success": true, "error": null }
}

{ "method": "account/updated", "params": { "authMode": "apikey" } }

3）使用 ChatGPT 登录（浏览器流程）

开始：

{ "method": "account/login/start", "id": 3, "params": { "type": "chatgpt" } }

{
  "id": 3,
  "result": {
    "type": "chatgpt",
    "loginId": "<uuid>",
    "authUrl": "https://chatgpt.com/...&redirect_uri=http%3A%2F%2Flocalhost%3A<port>%2Fauth%2Fcallback"
  }
}

在浏览器中打开 authUrl；app-server 会托管本地回调。

等待通知：

{
  "method": "account/login/completed",
  "params": { "loginId": "<uuid>", "success": true, "error": null }
}

{ "method": "account/updated", "params": { "authMode": "chatgpt" } }

3b）使用外部管理的 ChatGPT tokens 登录（chatgptAuthTokens）

当宿主应用自行掌管用户的 ChatGPT 认证生命周期，并直接提供 token 时，请使用这种模式。

发送：

{
  "method": "account/login/start",
  "id": 7,
  "params": {
    "type": "chatgptAuthTokens",
    "idToken": "<jwt>",
    "accessToken": "<jwt>"
  }
}

预期返回：

{ "id": 7, "result": { "type": "chatgptAuthTokens" } }

通知：

{
  "method": "account/login/completed",
  "params": { "loginId": null, "success": true, "error": null }
}

{
  "method": "account/updated",
  "params": { "authMode": "chatgptAuthTokens" }
}

当服务器收到 401 Unauthorized 时，它可能会向宿主应用请求刷新后的 token：

{
  "method": "account/chatgptAuthTokens/refresh",
  "id": 8,
  "params": { "reason": "unauthorized", "previousAccountId": "org-123" }
}
{ "id": 8, "result": { "idToken": "<jwt>", "accessToken": "<jwt>" } }

服务器会在收到成功的刷新响应后重试原始请求。请求大约会在 10 秒后超时。

4）取消一次 ChatGPT 登录

{ "method": "account/login/cancel", "id": 4, "params": { "loginId": "<uuid>" } }
{ "method": "account/login/completed", "params": { "loginId": "<uuid>", "success": false, "error": "..." } }

5）登出

{ "method": "account/logout", "id": 5 }
{ "id": 5, "result": {} }
{ "method": "account/updated", "params": { "authMode": null } }

6）速率限制（ChatGPT）

{ "method": "account/rateLimits/read", "id": 6 }
{ "id": 6, "result": {
  "rateLimits": {
    "limitId": "codex",
    "limitName": null,
    "primary": { "usedPercent": 25, "windowDurationMins": 15, "resetsAt": 1730947200 },
    "secondary": null
  },
  "rateLimitsByLimitId": {
    "codex": {
      "limitId": "codex",
      "limitName": null,
      "primary": { "usedPercent": 25, "windowDurationMins": 15, "resetsAt": 1730947200 },
      "secondary": null
    },
    "codex_other": {
      "limitId": "codex_other",
      "limitName": "codex_other",
      "primary": { "usedPercent": 42, "windowDurationMins": 60, "resetsAt": 1730950800 },
      "secondary": null
    }
  }
} }
{ "method": "account/rateLimits/updated", "params": {
  "rateLimits": {
    "limitId": "codex",
    "primary": { "usedPercent": 31, "windowDurationMins": 15, "resetsAt": 1730948100 }
  }
} }

字段说明：

rateLimits 是向后兼容的单桶视图。
rateLimitsByLimitId（若存在）是按计费 limit_id 分组的多桶视图（例如 codex）。
limitId 是计费桶标识符。
limitName 是可选的用户可见桶标签。
usedPercent 是当前配额窗口内的使用百分比。
windowDurationMins 是配额窗口长度。
