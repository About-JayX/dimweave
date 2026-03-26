# Codex 指令注入 V2 — 实施计划

> 基于 2026-03-26 session 的研究成果和链路验证结果。

## 当前状态

### 已完成

- [x] `baseInstructions` 替换 system prompt（角色 + 工具 + 输出格式 + 行为规则）
- [x] `outputSchema` 强制结构化输出 `{ message, send_to }` + daemon 自动路由
- [x] `dynamicTools` 注册 reply/check_messages/get_status
- [x] Codex 事件流式传输（thinking/delta/message/turnDone → GUI）
- [x] 虚拟消息列表（react-virtuoso）
- [x] Claude bridge 角色注入（AGENTBRIDGE_ROLE env → MCP instructions）
- [x] 默认 prompt 存档（`docs/codex/prompts/` 13 个模型，Apache 2.0）
- [x] 指令注入机制全景记录（baseInstructions/developerInstructions/AGENTS.md/Skills 源码确认）

### 已验证

- `baseInstructions` 完全替换 system prompt（PINEAPPLE 测试 ✅）
- `outputSchema` 路由生效（`[Route] coder → lead delivered` ✅）
- 覆盖 baseInstructions 不影响：MCP 工具、Skills、AGENTS.md、developer_sections
- 覆盖 baseInstructions 丢失：工具使用偏好、git 安全、编码风格（已手动补回 8 条）

---

## 迭代计划

### Phase 1: developerInstructions 用户自定义（前端 + 后端）

**目标:** 用户在 Connect Codex 前可以输入自定义指令，作为 `developerInstructions` 传入 thread/start。

**改动:**

| 文件 | 改动 |
|------|------|
| `src/components/AgentStatus/CodexPanel.tsx` | 新增 textarea 输入框（devInstructions） |
| `src/stores/bridge-store/types.ts` | 新增 `codexDevInstructions: string` |
| `src/stores/bridge-store/index.ts` | 新增 setter，`launchCodexTui` 传入 devInstructions |
| `src-tauri/src/commands.rs` | `daemon_launch_codex` 新增 `dev_instructions` 参数 |
| `src-tauri/src/daemon/codex/mod.rs` | 传入 SessionOpts.developer_instructions |
| `src-tauri/src/daemon/codex/session.rs` | SessionOpts 保留 developer_instructions 字段 |
| `src-tauri/src/daemon/codex/handshake.rs` | thread/start 同时传 baseInstructions + developerInstructions |

**数据流:**
```
UI textarea → invoke("daemon_launch_codex", { devInstructions })
  → handshake.rs thread/start {
      baseInstructions: 角色prompt（我们控制）,
      developerInstructions: 用户自定义（UI 输入）
    }
```

**注意:** baseInstructions 和 developerInstructions 并存不冲突。base 是 system prompt，dev 是 developer message。

---

### Phase 2: 内置 AgentBridge Skills

**目标:** 在 CODEX_HOME 中写入内置 skills，Codex 自动发现。

**新增 Skills:**

| Skill | 目录 | 用途 |
|-------|------|------|
| `agentbridge-comm` | `CODEX_HOME/skills/agentbridge-comm/SKILL.md` | 多 agent 通信：reply 路由决策树、结构化输出示例、角色交互模式 |
| `agentbridge-review` | `CODEX_HOME/skills/agentbridge-review/SKILL.md` | 代码审查流程：coder→reviewer→lead 工作流模板 |
| `agentbridge-debug` | `CODEX_HOME/skills/agentbridge-debug/SKILL.md` | 调试协作：收集→定位→修复→验证→汇报 |

**改动:**

| 文件 | 改动 |
|------|------|
| `src-tauri/src/daemon/session_manager.rs` | `create_session` 时写入 `skills/` 目录 |
| `src-tauri/src/daemon/codex/skills/` | 新增目录，存放 SKILL.md 模板常量 |

**写入时机:** `session_manager::create_session()` → 在创建 CODEX_HOME 时同步写入。

---

### Phase 3: Claude 侧对齐

**目标:** Claude 的指令能力目前弱于 Codex（只有 MCP instructions L5）。提升到 system prompt 级。

**方案:**
- `process.rs` 的 `build_claude_command` 增加 `--append-system-prompt` 或 `--append-system-prompt-file`
- 内容：角色身份 + 路由规则 + 工具使用规范
- 或：使用 `--agents '{"role":{...}}'` + `--agent role` 注入完整 subagent 定义

**改动:**

| 文件 | 改动 |
|------|------|
| `src-tauri/src/claude_session/process.rs` | `build_claude_command` 增加 `--append-system-prompt` |
| `src-tauri/src/daemon/role_config/roles.rs` | 新增 `claude_system_prompt` 字段 |

---

### Phase 4: 用户体验增强

| 功能 | 说明 | 优先级 |
|------|------|--------|
| devInstructions 持久化 | 保存到 localStorage 或项目 `.agentbridge/config.json` | 中 |
| 模型选择 UI（Codex 侧） | 下拉选 gpt-5.4 / gpt-5.3-codex 等 | 中 |
| personality 参数 | 按角色设置 friendly/pragmatic | 低 |
| collaborationMode | per-turn 切换 plan/execute/custom | 低 |
| Skills 管理 UI | 查看/启用/禁用内置和自定义 skills | 低 |

---

## 依赖关系

```
Phase 1 (devInstructions UI)
  ↓ 无依赖，可独立开发
Phase 2 (内置 Skills)
  ↓ 无依赖，可独立开发
Phase 3 (Claude 对齐)
  ↓ 依赖 Phase 1 的 role_config 结构
Phase 4 (UX 增强)
  ↓ 依赖 Phase 1-2 完成
```

Phase 1-3 可并行开发，互不阻塞。

---

## 关键参考

| 资料 | 路径 |
|------|------|
| Codex API 文档 | `docs/agents/codex-app-server-api.md` |
| Codex API 中文版 | `docs/agents/codex-app-server-api.zh-CN.md` |
| 默认 prompt 存档 | `docs/codex/prompts/*.md` |
| 链路修复记录 | `docs/agents/codex-chain.md` |
| Claude 链路记录 | `docs/agents/claude-chain.md` |
| 注入点全景 | `codex-chain.md` → "指令注入机制全景" 章节 |
| 源码参考 | `codex-rs/core/src/project_doc.rs`（AGENTS.md）、`codex-rs/core-skills/src/loader.rs`（Skills） |
