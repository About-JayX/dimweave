# Clipboard Paste Attachments Design

## Summary

Dimweave already supports attachment entry through the paperclip picker and Tauri drag/drop, but the composer still has no `paste` path. The current attachment pipeline is path-based from the moment files enter `useAttachments()`: frontend state stores local file paths, Codex image delivery reads `localImage` paths, and Claude image delivery reads/compresses local files before building base64 image blocks.

The new feature should add a third entry path - clipboard paste - without changing the existing attachment/send protocol. The user experience target is:

- text keeps pasting into the textarea normally
- pasted screenshots/images appear in the same attachment strip as uploaded files
- copied files from Finder / Explorer paste into the same attachment strip
- sending still reuses the current Claude/Codex attachment formatting logic

## Product Goal

- Make `Cmd/Ctrl+V` support both images and files in the reply composer.
- Keep pasted attachments on the exact same preview/remove/send path as picker and drag/drop attachments.
- Avoid redesigning the attachment model or provider delivery protocol.

## Scope

### Included

- `ReplyInput` paste handling that does not block normal text paste.
- A Tauri clipboard adapter that can read clipboard files and images.
- Persisting pasted images to a local cache path so the existing path-based attachment pipeline can reuse them unchanged.
- Regression tests for the new clipboard adapter and the frontend paste helper.

### Excluded

- Reworking `Attachment` or the daemon message schema.
- New upload progress UI or clipboard-specific toast notifications.
- Automatic cache pruning policy beyond writing into an app-owned cache location.
- Changing Claude/Codex delivery formats after attachments are created.

## Evidence and Constraints

### Current codebase constraints

- `ReplyInput` currently wires only `pick_files` and `getCurrentWebview().onDragDropEvent()`; there is no `onPaste` handling in `src/components/ReplyInput/index.tsx`.
- `useAttachments()` accepts local paths and derives `fileName`, `isImage`, and `mediaType` from those paths in `src/components/ReplyInput/use-attachments.ts`.
- Provider delivery already assumes local paths:
  - Codex builds `localImage` items from `Attachment.file_path` in `src-tauri/src/daemon/routing_format.rs`
  - Claude reads/compresses image files from `Attachment.file_path` in the same module

### Clipboard capability evidence

- MDN documents that `ClipboardEvent.clipboardData` uses `DataTransfer`, but that browser-level path exposure is not sufficient evidence for system file-copy paste across desktop shells: <https://developer.mozilla.org/en-US/docs/Web/API/DataTransfer>
- `tauri-plugin-clipboard` v2.1.11 exposes `has_files`, `read_files`, `has_image`, and `read_image_binary`, and `read_image_binary` materializes clipboard images as PNG bytes in its desktop implementation.
- Tauri 2 exposes `app.path().app_cache_dir()` via the manager path resolver, which resolves to an app-owned cache directory on desktop.

### Claude reference

- Anthropic's 2025 Claude Code usage PDF explicitly says design teams "Command+V" screenshots directly into Claude Code: <https://www-cdn.anthropic.com/58284b19e702b49db9302d5b6f135ad8871e7658.pdf>
- Local Claude Code v2.1.89 code shows the same architectural pattern we want to borrow:
  - pasted images are written into a local cache directory before reuse
  - file attachments are resolved to local paths before being prepended into the prompt

We should copy that **local materialization** idea, but keep Dimweave's cleaner `attachments` field instead of re-inlining everything into prompt text.

## Design

### 1. Add a Rust-side clipboard ingress layer

Add a new `src-tauri/src/paste_attachments.rs` module that owns clipboard-to-path conversion.

Responsibilities:

- read file paths from the system clipboard through `tauri_plugin_clipboard::Clipboard`
- filter blank / missing paths so frontend attachment state only receives valid files
- read clipboard image bytes and persist them as PNG files inside `app.path().app_cache_dir()/pasted-attachments/<pid>/`
- return a `Vec<String>` of local paths to the frontend

This keeps clipboard/platform differences isolated on the Rust side and lets the frontend stay path-driven.

### 2. Keep the frontend attachment contract path-based

`ReplyInput` should treat pasted attachments exactly like picker and drag/drop attachments:

- on `paste`, do **not** call `preventDefault()`
- asynchronously invoke the new Tauri command
- if it returns paths, forward them to `addFiles(paths)`
- if it returns an empty list, do nothing
- if it errors, log the failure and leave the text paste untouched

No `Attachment` shape changes are needed because the frontend already knows how to derive preview metadata from file paths.

### 3. Use the clipboard plugin only behind our own command

Do not expose the plugin's JS API directly in the composer.

Instead:

- add `tauri-plugin-clipboard = "2.1.11"` only on the Rust side
- initialize it in `src-tauri/src/main.rs`
- read the managed clipboard state from our own `read_paste_attachments` command

This is the best-maintained route because:

- there is one app-specific command boundary for clipboard ingress
- frontend code does not learn plugin-specific commands or permissions
- the rest of the app continues to think in local file paths

### 4. Do not change provider formatting

Once clipboard attachments become local file paths, the existing downstream behavior stays correct:

- `AttachmentStrip` previews pasted PNGs the same way it previews dragged image files
- `MessageBubble` keeps rendering image/file attachments from `BridgeMessage.attachments`
- Codex still receives pasted images as `localImage`
- Claude still receives pasted images as compressed base64 image blocks

The feature should stop at ingress; it should not reopen the already-working delivery pipeline.

## File Map

### Rust / Tauri

- `src-tauri/Cargo.toml`
- `src-tauri/src/main.rs`
- `src-tauri/src/paste_attachments.rs` (new)

### Frontend

- `src/components/ReplyInput/index.tsx`
- `src/components/ReplyInput/paste-attachments.ts` (new)
- `src/components/ReplyInput/paste-attachments.test.ts` (new)

## Testing Strategy

- Rust unit tests for path normalization and PNG persistence in `paste_attachments.rs`
- TypeScript unit tests for the paste helper's success/error behavior
- `bun run build` to ensure the new `invoke` path and ReplyInput wiring type-check
- full `cargo test --manifest-path src-tauri/Cargo.toml` after building the bridge binary, because that suite is green at baseline once `cargo build --manifest-path bridge/Cargo.toml` has run

## Acceptance Criteria

- Pasting plain text still inserts text into the textarea exactly as before.
- Pasting a screenshot/image adds an image chip to the attachment strip without blocking text paste.
- Copying files in the OS file manager and pasting into the composer adds those files to the same attachment strip.
- Pasted attachments can be removed before send and follow the existing send path unchanged.
- No provider-specific attachment regressions are introduced.
