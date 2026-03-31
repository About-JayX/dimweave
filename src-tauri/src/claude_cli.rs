use std::{fmt, path::PathBuf, process::Command};

/// Build a PATH that includes dirs where node/codex/claude are likely installed.
/// macOS .app bundles inherit a minimal PATH from launchd.
pub fn enriched_path() -> String {
    let sys_path = std::env::var("PATH").unwrap_or_default();
    let home = std::env::var("HOME").unwrap_or_default();
    let mut dirs: Vec<String> = Vec::new();
    let nvm_dir = PathBuf::from(&home).join(".nvm/versions/node");
    if let Ok(entries) = std::fs::read_dir(&nvm_dir) {
        let mut vers: Vec<PathBuf> = entries
            .filter_map(|e| e.ok().map(|e| e.path().join("bin")))
            .filter(|p| p.exists())
            .collect();
        vers.sort();
        vers.reverse();
        for v in vers { dirs.push(v.to_string_lossy().into()); }
    }
    for d in &[".bun/bin", ".local/bin", ".cargo/bin"] {
        let p = PathBuf::from(&home).join(d);
        if p.exists() { dirs.push(p.to_string_lossy().into()); }
    }
    for d in &["/usr/local/bin", "/opt/homebrew/bin"] {
        if PathBuf::from(d).exists() { dirs.push((*d).into()); }
    }
    if !sys_path.is_empty() { dirs.push(sys_path); }
    dirs.join(":")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ClaudeVersion {
    major: u64,
    minor: u64,
    patch: u64,
}

impl ClaudeVersion {
    pub fn supports_channels_preview(&self) -> bool {
        *self
            >= ClaudeVersion {
                major: 2,
                minor: 1,
                patch: 80,
            }
    }

    pub fn is_known_bad_managed_pty(&self) -> bool {
        false
    }
}

impl fmt::Display for ClaudeVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

pub fn parse_claude_version(output: &str) -> Option<ClaudeVersion> {
    output.split_whitespace().find_map(|token| {
        let cleaned = token.trim_matches(|c: char| !c.is_ascii_digit() && c != '.');
        let mut parts = cleaned.split('.');
        Some(ClaudeVersion {
            major: parts.next()?.parse().ok()?,
            minor: parts.next()?.parse().ok()?,
            patch: parts.next()?.parse().ok()?,
        })
    })
}

/// Resolve `claude` binary — bundled sidecar > enriched PATH > which.
pub fn resolve_claude_bin() -> Result<PathBuf, String> {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let sidecar = dir.join("claude");
            if sidecar.exists() { return Ok(sidecar); }
        }
    }
    let path = enriched_path();
    which::which_in("claude", Some(&path), ".")
        .or_else(|_| which::which("claude"))
        .map_err(|_| "Claude Code CLI not found".to_string())
}

pub fn ensure_claude_channel_ready() -> Result<ClaudeVersion, String> {
    let claude = resolve_claude_bin()?;
    let output = Command::new(&claude)
        .env("PATH", enriched_path())
        .arg("-v")
        .output()
        .map_err(|e| format!("failed to run `claude -v`: {e}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let details = if stderr.is_empty() { stdout } else { stderr };
        return Err(format!("`claude -v` failed: {details}"));
    }
    let raw = format!(
        "{} {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let version = parse_claude_version(&raw)
        .ok_or_else(|| format!("failed to parse Claude Code version from `{}`", raw.trim()))?;
    validate_claude_channel_ready(version)
}

fn validate_claude_channel_ready(version: ClaudeVersion) -> Result<ClaudeVersion, String> {
    if !version.supports_channels_preview() {
        return Err(format!(
            "Claude Code {version} is too old for channel preview; require >= 2.1.80"
        ));
    }
    if version.is_known_bad_managed_pty() {
        return Err(format!(
            "Claude Code {version} is blocked in AgentNexus managed PTY: this release can crash with `_4.useRef is not a function` after Claude starts rendering tool activity. Downgrade to 2.1.84 (for example: `claude install 2.1.84 --force` or `npm i -g @anthropic-ai/claude-code@2.1.84`) or upgrade once Anthropic ships a fixed release."
        ));
    }
    Ok(version)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_supported_claude_version() {
        let version = parse_claude_version("2.1.81 (Claude Code)").expect("version should parse");
        assert!(version.supports_channels_preview());
    }

    #[test]
    fn reject_old_claude_version() {
        let version = parse_claude_version("2.1.79").expect("version should parse");
        assert!(!version.supports_channels_preview());
    }

    #[test]
    fn accept_previously_blocked_version() {
        let version = parse_claude_version("2.1.85 (Claude Code)").expect("version should parse");
        let accepted = validate_claude_channel_ready(version).expect("2.1.85 should now be allowed");
        assert_eq!(accepted, version);
    }

    #[test]
    fn accept_supported_non_bad_version() {
        let version = parse_claude_version("2.1.84 (Claude Code)").expect("version should parse");
        let accepted = validate_claude_channel_ready(version).expect("2.1.84 should be allowed");
        assert_eq!(accepted, version);
    }
}
