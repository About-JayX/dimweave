use tauri::{AppHandle, Manager, Runtime};

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
    clipboard: tauri::State<'_, tauri_plugin_clipboard::Clipboard>,
) -> Result<Vec<String>, String> {
    let mut paths = Vec::new();

    if clipboard.has_files().map_err(|e| e.to_string())? {
        paths.extend(normalize_clipboard_paths(
            clipboard.read_files().map_err(|e| e.to_string())?,
        ));
    }

    if clipboard.has_image().map_err(|e| e.to_string())? {
        let cache_dir = paste_cache_dir(&app)?;
        let png = clipboard
            .read_image_binary()
            .map_err(|e| e.to_string())?;
        paths.push(persist_pasted_png(&cache_dir, &png)?);
    }

    Ok(paths)
}

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
