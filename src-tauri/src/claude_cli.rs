use std::{fmt, process::Command};

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

pub fn ensure_claude_channel_ready() -> Result<ClaudeVersion, String> {
    let claude =
        which::which("claude").map_err(|_| "Claude Code CLI not found in PATH".to_string())?;
    let output = Command::new(&claude)
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
    if !version.supports_channels_preview() {
        return Err(format!(
            "Claude Code {version} is too old for channel preview; require >= 2.1.80"
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
}
