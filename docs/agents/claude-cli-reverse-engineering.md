# Claude CLI 深度逆向记录（2026-03-31）

> 目标：确认 Claude CLI 是否存在比 `--append-system-prompt` 更强的 prompt 注入位点，并梳理它与 channel / stream / teammate 模式的真实链路。

## 结论先行

1. **Claude CLI 确实支持 `--system-prompt`，而且它不是“追加”，而是替换默认 prompt 的强注入位点。**
2. **`--append-system-prompt` 只是较弱的尾部追加。**
3. **Claude CLI 还内置了一组未在常规产品文档中充分暴露的 agent/swarm/channel/sdk 参数**，包括：
   - `--channels`
   - `--dangerously-load-development-channels`
   - `--agent-id`
   - `--agent-name`
   - `--team-name`
   - `--parent-session-id`
   - `--teammate-mode`
   - `--agent-type`
   - `--sdk-url`
4. **stream 返回与 channel 可以共存**：源码里 `--sdk-url` 会把 headless 模式切到 `stream-json`，而 channel 允许在交互或 print/SDK 模式下同时注册。

---

## 0. 样本与入口

### 本机样本

```bash
which claude
# /Users/jason/.nvm/versions/node/v24.14.0/bin/claude

claude --version
# 2.1.88 (Claude Code)
```

### 实际安装目录

```bash
$(npm root -g)/@anthropic-ai/claude-code
```

目录内关键文件：

- `cli.js`
- `cli.js.map`
- `README.md`
- `package.json`

说明：

- `cli.js` 是 bun/ts 打包后的入口 bundle
- `cli.js.map` 带完整 source map，可直接还原原始 TS/TSX 源文件名与源码内容

---

## 1. 从文档缺口切入

先查安装包自带 README / package metadata：

```bash
pkg=$(npm root -g)/@anthropic-ai/claude-code
rg -n "system-prompt|append-system-prompt|sdk-url|channels|dangerously-load-development-channels" \
  "$pkg/README.md" "$pkg/package.json"
```

结果：**无命中**。

这说明：

- 安装包自带 README 没有系统性描述这些参数
- 至少从“文档表层”看，`system-prompt / sdk-url / channels` 不是主推公开方案

---

## 2. 从 bundle 里搜关键字

```bash
pkg=$(npm root -g)/@anthropic-ai/claude-code
rg -n "systemPromptFlag|appendSystemPromptFlag|append-system-prompt|system-prompt|channels|dangerously-load-development-channels|sdk-url|parent-session-id|teammate-mode|agent-type" \
  "$pkg/cli.js"
```

命中表明：

- bundle 内部存在：
  - `systemPromptFlag`
  - `appendSystemPromptFlag`
- 同时存在 channel / teammate / sdk 参数处理逻辑

这一步的关键意义：

- Claude CLI 内部**明确区分**“system prompt”与“append system prompt”
- 它不是单一路径

---

## 3. 用 source map 还原真实源码位置

### 3.1 搜 source map 中的来源文件

```bash
node - <<'NODE'
const fs=require('fs');
const mapPath=require('child_process').execSync('npm root -g').toString().trim()+'/@anthropic-ai/claude-code/cli.js.map';
const map=JSON.parse(fs.readFileSync(mapPath,'utf8'));
const needles=['systemPromptFlag','appendSystemPromptFlag','--append-system-prompt','--system-prompt','dangerously-load-development-channels','parent-session-id','teammate-mode','agent-type'];
for (const needle of needles){
  const hits=[];
  map.sourcesContent.forEach((c,i)=>{ if(c && c.includes(needle)) hits.push({i,src:map.sources[i]}); });
  console.log('NEEDLE',needle,'HITS',hits.length);
  hits.slice(0,15).forEach(h=>console.log(' ',h.i,h.src));
}
NODE
```

关键命中：

- `../src/main.tsx`
- `../src/utils/systemPrompt.ts`
- `../src/cli/print.ts`
- 多个 swarm / teammate / channel 相关文件

---

## 4. 直接还原 prompt 解析链路

### 4.1 `main.tsx`：CLI 参数解析

source map 还原出的 `../src/main.tsx` 中，存在明确逻辑：

- `let systemPrompt = options.systemPrompt;`
- `if (options.systemPromptFile) { ... readFileSync ... }`
- `let appendSystemPrompt = options.appendSystemPrompt;`
- `if (options.appendSystemPromptFile) { ... readFileSync ... }`

也就是：

- `--system-prompt`
- `--system-prompt-file`
- `--append-system-prompt`
- `--append-system-prompt-file`

全部都是真参数，不是 telemetry 幻影字段。

### 4.2 `main.tsx`：启动 telemetry 也区分两者

同文件还会记录：

- `systemPromptFlag: systemPrompt ? ...`
- `appendSystemPromptFlag: appendSystemPrompt ? ...`

说明内部埋点也把两者视为**不同等级**的注入方式。

---

## 5. 还原真正的 prompt 组装优先级

source map 里的 `../src/utils/systemPrompt.ts` 给出了最关键证据。

### `buildEffectiveSystemPrompt()` 的优先级

源码注释可还原为：

0. `overrideSystemPrompt` → **完全替换**
1. coordinator prompt
2. agent prompt
3. `customSystemPrompt`（即 `--system-prompt`）
4. default system prompt

并且：

- `appendSystemPrompt` 永远在末尾追加
- 唯一例外是有 `overrideSystemPrompt`

### 关键实现含义

当 `customSystemPrompt` 存在时，返回逻辑等价于：

```ts
return [
  customSystemPrompt,
  ...(appendSystemPrompt ? [appendSystemPrompt] : []),
]
```

即：

- `--system-prompt` **替换默认 prompt 主体**
- `--append-system-prompt` **只是在末尾补一段**

这已经和 Codex 的“base instruction vs appendendum”结构非常接近。

---

## 6. 运行时探测：证明参数真实可用

为了避免只停留在静态分析，做了最小运行探测。

### 6.1 直接探测 `--system-prompt-file`

```bash
claude --system-prompt-file /definitely/missing -p hi
```

输出：

```text
Error: System prompt file not found: /definitely/missing
```

说明：

- 参数被正常识别
- 执行流进入了 system prompt 文件读取分支

### 6.2 直接探测 `--append-system-prompt-file`

```bash
claude --append-system-prompt-file /definitely/missing -p hi
```

输出：

```text
Error: Append system prompt file not found: /definitely/missing
```

### 6.3 探测隐藏参数 `--sdk-url`

```bash
claude --sdk-url ws://127.0.0.1:1 --system-prompt-file /definitely/missing -p hi
```

输出仍然先落在：

```text
Error: System prompt file not found: /definitely/missing
```

说明：

- `--sdk-url` 被 CLI 正常接受
- 否则应该先报 `unknown option`

### 6.4 探测隐藏参数 `--channels`

```bash
claude --channels server:test --system-prompt-file /definitely/missing -p hi
```

同样没有 unknown option。

### 6.5 探测隐藏 teammate 参数

```bash
claude --parent-session-id abc --agent-id a --agent-name b --team-name c \
  --teammate-mode auto --agent-type worker \
  --system-prompt-file /definitely/missing -p hi
```

同样被成功解析。

结论：

- 这些参数不是 dead code
- 当前本机版本 `2.1.88` 的确接受它们

---

## 7. channel 与 stream 共存链路

### 7.1 `main.tsx` 中对 `--channels` 的说明

source map 还原出的注释明确写了：

- `--channels works in both interactive and print/SDK modes`
- dev channels 只在 interactive 模式下保留确认框

这说明 channel 注册不是“仅交互可用”。

### 7.2 `main.tsx` 中对 `--sdk-url` 的行为

源码显示：

- 有 `sdkUrl` 时
  - 自动把 `inputFormat` 设为 `stream-json`
  - 自动把 `outputFormat` 设为 `stream-json`
  - 自动启用 `print`
  - 自动启用 `verbose`

### 7.3 `print.ts` 中的 stdout 保护

`../src/cli/print.ts` 里还有：

- 当 `outputFormat === 'stream-json'` 时安装 stdout guard
- 防止非 JSON 输出污染 SDK 流

综合起来：

- **channel**：负责会话外部结构化注入 / inbound notification
- **stream-json / sdk-url**：负责 headless 流式 I/O

两者是不同层，不冲突。

---

## 8. teammate / swarm 侧链路

source map 中 `main.tsx` 明确注册了：

- `--agent-id`
- `--agent-name`
- `--team-name`
- `--agent-color`
- `--plan-mode-required`
- `--parent-session-id`
- `--teammate-mode`
- `--agent-type`

这说明 Claude CLI 自己就有一整套 teammate/swarm 输入面。

因此，AgentNexus 当前用 channel 自行做多代理编排，并不是唯一可能路径；Claude CLI 内部其实也有自己的 agent/team 语义层。

---

## 9. 与当前 AgentNexus 实现的对照

### 当前项目的 Claude 注入方式

`src-tauri/src/claude_launch.rs`

- 当前写死的是 `--append-system-prompt`
- 内容来自 `role_config::claude_system_prompt(role)`

### 当前项目的 channel / PTY 启动方式

`src-tauri/src/claude_session/process.rs`

- `--dangerously-load-development-channels server:agentnexus`
- `--dangerously-skip-permissions`
- `--mcp-config <project>/.mcp.json`

### 当前项目的 prompt 文本来源

`src-tauri/src/daemon/role_config/claude_prompt.rs`

- 现在只是构造一段追加 prompt
- 依赖 Claude 自己遵守 `reply(to, text, status)` 协议

### 对照 Codex

`src-tauri/src/daemon/role_config/roles.rs`

- Codex 侧有：
  - `base_instructions`
  - `output_schema()`

强度明显高于 Claude 侧当前方案。

---

## 10. 可利用结论

### 方案 A：把 Claude 主角色 prompt 从 append 升级为 system

建议把当前 launcher 改成：

1. 主角色规范 → `--system-prompt`
2. 运行期附加约束/补丁 → `--append-system-prompt`

推荐拆分：

- `system prompt`：角色、路由、输出协议、必须回传 lead 等硬规则
- `append prompt`：任务态信息、实验开关、临时调试附言

### 方案 B：继续保留 channel

因为：

- channel 与 stream-json 可共存
- channel 仍然是 Claude 与 AgentNexus 间最自然的结构化通信入口

### 方案 C：必要时加 daemon 侧协议校验

即便使用 `--system-prompt`，Claude 仍不如 Codex `output_schema` 那么硬。

因此仍建议：

- worker 完成但没 `reply()` → 视为协议违规
- lead/coder/reviewer 的完成态必须由 bridge 校验

---

## 11. 最终判断

本次逆向已经确认：

- **Claude CLI 有强 prompt 位点**
- **当前项目只用了较弱的 append 位点**
- **Claude CLI 内建 channel / teammate / sdk 流式链路**
- **你完全可以把这条链路升级成“system-prompt + append-prompt + channel + stream-json”的组合拳**

这不是猜测，而是：

1. 本地安装包反查
2. bundle 关键字命中
3. source map 还原源码
4. 最小运行探测

四层证据叠起来的结论。
