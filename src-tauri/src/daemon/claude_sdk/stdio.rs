use tokio::{
    io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::{ChildStderr, ChildStdout},
};

/// Drain Claude stdio so long-running sessions do not block on full pipe buffers.
/// stderr lines are written to /tmp/claude-stderr.log for diagnostics.
pub fn spawn_stdio_drainers(stdout: Option<ChildStdout>, stderr: Option<ChildStderr>) {
    if let Some(stdout) = stdout {
        tokio::spawn(async move {
            let mut stdout = stdout;
            let mut sink = io::sink();
            let _ = io::copy(&mut stdout, &mut sink).await;
        });
    }
    if let Some(stderr) = stderr {
        tokio::spawn(async move {
            let mut lines = BufReader::new(stderr).lines();
            let mut file = tokio::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open("/tmp/claude-stderr.log")
                .await
                .ok();
            while let Ok(Some(line)) = lines.next_line().await {
                if let Some(ref mut f) = file {
                    let _ = f.write_all(format!("{line}\n").as_bytes()).await;
                }
            }
        });
    }
}
