---
paths:
  - "src-tauri/**"
---

# Tauri 开发规范

- Tauri 2，最小化 Rust 代码，仅负责窗口壳
- daemon 作为独立 Bun 进程运行，不嵌入 Tauri sidecar
