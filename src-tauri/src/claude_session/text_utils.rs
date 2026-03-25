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
            if matches!(chars.peek(), Some('[')) {
                chars.next();
                for esc in chars.by_ref() {
                    if ('@'..='~').contains(&esc) {
                        break;
                    }
                }
                continue;
            }
            continue;
        }
        out.push(ch);
    }
    out
}
