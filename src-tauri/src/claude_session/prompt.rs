use crate::daemon::gui::ClaudeStreamPayload;
use std::{
    io::{Read, Write},
    sync::{Arc, Mutex},
};
use tauri::AppHandle;

const CHANNEL_MARKER: &str = "channels: server:agentnexus";
const LOCAL_DEV_OPTION: &str = "1. i am using this for local development";
const CHANNELS_HINT: &str = "please use --channels to run a list of approved channels.";
const CHANNEL_MARKER_COMPACT: &str = "channels:server:agentnexus";
const LOCAL_DEV_OPTION_COMPACT: &str = "iamusingthisforlocaldevelopment";
const CHANNELS_HINT_COMPACT: &str = "pleaseuse--channelstorunalistofapprovedchannels.";

pub fn spawn_auto_confirm_thread(
    mut reader: Box<dyn Read + Send>,
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    app: AppHandle,
    emit_debug_logs: bool,
) {
    let _ = std::thread::Builder::new()
        .name("claude-pty-watch".into())
        .spawn(move || {
            let mut buf = [0_u8; 1024];
            let mut transcript = String::new();
            let mut pending_log = String::new();
            let mut confirmed = false;
            let mut attention_active = false;
            let mut last_preview = String::new();

            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        let chunk = String::from_utf8_lossy(&buf[..n]);
                        crate::daemon::gui::emit_claude_terminal_data(&app, &chunk);
                        transcript.push_str(&chunk);
                        trim_transcript(&mut transcript, 8192);
                        let dev_prompt = should_auto_confirm_development_prompt(&transcript);
                        if let Some(preview) = extract_terminal_preview(&transcript) {
                            if preview != last_preview {
                                last_preview = preview.clone();
                                crate::daemon::gui::emit_claude_stream(
                                    &app,
                                    ClaudeStreamPayload::Preview { text: preview },
                                );
                            }
                        }
                        if emit_debug_logs {
                            for line in drain_log_lines(&mut pending_log, &chunk) {
                                if !line.is_empty() {
                                    crate::daemon::gui::emit_system_log(
                                        &app,
                                        "info",
                                        &format!("[Claude PTY] {line}"),
                                    );
                                }
                            }
                        }
                        let attention = next_attention_event(
                            attention_active,
                            &transcript,
                            confirmed,
                            dev_prompt,
                        );
                        attention_active = attention.active;
                        if attention.emit {
                            crate::daemon::gui::emit_claude_terminal_attention(&app);
                        }
                        if confirmed || !dev_prompt {
                            continue;
                        }
                        if let Ok(mut tty) = writer.lock() {
                            if tty.write_all(b"1\n").and_then(|_| tty.flush()).is_ok() {
                                confirmed = true;
                                eprintln!(
                                    "[Claude] auto-confirmed local server:agentnexus prompt"
                                );
                                if emit_debug_logs {
                                    crate::daemon::gui::emit_system_log(
                                        &app,
                                        "info",
                                        "[Claude PTY] auto-confirmed local server:agentnexus prompt",
                                    );
                                }
                            }
                        }
                    }
                    Err(_) => break,
                }
            }
        });
}

fn trim_transcript(text: &mut String, keep: usize) {
    let char_len = text.chars().count();
    if char_len <= keep {
        return;
    }
    let drop_chars = char_len - keep;
    let split_idx = text
        .char_indices()
        .nth(drop_chars)
        .map(|(idx, _)| idx)
        .unwrap_or(0);
    *text = text[split_idx..].to_owned();
}

pub fn extract_terminal_preview(output: &str) -> Option<String> {
    super::text_utils::extract_terminal_preview(output)
}

pub fn should_auto_confirm_development_prompt(output: &str) -> bool {
    let normalized = normalize_prompt_text(output);
    let compact = normalize_prompt_compact_text(output);

    let has_hint = normalized.contains(CHANNELS_HINT) || compact.contains(CHANNELS_HINT_COMPACT);
    let has_channel =
        normalized.contains(CHANNEL_MARKER) || compact.contains(CHANNEL_MARKER_COMPACT);
    let has_local_dev =
        normalized.contains(LOCAL_DEV_OPTION) || compact.contains(LOCAL_DEV_OPTION_COMPACT);

    has_hint && has_channel && has_local_dev
}

pub fn drain_log_lines(pending: &mut String, chunk: &str) -> Vec<String> {
    pending.push_str(chunk);
    let normalized = strip_ansi(pending).replace('\r', "\n");
    let mut parts = normalized
        .split('\n')
        .map(str::to_string)
        .collect::<Vec<_>>();
    let tail = parts.pop().unwrap_or_default();
    *pending = tail;
    parts
        .into_iter()
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty())
        .collect()
}

/// Detect interactive prompts that need manual user input.
/// Looks for numbered options, y/n questions, or "?" prompts.
/// Excludes the auto-confirmed development channel prompt.
fn needs_user_attention(transcript: &str) -> bool {
    let clean = strip_ansi(transcript);
    let tail = tail_chars(&clean, 500);
    let last_block = tail
        .lines()
        .rev()
        .skip_while(|line| line.trim().is_empty())
        .take_while(|line| !line.trim().is_empty())
        .collect::<Vec<_>>();
    let recent_window = last_block
        .into_iter()
        .rev()
        .map(str::trim)
        .collect::<Vec<_>>();
    let recent_text = recent_window.join("\n");
    let lower = recent_text.to_ascii_lowercase();

    // Skip only when the most recent prompt window is the dimweave dev confirmation itself.
    if lower.contains("server:agentnexus") && lower.contains("local development") {
        return false;
    }

    let has_options = recent_window.iter().any(|line| {
        let t = line.trim();
        t.starts_with("1.") || t.starts_with("2.") || t.starts_with("1)")
    });
    let has_yn = lower.contains("(y/n)")
        || lower.contains("[y/n]")
        || lower.contains("(yes/no)")
        || lower.contains("[yes/no]");
    let has_question = recent_window.iter().rev().take(3).any(|line| {
        let t = line.trim();
        t.ends_with('?')
    });

    has_options || has_yn || has_question
}

fn should_emit_attention(transcript: &str, confirmed: bool, dev_prompt: bool) -> bool {
    (!confirmed && dev_prompt) || needs_user_attention(transcript)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct AttentionEvent {
    active: bool,
    emit: bool,
}

fn next_attention_event(
    was_active: bool,
    transcript: &str,
    confirmed: bool,
    dev_prompt: bool,
) -> AttentionEvent {
    let active = should_emit_attention(transcript, confirmed, dev_prompt);
    AttentionEvent {
        active,
        emit: active && !was_active,
    }
}

use super::text_utils::{
    normalize_prompt_compact_text, normalize_prompt_text, strip_ansi, tail_chars,
};

#[cfg(test)]
#[path = "prompt_tests.rs"]
mod prompt_tests;
