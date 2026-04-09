# Feishu MCP Bug Inbox — 完整流程 Plan

**日期:** 2026-04-09
**目标:** 通过飞书 MCP 拉取缺陷/待办数据，在 Bug Inbox 面板展示，支持筛选和翻页，点击 Handle 后自动拉取详情+评论组成上下文发给 lead 开始修复。

---

## 1. 数据流概览

```text
飞书 MCP Server (project.feishu.cn/mcp_server/v1)
    │
    │  HTTP JSON-RPC 2.0
    │  Auth: X-Mcp-Token header
    ▼
Rust MCP Client (feishu_project/mcp_client.rs)
    │  initialize → tools/list → tools/call
    ▼
MCP Sync Adapter (feishu_project/mcp_sync.rs)
    │  sync_mode: Todo → list_todo
    │  sync_mode: Issues → search_by_mql (分页)
    ▼
Inbox Store (feishu_project/store.rs)
    │  upsert → persist → emit event
    ▼
Frontend Store (feishu-project-store.ts)
    │  event listener + invoke
    ▼
BugInboxPanel (ConfigCard + SyncModeNav + IssueList)
    │
    │  Handle 点击
    ▼
Task Link (daemon/feishu_project_task_link.rs)
    │  fetch_issue_context → get_workitem_brief + list_workitem_comments
    │  create task → write snapshot → route handoff to lead
    ▼
Lead Agent (Claude/Codex)
```

---

## 2. 配置层

### 2.1 Config 文件

路径: `~/Library/Application Support/com.dimweave.app/feishu_project.json`

```json
{
  "enabled": true,
  "domain": "https://project.feishu.cn",
  "mcp_user_token": "m-xxx",
  "workspace_hint": "manciyuan",
  "refresh_interval_minutes": 10,
  "sync_mode": "issues"
}
```

### 2.2 字段说明

| 字段 | 说明 |
|------|------|
| `mcp_user_token` | 飞书项目 MCP 令牌，HTTP header `X-Mcp-Token` |
| `workspace_hint` | 空间 simpleName，用于 MQL FROM 子句和 URL 生成 |
| `sync_mode` | `todo` = list_todo（我的待办）, `issues` = search_by_mql（缺陷管理） |
| `refresh_interval_minutes` | 自动刷新间隔（分钟） |

### 2.3 Save 合并逻辑

前端 edit 保存时，空字段不覆盖已有值（`feishu_project_lifecycle.rs :: save_and_restart`）：
- `mcp_user_token` 为空 → 保留磁盘上的旧 token
- `workspace_hint` 为空 → 保留磁盘上的旧值

---

## 3. 连接与发现

### 3.1 connect_and_discover (`runtime.rs`)

1. 创建 `McpClient(domain, token)`
2. `initialize` → 协议握手
3. `tools/list` → 发现 35 个工具，缓存到 `client.catalog`
4. 状态设为 `Connected`

### 3.2 首次连接附加查询

连接成功后自动执行：

| 查询 | MCP 工具 | 结果 |
|------|---------|------|
| 项目名 | `search_project_info(project_key)` | `极光矩阵--娱乐站` → 存入 `RuntimeState.project_name` |
| 经办人列表 | `search_by_mql(GROUP BY current_status_operator)` | 去重后的经办人名字列表 → 存入 `RuntimeState.team_members` |

### 3.3 RuntimeState 存储

`update_mcp_state` 把运行时状态写入 `DaemonState.feishu_project_runtime`，前端 `fetchState` 直接从内存读取（不再从磁盘重建，解决了 StatusDot 不变绿的问题）。

---

## 4. 同步模式

### 4.1 我的待办 (sync_mode = todo)

```
MCP: list_todo(action="todo", page_num=1)
返回: {total, list: [{work_item_info: {work_item_id, work_item_name, work_item_type_key}, project_key, node_info, ...}]}
```

- 一次请求，不翻页
- 解析嵌套的 `work_item_info` 结构

### 4.2 缺陷管理 (sync_mode = issues)

```
MCP: search_by_mql(project_key, mql="SELECT work_item_id, name, priority, current_status_operator, bug_classification FROM {ws}.issue LIMIT {offset}, 50")
返回: {list: [{count}], session_id, data: {"1": [{moql_field_list: [...]}]}}
```

- 首次加载 50 条（offset=0）
- 触底自动加载下一页（offset += 50）
- MQL 实际每页上限 **50 条**（飞书服务端限制，不受 LIMIT 值控制）
- `work_item_id` 从 `long_value` 类型提取（非 `string_value`）

### 4.3 MQL 字段解析 (parse_mql_item)

```
moql_field_list 字段 → value 类型映射:
- name          → value.string_value
- work_item_id  → value.long_value
- priority      → value.key_label_value.label (P0/P1/P2/P3)
- current_status_operator → value.user_value_list[].name_cn
- bug_classification      → value.key_label_value.label (iOS/Android/WEB/Server)
```

### 4.4 Store 刷新策略

每次 sync 前 **清空** store 再填充（`store.items.clear()`），避免切换 mode 后旧数据残留。

---

## 5. 翻页与加载

### 5.1 后端

- `run_mcp_sync_cycle` → 首页 50 条
- `load_more` → `sync_issues_page(client, workspace, offset)` 追加到 store
- 返回本页条数，前端根据 `count < 50` 判断是否还有更多

### 5.2 前端

- `IssueList` 底部放 IntersectionObserver sentinel
- sentinel 进入视口 → 调 `loadMore()`
- loading 时显示 spinner，不是全屏骨架
- `hasMore = false` 时移除 sentinel

### 5.3 命令链路

```
前端 loadMore() → invoke("feishu_project_load_more")
  → DaemonCmd::FeishuProjectLoadMore
  → lifecycle::load_more()
  → connect_lite (仅 initialize，跳过 tools/list)
  → sync_issues_page(offset)
  → upsert to store + persist + emit
  → 前端 invoke("feishu_project_list_items") 拿最新列表
```

### 5.4 连接优化

| 场景 | 连接方式 | 请求数 |
|------|---------|--------|
| 首次 sync / Sync now | `connect_and_discover` (initialize + tools/list) | 3 (握手+发现+查询) |
| load_more 翻页 | `connect_lite` (仅 initialize) | 2 (握手+查询) |
| Handle 拉详情 | `connect_lite` (仅 initialize) | 3 (握手+详情+评论) |

`connect_lite` 跳过 `tools/list`（35 个工具的 catalog 发现），翻页和详情不需要 catalog。

### 5.5 自动 dispatch 策略

sync 拉到新 item 后 **不自动 dispatch 给 lead**。原因：
- 50 个新 bug 会触发 50 次 `start_handling`，每次都连接 MCP 拉详情+评论
- 总共 150+ 次 HTTP 请求，造成卡顿
- 用户应在 Bug Inbox 里手动选择哪些 bug 需要 Handle

---

## 6. 筛选

### 6.1 经办人下拉

- 数据源：`RuntimeState.team_members`（连接时一次 MQL GROUP BY 拿到）
- 仅显示缺陷管理中有 bug 分配的经办人
- 前端 `assigneeFilter` 本地过滤 `items`，不重新请求后端

### 6.2 Mode 切换

- `SyncModeNav` 下拉切换 `todo` / `issues`
- 切换时：清空 items → saveConfig(新 mode) → syncNow → fetchItems
- 重置 assigneeFilter 和 hasMore

---

## 7. Handle（派发给 Lead）

### 7.1 触发

用户在 IssueList 的 ActionMenu 点击 "Handle"

### 7.2 流程 (`feishu_project_task_link.rs :: start_handling`)

```
1. 查找 work_item → 检查 linked_task_id
   ├─ 已有 task → select_task → 返回（不重复创建）
   └─ 没有 task → 继续
2. fetch_issue_context()
   ├─ get_workitem_brief(project_key, work_item_id, fields=[description, priority, bug_classification, issue_stage])
   │   → 拿到: 描述、状态、优先级、分类、阶段、经办人、报告人、创建/更新时间
   └─ list_workitem_comments(project_key, work_item_id, page_num=1)
       → 拿到: 评论列表（author, content, created_at）
3. 组成 context JSON:
   {work_item_id, name, type, status, priority, classification, stage,
    reporter, operator, description, comments[], source_url, created_at, updated_at}
4. create_and_select_task(workspace, "[issue] title")
5. write_context_snapshot(context, task_id) → 写入 snapshots 目录
6. build_handoff_message()
   → 消息内容包含: 标题、状态、描述摘要、评论摘要、飞书链接
   → 附件: snapshot JSON 文件
7. route_message(to="lead") → 发给 lead
```

### 7.3 Snapshot 文件

路径: `~/Library/Application Support/com.dimweave.app/feishu_project_snapshots/{task_id}.json`

```json
{
  "work_item_id": "6952644360",
  "name": "安全问题：可通过获取user_id...",
  "type": "issue",
  "status": "已关闭",
  "priority": "P3",
  "classification": "WEB",
  "stage": "测试环境",
  "reporter": [{"key": "762...", "name": "大永"}],
  "operator": [{"key": "761...", "name": "Grape"}],
  "description": "URL：https://...\n复现步骤：...\n[图片]",
  "comments": [
    {"author": "762...", "content": "@Grape 邮箱敏感信息...", "created_at": "2026-04-09 18:33:15"},
    {"author": "761...", "content": "历史接口问题...", "created_at": "2026-04-09 19:38:44"}
  ],
  "source_url": "https://project.feishu.cn/manciyuan/issue/detail/6952644360",
  "created_at": "2026-04-09T18:30:35+08:00",
  "updated_at": "2026-04-09T19:51:58+08:00"
}
```

### 7.4 已知限制

- **图片无法通过 MCP 获取** — 描述中的 `[图片]` 只是占位符，需要点飞书链接查看
- 获取图片需要飞书 OpenAPI（app_id + app_secret），当前未接入

---

## 8. UI 布局

### 8.1 BugInboxPanel 结构

```
┌─────────────────────────────────────────┐
│ ● 极光矩阵--娱乐站        19:53    ⋮  │ ← ConfigCard (固定)
│ [缺陷管理 ▾] [全部经办人 ▾]            │ ← SyncModeNav (固定)
├─────────────────────────────────────────┤
│ ISSUE  P1·WEB                       ⋮  │ ← IssueList (滚动)
│ 组件行数限制问题                         │
│ 橙子  Mcp                               │
│─────────────────────────────────────────│
│ ISSUE  P3·WEB                       ⋮  │
│ 我的简介信息存在空格...                   │
│ 大永  Mcp                               │
│─────────────────────────────────────────│
│ ...                                     │
│         ◌ (触底加载 spinner)             │
└─────────────────────────────────────────┘
```

### 8.2 组件职责

| 组件 | 职责 |
|------|------|
| `ConfigCard` | 项目名 + StatusDot + 时间 + ActionMenu(Sync/Edit/Disable) |
| `SyncModeNav` | mode 下拉 + 经办人筛选下拉 |
| `IssueList` | 列表渲染 + 触底加载 + 骨架屏 + ActionMenu(Handle/Ignore) |
| `ActionMenu` | portal 渲染下拉菜单（避免 overflow:hidden 裁剪） |

### 8.3 固定区 vs 滚动区

- ConfigCard + SyncModeNav 在固定区（`shrink-0`）
- IssueList 在独立滚动区（`flex-1 overflow-y-auto`）
- 父容器 TaskContextPopover 对 bugs 面板用 `overflow-hidden`，由 BugInboxPanel 自己管滚动

---

## 9. 文件映射

| 文件 | 职责 |
|------|------|
| `feishu_project/types.rs` | FeishuSyncMode, FeishuProjectConfig, RuntimeState (含 project_name, team_members) |
| `feishu_project/config.rs` | 磁盘读写 config JSON |
| `feishu_project/store.rs` | InboxItem upsert/persist (返回 bool 区分新增/更新) |
| `feishu_project/mcp_client.rs` | MCP HTTP 客户端 (initialize, tools/list, tools/call) |
| `feishu_project/mcp_sync.rs` | sync_todo, sync_issues_page, parse_mql_items, parse_tool_result |
| `feishu_project/runtime.rs` | connect_and_discover, run_mcp_sync_cycle, fetch_project_name, fetch_team_members, update_mcp_state |
| `daemon/feishu_project_lifecycle.rs` | save_and_restart, sync_now, load_more, get_runtime_state |
| `daemon/feishu_project_task_link.rs` | start_handling, fetch_issue_context, write_context_snapshot, build_handoff_message |
| `daemon/cmd.rs` | DaemonCmd 枚举 (含 FeishuProjectLoadMore) |
| `daemon/mod.rs` | daemon 主循环 command 处理 |
| `daemon/state.rs` | DaemonState.feishu_project_runtime 内存缓存 |
| `commands_feishu_project.rs` | Tauri command handlers |
| `main.rs` | command 注册 |
| `stores/feishu-project-store.ts` | 前端 Zustand store (含 loadMore, hasMore, loadingMore) |
| `BugInboxPanel/index.tsx` | 主面板 (固定区+滚动区布局) |
| `BugInboxPanel/ConfigCard.tsx` | 项目 header |
| `BugInboxPanel/SyncModeNav.tsx` | mode + 经办人下拉 |
| `BugInboxPanel/IssueList.tsx` | 列表 + IntersectionObserver 触底加载 |
| `AgentStatus/ActionMenu.tsx` | portal 下拉菜单 |

---

## 10. MCP 工具使用汇总

| 用途 | 工具 | 调用时机 |
|------|------|---------|
| 协议握手 | `initialize` | 每次连接 |
| 工具发现 | `tools/list` | 每次连接 |
| 项目名 | `search_project_info` | 连接后 |
| 经办人列表 | `search_by_mql` (GROUP BY) | 连接后 |
| 待办列表 | `list_todo` | sync (todo mode) |
| 缺陷列表 | `search_by_mql` (LIMIT offset, 50) | sync (issues mode) |
| 缺陷详情 | `get_workitem_brief` | Handle 时 |
| 评论列表 | `list_workitem_comments` | Handle 时 |

---

## CM Memory

| Commit | Task | 验证命令 | Memory |
|--------|------|---------|--------|
| `0e6c5abb` | 修复 feishu task-link 测试漂移 | `cargo test --manifest-path src-tauri/Cargo.toml feishu_project_task_link -- --nocapture` (5 passed) | 测试调用 `write_context_snapshot` + 4 参数 `build_handoff_message`，断言基于 context JSON 字段而非旧 camelCase DTO |
| `3277d2c8` | Task 2A: 全量 refresh 保留本地工作流状态 | `cargo test --manifest-path src-tauri/Cargo.toml feishu_project::store -- --nocapture` (9 passed); `cargo check --manifest-path src-tauri/Cargo.toml` ✅ | 全量 refresh 必须用 `sync_replace()` 而非 `items.clear()` + `upsert()`；`sync_replace` 先 retain 远端仍存在的 item（保留其 `ignored` / `linked_task_id`），再 upsert 新增 item，最后移除远端已不存在的 item |
| `7bd19593` | Task 2B: handoff description 富文本链路 | `cargo test --manifest-path src-tauri/Cargo.toml feishu_project_task_link -- --nocapture` (8 passed); `cargo check --manifest-path src-tauri/Cargo.toml` ✅ | handoff description 必须用 `extract_description_text()` 而非直接 `.as_str()`；优先提取 rich-text 对象的 `doc_text` 字段，纯字符串直接返回，缺失时才 fallback `(no description)` |
| `a503590b` | Task 2C: lifecycle runtime-state stale cache | `cargo test --manifest-path src-tauri/Cargo.toml feishu_project_lifecycle -- --nocapture` (4 passed); `cargo check --manifest-path src-tauri/Cargo.toml` ✅ | `save_and_restart()` stop 后必须先清 `feishu_project_runtime = None`；disable/start-failure 不能继续回发旧 connected/project_name/team_members 运行态；`get_runtime_state()` 缓存为 None 时回退 `from_config()` 重建 |
| `a822d63f` | Task 2D: mcp_sync 过期测试对齐 | `cargo test --manifest-path src-tauri/Cargo.toml feishu_project::mcp_sync -- --nocapture` (10 passed); `cargo test --manifest-path src-tauri/Cargo.toml feishu_project -- --nocapture` (72 passed) | `resolve_listing_tool` 现为 exact-match only（`find_tool` 用 `t.name == name`），测试不得再假设 substring fallback；no-match 错误文案为 `"no listing tool found in catalog (N tools)"`，不再列举 catalog 名 |
| `8788730d` | Task 2E: Disable 配置清理语义 | `cargo test --manifest-path src-tauri/Cargo.toml feishu_project_lifecycle -- --nocapture` (8 passed); `cargo check --manifest-path src-tauri/Cargo.toml` ✅ | disable（`enabled=false`）时不得 merge 回旧 `mcp_user_token` / `workspace_hint`，空字段保持为空；enabled=true 的 edit-save 仍保留空字段旧值；`merge_config()` 纯函数收口此逻辑 |
| `67a843a8` | Task 3A: BugInboxPanel 前端测试对齐 | `bun test src/stores/feishu-project-store.test.ts src/components/BugInboxPanel/ConfigCard.test.tsx src/components/BugInboxPanel/index.test.tsx` (12 passed); `bun run build` ✅ | ActionMenu 使用 `createPortal` 渲染菜单项，`renderToStaticMarkup` 不含 portal 内容；测试必须断言 trigger（`aria-label="Actions"`）和 inline 输出（名称/Linked badge/样式），不断言菜单项文案 |
| `33125abc` | Task 2F: manual sync_now runtime-state 刷新 | `cargo test --manifest-path src-tauri/Cargo.toml feishu_project_lifecycle -- --nocapture` (8 passed); `cargo check --manifest-path src-tauri/Cargo.toml` ✅ | manual `sync_now()` 成功后必须调用 `update_mcp_state` 刷新并 emit 最新 runtime state（`last_sync_at` / `mcp_status` / `project_name` / `team_members`）；不能只更新 items 而让 runtime state 停留旧值；`update_mcp_state` 已提升为 `pub(crate)` 可复用 |
