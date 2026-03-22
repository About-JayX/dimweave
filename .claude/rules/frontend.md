---
paths:
  - "src/**/*.{ts,tsx}"
---

# 前端开发规范

- React 19 + 纯 inline styles（暗色主题，背景 #0a0a0a）
- 不使用 CSS 框架或外部 UI 库
- 状态通过 `useWebSocket` hook 集中管理，组件只接收 props
- WebSocket 自动重连，间隔 3 秒
- TypeScript 配置用 tsconfig.app.json
