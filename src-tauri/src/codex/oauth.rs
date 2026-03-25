use serde::Serialize;
use std::{
    collections::VecDeque,
    process::ExitStatus,
    sync::{Arc, Mutex},
};
use tokio::{
    io::{AsyncBufReadExt, AsyncRead, BufReader},
    process::Command,
    sync::{mpsc, oneshot},
    time::{timeout, Duration, Instant},
};

use std::process::Stdio;

// ── Types ────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OAuthLaunchInfo {
    pub verification_uri: Option<String>,
}

/// Shared state to allow cancellation from frontend.
pub struct OAuthHandle {
    cancel_tx: Mutex<Option<oneshot::Sender<()>>>,
}

impl OAuthHandle {
    pub fn new() -> Self {
        Self {
            cancel_tx: Mutex::new(None),
        }
    }

    pub fn cancel(&self) -> bool {
        if let Some(tx) = self.cancel_tx.lock().unwrap().take() {
            let _ = tx.send(());
            true
        } else {
            false
        }
    }
}

// ── Helpers ──────────────────────────────────────────────

fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            if chars.peek() == Some(&'[') {
                chars.next();
                for ch in chars.by_ref() {
                    if ch.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn parse_verification_uri(line: &str) -> Option<String> {
    line.split_whitespace().find_map(|token| {
        let t = token.trim_matches(|c: char| {
            !c.is_ascii_alphanumeric() && c != ':' && c != '/' && c != '.' && c != '-'
        });
        if t.starts_with("https://") || t.starts_with("http://") {
            Some(t.to_string())
        } else {
            None
        }
    })
}

fn find_codex() -> Result<std::path::PathBuf, String> {
    let candidates = ["/Applications/Codex.app/Contents/Resources/codex"];
    for c in &candidates {
        let p = std::path::Path::new(c);
        if p.exists() {
            return Ok(p.to_path_buf());
        }
    }
    if let Some(home) = dirs::home_dir() {
        let p = home.join("Applications/Codex.app/Contents/Resources/codex");
        if p.exists() {
            return Ok(p);
        }
    }
    which::which("codex").map_err(|_| "Codex CLI not found. Please install Codex.".to_string())
}

#[derive(Debug)]
enum StreamEvent {
    Line { stream: &'static str, line: String },
    Closed { stream: &'static str },
}

struct LoginState {
    verification_uri: Option<String>,
    stdout_closed: bool,
    stderr_closed: bool,
    recent_output: VecDeque<String>,
}

impl LoginState {
    fn new() -> Self {
        Self {
            verification_uri: None,
            stdout_closed: false,
            stderr_closed: false,
            recent_output: VecDeque::new(),
        }
    }

    fn apply(&mut self, event: StreamEvent) {
        match event {
            StreamEvent::Line { stream, line } => {
                let clean = strip_ansi(&line).trim().to_string();
                self.recent_output.push_back(format!("[{stream}] {clean}"));
                while self.recent_output.len() > 24 {
                    self.recent_output.pop_front();
                }
                if self.verification_uri.is_none() {
                    self.verification_uri = parse_verification_uri(&clean);
                }
            }
            StreamEvent::Closed { stream } => match stream {
                "stdout" => self.stdout_closed = true,
                "stderr" => self.stderr_closed = true,
                _ => {}
            },
        }
    }

    fn all_closed(&self) -> bool {
        self.stdout_closed && self.stderr_closed
    }
}

async fn pump_stream<R>(reader: R, stream: &'static str, tx: mpsc::UnboundedSender<StreamEvent>)
where
    R: AsyncRead + Unpin,
{
    let mut lines = BufReader::new(reader).lines();
    loop {
        match lines.next_line().await {
            Ok(Some(line)) => {
                let _ = tx.send(StreamEvent::Line { stream, line });
            }
            Ok(None) => {
                let _ = tx.send(StreamEvent::Closed { stream });
                break;
            }
            Err(_) => {
                let _ = tx.send(StreamEvent::Closed { stream });
                break;
            }
        }
    }
}

fn format_failure(status: ExitStatus, state: &LoginState) -> String {
    let mut detail = format!(
        "codex login exited with code {}",
        status.code().unwrap_or(-1)
    );
    if !state.recent_output.is_empty() {
        detail.push_str("\nRecent output:\n");
        for line in &state.recent_output {
            detail.push_str(line);
            detail.push('\n');
        }
    }
    detail
}

// ── Public API ───────────────────────────────────────────

/// Launch `codex login` and return verification URL + completion future.
pub async fn start_login(
    handle: Arc<OAuthHandle>,
) -> Result<OAuthLaunchInfo, String> {
    let bin = find_codex()?;

    let mut child = Command::new(&bin)
        .arg("login")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("failed to spawn codex login: {e}"))?;

    let stdout = child.stdout.take().ok_or("no stdout pipe")?;
    let stderr = child.stderr.take().ok_or("no stderr pipe")?;
    let (tx, mut rx) = mpsc::unbounded_channel::<StreamEvent>();

    tokio::spawn(pump_stream(stdout, "stdout", tx.clone()));
    tokio::spawn(pump_stream(stderr, "stderr", tx));

    let mut state = LoginState::new();
    let launch_deadline = Instant::now() + Duration::from_secs(3);

    // Wait briefly for verification URI
    while state.verification_uri.is_none() {
        let remaining = launch_deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            break;
        }
        match timeout(remaining, rx.recv()).await {
            Ok(Some(event)) => state.apply(event),
            _ => break,
        }
        if state.all_closed() {
            break;
        }
    }

    let info = OAuthLaunchInfo {
        verification_uri: state.verification_uri.clone(),
    };

    // Set up cancellation
    let (cancel_tx, cancel_rx) = oneshot::channel::<()>();
    *handle.cancel_tx.lock().unwrap() = Some(cancel_tx);

    // Spawn background task to wait for login completion
    tokio::spawn(async move {
        tokio::pin!(cancel_rx);
        loop {
            tokio::select! {
                _ = &mut cancel_rx => {
                    let _ = child.kill().await;
                    return;
                }
                maybe_event = rx.recv() => match maybe_event {
                    Some(event) => state.apply(event),
                    None => break,
                }
            }
        }
        let _ = child.wait().await;
    });

    Ok(info)
}

/// Launch `codex logout`.
pub async fn do_logout() -> Result<(), String> {
    let bin = find_codex()?;
    let status = Command::new(&bin)
        .arg("logout")
        .status()
        .await
        .map_err(|e| format!("failed to run codex logout: {e}"))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "codex logout exited with code {}",
            status.code().unwrap_or(-1)
        ))
    }
}
