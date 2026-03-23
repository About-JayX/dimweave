use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::thread;
use tauri::{AppHandle, Emitter};

struct PtyState {
    writer: Box<dyn Write + Send>,
    pair: portable_pty::PtyPair,
    child: Option<Box<dyn portable_pty::Child + Send>>,
}

static PTY_STATE: std::sync::OnceLock<Arc<Mutex<Option<PtyState>>>> = std::sync::OnceLock::new();

fn get_state() -> &'static Arc<Mutex<Option<PtyState>>> {
    PTY_STATE.get_or_init(|| Arc::new(Mutex::new(None)))
}

#[tauri::command]
pub fn launch_pty(
    app: AppHandle,
    cwd: String,
    cols: u16,
    rows: u16,
    role_id: String,
    agents_json: String,
) -> Result<(), String> {
    let state = get_state();
    {
        let guard = state.lock().unwrap();
        if guard.is_some() {
            return Err("PTY already running".into());
        }
    }

    let pty_system = native_pty_system();
    let pair = pty_system
        .openpty(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| format!("Failed to open PTY: {e}"))?;

    // Support CLAUDE_PATH env var for custom Claude binary location
    let claude_bin = std::env::var("CLAUDE_PATH").unwrap_or_else(|_| "claude".to_string());
    let mut cmd = CommandBuilder::new(&claude_bin);
    cmd.arg("--dangerously-skip-permissions");
    // Ensure MCP servers (agentbridge) are loaded
    let home = dirs::home_dir().unwrap_or_default();
    let mcp_config = home.join(".claude").join("mcp.json");
    if mcp_config.exists() {
        cmd.arg("--mcp-config");
        cmd.arg(mcp_config.to_string_lossy().as_ref());
    }
    if !role_id.is_empty() && !agents_json.is_empty() {
        cmd.arg("--agent");
        cmd.arg(&role_id);
        cmd.arg("--agents");
        cmd.arg(&agents_json);
    }
    cmd.cwd(&cwd);

    let child = pair
        .slave
        .spawn_command(cmd)
        .map_err(|e| format!("Failed to spawn claude: {e}"))?;

    let writer = pair
        .master
        .take_writer()
        .map_err(|e| format!("Failed to get PTY writer: {e}"))?;

    // Read PTY output in background thread -> emit to frontend
    let mut reader = pair
        .master
        .try_clone_reader()
        .map_err(|e| format!("Failed to get PTY reader: {e}"))?;

    let app_handle = app.clone();
    thread::spawn(move || {
        let mut buf = [0u8; 4096];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    let data = String::from_utf8_lossy(&buf[..n]).to_string();
                    let _ = app_handle.emit("pty-data", &data);
                }
                Err(_) => break,
            }
        }
    });

    // Store state (child stays in PtyState for proper lifecycle management)
    {
        let mut guard = state.lock().unwrap();
        *guard = Some(PtyState {
            writer,
            pair,
            child: Some(child),
        });
    }

    // Monitor child exit in background
    let app_exit = app.clone();
    let state_exit = state.clone();
    thread::spawn(move || {
        // Wait for child to exit by polling
        loop {
            let should_wait = {
                let guard = state_exit.lock().unwrap();
                guard.is_some()
            };
            if !should_wait {
                break;
            }

            // Try to wait on the child
            let exit_code = {
                let mut guard = state_exit.lock().unwrap();
                if let Some(ref mut pty_state) = *guard {
                    if let Some(ref mut child) = pty_state.child {
                        // Try non-blocking wait
                        match child.try_wait() {
                            Ok(Some(status)) => Some(status.exit_code()),
                            Ok(None) => None, // Still running
                            Err(_) => Some(1),
                        }
                    } else {
                        // Child already taken (stopped externally)
                        break;
                    }
                } else {
                    break;
                }
            };

            if let Some(code) = exit_code {
                let _ = app_exit.emit("pty-exit", code);
                let mut guard = state_exit.lock().unwrap();
                *guard = None;
                break;
            }

            thread::sleep(std::time::Duration::from_millis(200));
        }
    });

    Ok(())
}

#[tauri::command]
pub fn pty_write(data: String) -> Result<(), String> {
    let state = get_state();
    let mut guard = state.lock().unwrap();
    if let Some(ref mut pty) = *guard {
        pty.writer
            .write_all(data.as_bytes())
            .map_err(|e| format!("Write failed: {e}"))?;
        pty.writer.flush().map_err(|e| format!("Flush failed: {e}"))?;
        Ok(())
    } else {
        Err("PTY not running".into())
    }
}

#[tauri::command]
pub fn pty_resize(cols: u16, rows: u16) -> Result<(), String> {
    let state = get_state();
    let guard = state.lock().unwrap();
    if let Some(ref pty) = *guard {
        pty.pair
            .master
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| format!("Resize failed: {e}"))?;
        Ok(())
    } else {
        Err("PTY not running".into())
    }
}

#[tauri::command]
pub fn pty_is_running() -> bool {
    let state = get_state();
    let guard = state.lock().unwrap();
    guard.is_some()
}

#[tauri::command]
pub fn stop_pty() -> Result<(), String> {
    let state = get_state();
    let mut guard = state.lock().unwrap();
    if let Some(mut pty_state) = guard.take() {
        // Kill child process explicitly
        if let Some(mut child) = pty_state.child.take() {
            let _ = child.kill();
            // Brief wait for cleanup
            let _ = child.try_wait();
        }
        // Drop writer + pair closes the PTY
        drop(pty_state);
    }
    Ok(())
}
