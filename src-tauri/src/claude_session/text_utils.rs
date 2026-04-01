pub fn tail_chars(text: &str, keep: usize) -> &str {
    let char_len = text.chars().count();
    if char_len <= keep {
        return text;
    }
    let drop_chars = char_len - keep;
    let split_idx = text
        .char_indices()
        .nth(drop_chars)
        .map(|(idx, _)| idx)
        .unwrap_or(0);
    &text[split_idx..]
}

pub fn normalize_prompt_text(raw: &str) -> String {
    strip_ansi(raw)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase()
}

pub fn normalize_prompt_compact_text(raw: &str) -> String {
    strip_ansi(raw)
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect::<String>()
        .to_ascii_lowercase()
}

pub fn strip_ansi(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut chars = raw.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' {
            match chars.peek() {
                // CSI sequence: ESC [ ... <final byte>
                Some('[') => {
                    chars.next();
                    for esc in chars.by_ref() {
                        if ('@'..='~').contains(&esc) {
                            break;
                        }
                    }
                }
                // OSC sequence: ESC ] ... (BEL | ESC \)
                Some(']') => {
                    chars.next();
                    while let Some(osc) = chars.next() {
                        if osc == '\x07' {
                            break;
                        }
                        if osc == '\u{1b}' && matches!(chars.peek(), Some('\\')) {
                            chars.next();
                            break;
                        }
                    }
                }
                // Other ESC sequences (single char after ESC)
                _ => {
                    chars.next();
                }
            }
            continue;
        }
        // Strip other common control chars (BEL, etc.) but keep \n \r \t
        if ch.is_control() && ch != '\n' && ch != '\r' && ch != '\t' {
            continue;
        }
        out.push(ch);
    }
    out
}

pub fn extract_terminal_preview(raw: &str) -> Option<String> {
    let normalized = normalize_terminal_lines(raw);
    let mut blocks: Vec<Vec<String>> = Vec::new();
    let mut current: Vec<String> = Vec::new();

    for line in normalized.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if !current.is_empty() {
                blocks.push(std::mem::take(&mut current));
            }
            continue;
        }
        if is_terminal_chrome_line(trimmed) || is_box_drawing_only(trimmed) {
            continue;
        }
        current.push(trimmed.to_string());
    }

    if !current.is_empty() {
        blocks.push(current);
    }

    blocks
        .into_iter()
        .rev()
        .find(|block| !block.is_empty())
        .map(|block| block.join("\n"))
}

fn normalize_terminal_lines(raw: &str) -> String {
    let clean = strip_ansi(raw);
    let mut lines: Vec<String> = Vec::new();
    let mut current = String::new();

    for ch in clean.chars() {
        match ch {
            '\r' => current.clear(),
            '\n' => {
                lines.push(current.clone());
                current.clear();
            }
            _ => current.push(ch),
        }
    }

    if !current.is_empty() {
        lines.push(current);
    }

    lines.join("\n")
}

fn is_terminal_chrome_line(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    lower.starts_with("esc to interrupt")
        || lower.starts_with("press esc")
        || lower.starts_with("ctrl+c to exit")
        || lower.starts_with("claude terminal exited")
        || lower.starts_with("[agentnexus]")
}

fn is_box_drawing_only(line: &str) -> bool {
    line.chars().all(|ch| {
        ch.is_whitespace()
            || ('\u{2500}'..='\u{257F}').contains(&ch) // Box Drawing block
            || ('\u{2580}'..='\u{259F}').contains(&ch) // Block Elements
            || matches!(ch, '╭' | '╮' | '╰' | '╯') // rounded corners (U+256D–U+2570)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_ansi_csi() {
        assert_eq!(strip_ansi("\x1b[31mhello\x1b[0m"), "hello");
        assert_eq!(strip_ansi("\x1b[1;34mcolored\x1b[0m text"), "colored text");
    }
    #[test]
    fn strip_ansi_osc_bel() {
        assert_eq!(strip_ansi("\x1b]0;title\x07text"), "text");
    }
    #[test]
    fn strip_ansi_osc_st() {
        assert_eq!(strip_ansi("\x1b]0;title\x1b\\text"), "text");
    }
    #[test]
    fn strip_ansi_standalone_bel() {
        assert_eq!(strip_ansi("a\x07b"), "ab");
    }
    #[test]
    fn strip_ansi_keeps_whitespace() {
        assert_eq!(strip_ansi("a\nb\tc"), "a\nb\tc");
    }
    #[test]
    fn strip_ansi_control_chars() {
        assert_eq!(strip_ansi("a\x01\x02b"), "ab");
    }
    #[test]
    fn strip_ansi_mixed() {
        assert_eq!(strip_ansi("\x1b]0;t\x07\x1b[32mg\x1b[0m n"), "g n");
    }
    #[test]
    fn tail_chars_basic() {
        assert_eq!(tail_chars("abcdef", 3), "def");
        assert_eq!(tail_chars("ab", 5), "ab");
    }
    #[test]
    fn normalize_strips_and_joins() {
        assert_eq!(
            normalize_prompt_text("\x1b[1m  hello   world  \x1b[0m"),
            "hello world"
        );
    }
    #[test]
    fn preview_skips_chrome() {
        let input = "Esc to interrupt\n╭──────╮\nreal content\n╰──────╯\n";
        assert_eq!(
            extract_terminal_preview(input).as_deref(),
            Some("real content")
        );
    }
}
