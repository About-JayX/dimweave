# Clipboard Paste Attachments Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `Cmd/Ctrl+V` support for image and file attachments in the reply composer without changing the existing path-based attachment/send pipeline.

**Architecture:** Keep clipboard ingestion behind a new Rust-side Tauri command that converts clipboard files/images into local file paths. Reuse the existing `addFiles(paths)` attachment entrypoint in the frontend so picker, drag/drop, and paste all converge before any provider-specific logic runs.

**Tech Stack:** React 19, TypeScript, Zustand, Tauri 2, Rust, tokio, tauri-plugin-clipboard, Bun, Cargo

---

## Baseline Notes

- `bun test` is **not** fully green on this branch before this feature starts: the unrelated `BackToBottomButton` tests already fail in `src/components/MessagePanel/presentational.test.tsx`.
- `cargo test --manifest-path src-tauri/Cargo.toml` is green **after** `cargo build --manifest-path bridge/Cargo.toml` produces `target/debug/dimweave-bridge`.

Do not expand this feature to repair those unrelated baseline issues. Use the targeted verification commands below plus the known-green Rust suite/build commands.

## File Map

### Clipboard ingress

- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/src/main.rs`
- Create: `src-tauri/src/paste_attachments.rs`

### Frontend paste wiring

- Modify: `src/components/ReplyInput/index.tsx`
- Create: `src/components/ReplyInput/paste-attachments.ts`
- Create: `src/components/ReplyInput/paste-attachments.test.ts`

## CM Memory

| Task | Planned commit message | Verification | Memory |
|------|------------------------|--------------|--------|
| Task 1 | `feat: ingest pasted clipboard attachments` | `cargo build --manifest-path bridge/Cargo.toml`; `cargo test --manifest-path src-tauri/Cargo.toml paste_attachments::tests::`; `cargo test --manifest-path src-tauri/Cargo.toml` | Clipboard-specific complexity belongs in one Rust ingress layer; the rest of the app should still only see local file paths. |
| Task 2 | `feat: wire composer paste into attachment flow` | `bun test src/components/ReplyInput/paste-attachments.test.ts src/components/ReplyInput/index.test.tsx`; `bun run build`; `git diff --check` | Paste should be a third attachment source, not a parallel attachment model; text paste must keep its native behavior. |

## Task 1: Add a Rust clipboard-to-path ingress command

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/src/main.rs`
- Create: `src-tauri/src/paste_attachments.rs`

- [ ] **Step 1: Write the failing Rust tests first**

Create `src-tauri/src/paste_attachments.rs` with these tests before the implementation:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "dimweave-paste-attachments-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn normalize_clipboard_paths_keeps_only_existing_non_blank_paths() {
        let dir = temp_dir("normalize");
        let keep = dir.join("keep.txt");
        std::fs::write(&keep, "ok").unwrap();

        let result = normalize_clipboard_paths(vec![
            keep.to_string_lossy().to_string(),
            String::new(),
            "   ".into(),
            dir.join("missing.txt").to_string_lossy().to_string(),
        ]);

        assert_eq!(result, vec![keep.to_string_lossy().to_string()]);
    }

    #[test]
    fn persist_pasted_png_writes_a_real_png_file() {
        let dir = temp_dir("png");
        let png = {
            let mut out = std::io::Cursor::new(Vec::new());
            image::DynamicImage::new_rgba8(1, 1)
                .write_to(&mut out, image::ImageFormat::Png)
                .unwrap();
            out.into_inner()
        };

        let path = persist_pasted_png(&dir, &png).unwrap();
        assert!(path.ends_with(".png"));
        assert!(std::path::Path::new(&path).exists());
        assert_eq!(std::fs::read(path).unwrap(), png);
    }
}
```

- [ ] **Step 2: Run the targeted Rust tests to verify they fail**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml paste_attachments::tests::
```

Expected: FAIL because the new module helpers do not exist yet.

- [ ] **Step 3: Implement the clipboard ingress layer**

Add the Rust dependency:

```toml
tauri-plugin-clipboard = "2.1.11"
```

Initialize the plugin in `src-tauri/src/main.rs` and register the new command:

```rust
mod paste_attachments;

fn main() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard::init())
        // ...
        .invoke_handler(tauri::generate_handler![
            // ...
            paste_attachments::read_paste_attachments,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application");
}
```

Implement `src-tauri/src/paste_attachments.rs` so clipboard files and images become local paths:

```rust
use tauri::{AppHandle, Manager, Runtime, State};

fn normalize_clipboard_paths(paths: Vec<String>) -> Vec<String> {
    paths
        .into_iter()
        .map(|path| path.trim().to_string())
        .filter(|path| !path.is_empty())
        .filter(|path| std::path::Path::new(path).exists())
        .collect()
}

fn paste_cache_dir<R: Runtime>(app: &AppHandle<R>) -> Result<std::path::PathBuf, String> {
    let dir = app
        .path()
        .app_cache_dir()
        .map_err(|err| err.to_string())?
        .join("pasted-attachments")
        .join(std::process::id().to_string());
    std::fs::create_dir_all(&dir).map_err(|err| err.to_string())?;
    Ok(dir)
}

fn persist_pasted_png(dir: &std::path::Path, png: &[u8]) -> Result<String, String> {
    let path = dir.join(format!("{}.png", uuid::Uuid::new_v4()));
    std::fs::write(&path, png).map_err(|err| err.to_string())?;
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn read_paste_attachments<R: Runtime>(
    app: AppHandle<R>,
    clipboard: State<'_, tauri_plugin_clipboard::Clipboard>,
) -> Result<Vec<String>, String> {
    let mut paths = Vec::new();

    if clipboard.has_files()? {
        paths.extend(normalize_clipboard_paths(clipboard.read_files()?));
    }

    if clipboard.has_image()? {
        let cache_dir = paste_cache_dir(&app)?;
        let png = clipboard.read_image_binary()?;
        paths.push(persist_pasted_png(&cache_dir, &png)?);
    }

    Ok(paths)
}
```

- [ ] **Step 4: Run Rust verification**

Run:

```bash
cargo build --manifest-path bridge/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml paste_attachments::tests::
cargo test --manifest-path src-tauri/Cargo.toml
git diff --check
```

Expected: PASS. The bridge binary must exist before the full Tauri Rust test suite runs.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/src/main.rs src-tauri/src/paste_attachments.rs
git commit -m "feat: ingest pasted clipboard attachments"
```

- [ ] **Step 6: Update `## CM Memory` with the real commit SHA after review**

## Task 2: Trigger clipboard ingestion from the reply composer without blocking text paste

**Files:**
- Modify: `src/components/ReplyInput/index.tsx`
- Create: `src/components/ReplyInput/paste-attachments.ts`
- Create: `src/components/ReplyInput/paste-attachments.test.ts`

- [ ] **Step 1: Write the failing TypeScript tests first**

Create `src/components/ReplyInput/paste-attachments.test.ts` with these tests:

```ts
import { describe, expect, mock, test } from "bun:test";
import { collectPastedAttachmentPaths } from "./paste-attachments";

describe("collectPastedAttachmentPaths", () => {
  test("returns the backend-provided paths unchanged", async () => {
    const result = await collectPastedAttachmentPaths(async () => [
      "/tmp/clip.png",
      "/tmp/spec.md",
    ]);

    expect(result).toEqual(["/tmp/clip.png", "/tmp/spec.md"]);
  });

  test("swallows backend failures so normal text paste can continue", async () => {
    const oldError = console.error;
    const errorSpy = mock(() => {});
    console.error = errorSpy;

    try {
      const result = await collectPastedAttachmentPaths(async () => {
        throw new Error("clipboard unavailable");
      });

      expect(result).toEqual([]);
      expect(errorSpy).toHaveBeenCalled();
    } finally {
      console.error = oldError;
    }
  });
});
```

- [ ] **Step 2: Run the targeted TypeScript tests to verify they fail**

Run:

```bash
bun test src/components/ReplyInput/paste-attachments.test.ts
```

Expected: FAIL because the helper file does not exist yet.

- [ ] **Step 3: Implement the frontend paste helper and wire it into `ReplyInput`**

Create `src/components/ReplyInput/paste-attachments.ts`:

```ts
import { invoke } from "@tauri-apps/api/core";

export async function collectPastedAttachmentPaths(
  readPasteAttachments: () => Promise<string[]> = () =>
    invoke<string[]>("read_paste_attachments"),
): Promise<string[]> {
  try {
    return (await readPasteAttachments()).filter((path) => path.trim().length > 0);
  } catch (error) {
    console.error("[ReplyInput] paste attachments failed", error);
    return [];
  }
}
```

Wire it into `src/components/ReplyInput/index.tsx` without calling `preventDefault()`:

```tsx
import { collectPastedAttachmentPaths } from "./paste-attachments";

const handlePaste = useCallback(() => {
  void collectPastedAttachmentPaths().then((paths) => {
    if (paths.length > 0) addFiles(paths);
  });
}, [addFiles]);

<textarea
  ref={textareaRef}
  value={draft}
  onChange={(e) => setDraft(e.target.value)}
  onPaste={handlePaste}
  onKeyDown={handleKeyDown}
  // ...
/>
```

Do **not** parse clipboard files in the browser event itself and do **not** call `preventDefault()`. The browser should keep handling text paste, while the async command appends attachment paths in parallel.

- [ ] **Step 4: Run frontend verification**

Run:

```bash
bun test src/components/ReplyInput/paste-attachments.test.ts src/components/ReplyInput/index.test.tsx
bun run build
git diff --check
```

Expected: PASS. The new helper tests should be green, the existing ReplyInput tests should stay green, and the app should still build cleanly.

- [ ] **Step 5: Commit**

```bash
git add src/components/ReplyInput/index.tsx src/components/ReplyInput/paste-attachments.ts src/components/ReplyInput/paste-attachments.test.ts
git commit -m "feat: wire composer paste into attachment flow"
```

- [ ] **Step 6: Update `## CM Memory` with the real commit SHA after review**
