# File Attachment Support (Phase 1 — File Paths)

> 日期: 2026-04-03
> 状态: ✅ 已完成

## 背景

用户需要在 Dimweave 中发送文件和图片给 agent。当前消息系统是纯文本的（`BridgeMessage.content: String`），前端到 daemon 到 agent 的整条链路都没有附件字段。

## 协议调研结论

| 能力 | Claude (`--sdk-url`) | Codex (`app-server`) |
|------|---------------------|---------------------|
| 文本 | ✅ `{"type":"text","text":"..."}` | ✅ `{"type":"text","text":"..."}` |
| 图片 URL | ❌ 无文档依据 | ✅ `{"type":"image","url":"..."}` |
| 本地图片 | ❌ 无文档依据 | ✅ `{"type":"localImage","path":"..."}` |

**证据来源:**
- Codex: `docs/agents/codex-app-server-api.md` lines 505-509，`model/list` 返回 `inputModalities: ["text", "image"]`
- Claude: `src-tauri/src/daemon/claude_sdk/protocol.rs` lines 92-103，NDJSON content 数组只支持 `{"type":"text"}`
- Claude 协议测试: `protocol_tests_format.rs` 显式验证只有 `"text"` 类型

## 设计决策

| 决策 | 选择 | 原因 |
|------|------|------|
| Phase 1 范围 | 仅文件路径传递 | 两个 agent 都能读本地文件；图片需要更多协议调研 |
| 附件传输方式 | 文本内联 `[Attached files:\n- /path]` | 最小改动面，不需要改 Codex 注入通道类型 |
| 前端交互 | 拖拽 + Paperclip 按钮 | 用户选择 |
| 气泡展示 | Paperclip 图标 + 文件名标签 | Phase 2 加图片预览 |

## 架构改动

### 数据流

```
用户拖拽/选择文件
  ↓
ReplyInput: useAttachments() 管理 Attachment[]
  ↓
sendToCodex(content, target, attachments)
  ↓
invoke("daemon_send_user_input", { content, target, attachments })
  ↓
DaemonCmd::SendUserInput { content, target, attachments }
  ↓
route_user_input() → build_user_message(attachments)
  ↓
BridgeMessage { content, attachments: Some([...]) }
  ↓
├─ Claude: format_ndjson_user_message() → 内容末尾追加 [Attached files: ...]
├─ Codex: format_codex_input() → 同上
└─ GUI: emit_agent_message() → 气泡渲染附件标签
```

### 新增类型

```typescript
// src/types.ts
interface Attachment { filePath: string; fileName: string; }
// BridgeMessage 新增: attachments?: Attachment[]
```

```rust
// src-tauri/src/daemon/types.rs + bridge/src/types.rs
struct Attachment { file_path: String, file_name: String }
// BridgeMessage 新增: attachments: Option<Vec<Attachment>>
```

### 新增/修改文件

| 文件 | 操作 | 说明 |
|------|------|------|
| `src/types.ts` | 修改 | `Attachment` 接口 + `BridgeMessage.attachments` |
| `src-tauri/src/daemon/types.rs` | 修改 | `Attachment` struct + field |
| `src-tauri/src/daemon/types_dto.rs` | 新建 | 从 types.rs 提取前端 DTO，保持 200 行限制 |
| `bridge/src/types.rs` | 修改 | 同步 `Attachment` |
| `src-tauri/src/main.rs` | 修改 | `pick_files` 命令 |
| `src-tauri/src/daemon/cmd.rs` | 修改 | `SendUserInput.attachments` |
| `src-tauri/src/commands.rs` | 修改 | 接受 `attachments` 参数 |
| `src-tauri/src/daemon/routing_user_input.rs` | 修改 | 透传 attachments |
| `src-tauri/src/daemon/routing_format.rs` | 新建 | 从 routing.rs 提取格式化函数 + `append_attachment_context` |
| `src/stores/bridge-store/types.ts` | 修改 | `sendToCodex` 签名 |
| `src/stores/bridge-store/index.ts` | 修改 | 传递 attachments |
| `src/components/ReplyInput/index.tsx` | 重构 | 拖拽 + 附件按钮 + 发送 |
| `src/components/ReplyInput/TargetPicker.tsx` | 新建 | 从 ReplyInput 提取 |
| `src/components/ReplyInput/AttachmentStrip.tsx` | 新建 | 附件预览条 |
| `src/components/ReplyInput/use-attachments.ts` | 新建 | 附件状态管理 hook |
| `src/components/MessagePanel/MessageBubble.tsx` | 修改 | 渲染附件标签 |

## 本次会话全部 Commit 记录

基线: `0ef312d3 Merge branch 'perf-ux-rebuild'`

### UI 布局与品牌重命名

| Commit | 说明 | 文件数 |
|--------|------|--------|
| `f6f84e6e` | 合并双顶栏、可调宽侧边栏、发送区重构、品牌图标、配色优化，agent-nexus → dimweave 重命名（crate/config/docs） | 42 |
| `cd6a81d8` | 深度品牌重命名：Rust 源码测试、文档、临时目录前缀全部 agent-nexus → dimweave | 24 |

### 消息气泡与输入 UX

| Commit | 说明 | 文件数 |
|--------|------|--------|
| `96a42be7` | Disconnect 红色按钮、per-source 品牌色气泡背景、简化 SourceBadge、stream indicator 融入聊天流、连接标签药丸化、Enter 发送 + IME 防护、CSS radius vars | 14 |

### 主题与圆角系统

| Commit | 说明 | 文件数 |
|--------|------|--------|
| `0c02217b` | Light/dark/auto 主题切换（use-theme hook）、全局 `--app-radius` 圆角控制（use-border-radius hook）、light mode 配色、滚动条/阴影/CyberSelect 主题适配、sidebar 弹出菜单选择器、radius-keep 豁免类 | 19 |

### 文件附件功能 (Phase 1)

| Commit | 说明 | 文件数 |
|--------|------|--------|
| `7f8b6f73` | `Attachment` 类型 + `BridgeMessage.attachments` 跨 TS/Rust/Bridge，提取 types_dto.rs | 19 |
| `45b8f15c` | `pick_files` Tauri 命令 + DaemonCmd/commands 层 attachments 参数透传 | 5 |
| `e941fdc4` | 新建 `routing_format.rs`，从 routing.rs 提取格式化函数 + `append_attachment_context` | 3 |
| `3aa2292c` | Store `sendToCodex` 接受 attachments 参数 | 2 |
| `3fd588f5` | ReplyInput 拆分为目录（index + TargetPicker + AttachmentStrip + use-attachments），拖拽 + Paperclip 按钮 | 4 |
| `d6de4640` | MessageBubble 气泡内渲染 Paperclip + 文件名附件标签 | 1 |
| `0c855d97` | CLAUDE.md + tauri.md 文档更新 | 2 |
| `c7de1bfd` | 计划+执行文档 | 1 |
| `095ac038` | cargo fmt 全工作区格式化（无功能改动） | 19 |

### 统计

- **总 commit**: 13
- **总改动文件**: ~155 次文件修改（含重复修改同一文件）
- **新建文件**: types_dto.rs, routing_format.rs, use-theme.ts, use-border-radius.ts, BrandIcons.tsx, ReplyInput/TargetPicker.tsx, ReplyInput/AttachmentStrip.tsx, ReplyInput/use-attachments.ts, 2026-04-03-file-attachment-support.md

## 验证结果

- ✅ 289 Rust 测试通过（含 3 个新 routing_format 测试）
- ✅ TypeScript 无新错误
- ✅ 所有新文件 ≤200 行
- ✅ 工作区干净（git status clean）
- ⏳ 手动集成测试待执行

## Phase 2 范围（未实现）

- 图片内联预览（缩略图）在气泡中渲染
- Codex `localImage` 结构化输入（需要改 inject 通道类型）
- Claude 图片支持（需要验证 `--sdk-url` 是否接受 image content block）
- 文件大小/类型校验
- 粘贴板图片支持
