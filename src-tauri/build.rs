fn main() {
    // Copy the bridge sidecar binary into binaries/ so Tauri can bundle it.
    // The binary is only present after `cargo build` runs for the workspace.
    let profile = std::env::var("PROFILE").unwrap_or_else(|_| "debug".into());
    let target = std::env::var("TARGET").unwrap_or_else(|_| String::new());
    let src = format!("../target/{profile}/agent-bridge-bridge");
    if std::path::Path::new(&src).exists() {
        std::fs::create_dir_all("binaries").ok();
        let dst = format!("binaries/agent-bridge-bridge-{target}");
        std::fs::copy(&src, &dst).ok();
    } else {
        println!(
            "cargo:warning=bridge binary not found at {src} — build `bridge` crate first, \
             or externalBin will be missing at runtime"
        );
    }

    tauri_build::build()
}
