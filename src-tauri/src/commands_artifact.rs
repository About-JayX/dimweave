use crate::daemon::{types::TaskSnapshot, DaemonCmd};
use crate::DaemonSender;
use serde::Serialize;
use tauri::State;
use tokio::io::AsyncReadExt;

const MAX_PREVIEW_BYTES: usize = 65_536;

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactDetailPayload {
    pub reference: String,
    pub file_name: Option<String>,
    pub exists: bool,
    pub preview: Option<String>,
    pub truncated: bool,
}

fn artifact_visible_in_snapshot(snapshot: Option<&TaskSnapshot>, content_ref: &str) -> bool {
    snapshot.is_some_and(|task| {
        task.artifacts
            .iter()
            .any(|artifact| artifact.content_ref == content_ref)
    })
}

fn preview_text(bytes: &[u8]) -> Option<String> {
    match std::str::from_utf8(bytes) {
        Ok(text) => Some(text.to_string()),
        Err(error) if error.valid_up_to() > 0 => {
            std::str::from_utf8(&bytes[..error.valid_up_to()])
                .ok()
                .map(|text| text.to_string())
        }
        Err(_) => None,
    }
}

fn preview_from_bytes(bytes: &[u8]) -> (Option<String>, bool) {
    let truncated = bytes.len() > MAX_PREVIEW_BYTES;
    let preview_bytes = if truncated {
        &bytes[..MAX_PREVIEW_BYTES]
    } else {
        bytes
    };
    let preview = preview_text(preview_bytes);
    (preview, truncated)
}

async fn active_task_snapshot(sender: &State<'_, DaemonSender>) -> Result<Option<TaskSnapshot>, String> {
    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
    sender
        .0
        .send(DaemonCmd::GetTaskSnapshot { reply: reply_tx })
        .await
        .map_err(|e| e.to_string())?;
    reply_rx
        .await
        .map_err(|_| "daemon dropped task_snapshot reply".to_string())
}

#[tauri::command]
pub async fn daemon_get_artifact_detail(
    content_ref: String,
    sender: State<'_, DaemonSender>,
) -> Result<ArtifactDetailPayload, String> {
    let snapshot = active_task_snapshot(&sender).await?;
    if !artifact_visible_in_snapshot(snapshot.as_ref(), &content_ref) {
        return Err("Artifact is not available in the active task context.".to_string());
    }

    let path = std::path::PathBuf::from(&content_ref);
    let file_name = path.file_name().map(|name| name.to_string_lossy().into_owned());
    let is_file = tokio::fs::metadata(&path)
        .await
        .map(|metadata| metadata.is_file())
        .unwrap_or(false);
    if !is_file {
        return Ok(ArtifactDetailPayload {
            reference: content_ref,
            file_name,
            exists: false,
            preview: None,
            truncated: false,
        });
    }

    let file = tokio::fs::File::open(&path)
        .await
        .map_err(|error| format!("Unable to read artifact: {error}"))?;
    let mut limited = file.take((MAX_PREVIEW_BYTES + 1) as u64);
    let mut bytes = Vec::with_capacity(MAX_PREVIEW_BYTES + 1);
    limited
        .read_to_end(&mut bytes)
        .await
        .map_err(|error| format!("Unable to read artifact: {error}"))?;
    let (preview, truncated) = preview_from_bytes(&bytes);

    Ok(ArtifactDetailPayload {
        reference: content_ref,
        file_name,
        exists: true,
        preview,
        truncated,
    })
}

#[cfg(test)]
mod tests {
    use super::{artifact_visible_in_snapshot, preview_from_bytes, MAX_PREVIEW_BYTES};
    use crate::daemon::{
        task_graph::types::{Artifact, ArtifactKind, Provider, Task, TaskStatus},
        types::TaskSnapshot,
    };

    fn snapshot_with_artifact(content_ref: &str) -> TaskSnapshot {
        TaskSnapshot {
            task: Task {
                task_id: "task_1".to_string(),
                workspace_root: "/workspace".to_string(),
                title: "Task".to_string(),
                status: TaskStatus::Draft,
                lead_session_id: None,
                current_coder_session_id: None,
                lead_provider: Provider::Claude,
                coder_provider: Provider::Codex,
                created_at: 1,
                updated_at: 1,
            },
            sessions: Vec::new(),
            artifacts: vec![Artifact {
                artifact_id: "artifact_1".to_string(),
                task_id: "task_1".to_string(),
                session_id: "session_1".to_string(),
                kind: ArtifactKind::Diff,
                title: "Patch".to_string(),
                content_ref: content_ref.to_string(),
                created_at: 1,
            }],
            provider_summary: None,
        }
    }

    #[test]
    fn artifact_detail_access_requires_active_task_reference() {
        let snapshot = snapshot_with_artifact("/tmp/patch.diff");
        assert!(artifact_visible_in_snapshot(Some(&snapshot), "/tmp/patch.diff"));
        assert!(!artifact_visible_in_snapshot(Some(&snapshot), "/tmp/other.diff"));
        assert!(!artifact_visible_in_snapshot(None, "/tmp/patch.diff"));
    }

    #[test]
    fn preview_builder_truncates_and_skips_binary_payloads() {
        let (preview, truncated) = preview_from_bytes(&vec![b'a'; MAX_PREVIEW_BYTES + 4]);
        assert_eq!(preview, Some("a".repeat(MAX_PREVIEW_BYTES)));
        assert!(truncated);

        let (binary_preview, binary_truncated) = preview_from_bytes(&[0xff, 0xfe, 0xfd]);
        assert_eq!(binary_preview, None);
        assert!(!binary_truncated);
    }

    #[test]
    fn preview_builder_preserves_valid_utf8_prefix_when_truncation_splits_a_char() {
        let content = format!("{}{}", "a".repeat(MAX_PREVIEW_BYTES - 1), "界");
        let (preview, truncated) = preview_from_bytes(content.as_bytes());
        assert_eq!(preview, Some("a".repeat(MAX_PREVIEW_BYTES - 1)));
        assert!(truncated);
    }
}
