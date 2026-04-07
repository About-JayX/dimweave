# White Screen Bugfix — React Hooks Violation & Claude History Slug

**日期**: 2026-04-07
**状态**: 已完成

## 目标

修复 production build 随机白屏崩溃问题，同时修复 Claude provider history 在含下划线目录名下为空的问题。

## 根因分析

### Bug 1: 白屏崩溃（React error #300 / "Rendered fewer hooks than expected"）

**现象**: App 启动正常，使用过程中随机白屏，ErrorBoundary 捕获到 React error #300。

**根因**: `ClaudeStreamIndicator` 组件在条件 `return null` 之后调用了 `useMemo`，违反了 React hooks 规则。当 Claude 从 thinking 状态切换到空闲时，hooks 调用数量变化，触发 React 内部校验失败。

**定位过程**:
1. minified production build 只能看到 `Minified React error #300`
2. 通过 vite `define: { "process.env.NODE_ENV": "development" }` + `minify: false` 强制 React dev 模式打包
3. dev 模式显示完整错误: `"Rendered fewer hooks than expected. This may be caused by an accidental early return statement."`
4. 排查定位到 `ClaudeStreamIndicator.tsx` 第 10 行 early return 在 `useMemo` 之前

**问题代码**:
```tsx
// BEFORE (broken)
export function ClaudeStreamIndicator() {
  const thinking = useBridgeStore((s) => s.claudeStream.thinking);
  const previewText = useBridgeStore((s) => s.claudeStream.previewText);
  if (!thinking && !previewText) return null;  // ← early return
  const displayText = useMemo(...);            // ← hook after return = violation
```

**修复**: 将 `useMemo` 移到 early return 之前。

**全量审计**: 扫描了所有 60+ tsx 文件，确认无其他 hooks 违规。

### Bug 2: Claude History 下拉为空

**现象**: 含下划线的 workspace 目录名（如 `ipa_pj`）下 Claude History 下拉只有 "New session"。

**根因**: Claude CLI 生成项目目录 slug 时将 `_` 转为 `-`（`ipa_pj` → `ipa-pj`），但 daemon 的 `workspace_history_dir` 只转换 `/\:`，不转 `_`，导致 slug 不匹配。

**修复**: slug 转换增加 `'_' => '-'`。

## 文件映射

| 文件 | 操作 | 说明 |
|------|------|------|
| `src/components/MessagePanel/ClaudeStreamIndicator.tsx` | 修改 | hooks 移到 early return 之前 |
| `src/components/ErrorBoundary.tsx` | 新增 | 错误写入 log tab + 自动恢复，不弹独立 UI |
| `src/main.tsx` | 修改 | 包裹 ErrorBoundary |
| `src-tauri/src/daemon/provider/claude.rs` | 修改 | slug 增加 `_` → `-` 转换 |
| `src-tauri/Cargo.toml` | 修改 | 移除临时 devtools feature |
| `src-tauri/src/main.rs` | 修改 | 移除临时 open_devtools 调用 |
| `vite.config.ts` | 还原 | 移除临时 React dev mode 配置 |

## CM Memory

| Task | Commit | Verification |
|------|--------|--------------|
| 修复 hooks 违规 + slug bug + ErrorBoundary + 清理 devtools | `9c84b9d9` | `cargo test` 310 pass; `bun run build` pass; `bun test` 188 pass; 全量 hooks 审计通过 |

## 验收标准

- `cargo test` 全通过
- `bun run build` 通过
- Claude thinking 状态切换不再白屏
- 含下划线目录的 Claude history 正常显示
- ErrorBoundary 错误写入 log tab，不弹独立页面
- 无 devtools 残留在 production build
