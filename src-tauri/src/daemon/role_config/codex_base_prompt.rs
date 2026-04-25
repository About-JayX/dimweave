//! Archived official Codex model prompts used as Dimweave merge bases.
//!
//! `baseInstructions` replaces the base instruction layer Codex sends to the
//! Responses API. Dimweave therefore starts from the selected model's archived
//! official default prompt and appends the Dimweave role protocol in `roles.rs`.

pub const FALLBACK_MODEL_PROMPT: &str = "gpt-5.5";

const GPT_5: &str = include_str!("../../../../docs/codex/prompts/gpt-5.md");
const GPT_5_CODEX: &str = include_str!("../../../../docs/codex/prompts/gpt-5-codex.md");
const GPT_5_CODEX_MINI: &str = include_str!("../../../../docs/codex/prompts/gpt-5-codex-mini.md");
const GPT_5_1: &str = include_str!("../../../../docs/codex/prompts/gpt-5.1.md");
const GPT_5_1_CODEX: &str = include_str!("../../../../docs/codex/prompts/gpt-5.1-codex.md");
const GPT_5_1_CODEX_MAX: &str = include_str!("../../../../docs/codex/prompts/gpt-5.1-codex-max.md");
const GPT_5_1_CODEX_MINI: &str =
    include_str!("../../../../docs/codex/prompts/gpt-5.1-codex-mini.md");
const GPT_5_2: &str = include_str!("../../../../docs/codex/prompts/gpt-5.2.md");
const GPT_5_2_CODEX: &str = include_str!("../../../../docs/codex/prompts/gpt-5.2-codex.md");
const GPT_5_3_CODEX: &str = include_str!("../../../../docs/codex/prompts/gpt-5.3-codex.md");
const GPT_5_4: &str = include_str!("../../../../docs/codex/prompts/gpt-5.4.md");
const GPT_5_5: &str = include_str!("../../../../docs/codex/prompts/gpt-5.5.md");
const GPT_OSS_20B: &str = include_str!("../../../../docs/codex/prompts/gpt-oss-20b.md");
const GPT_OSS_120B: &str = include_str!("../../../../docs/codex/prompts/gpt-oss-120b.md");

/// Return the archived official default `base_instructions` for the selected
/// model. Unknown or omitted models fall back to the newest archived prompt so
/// Dimweave can still inject a complete `baseInstructions` value.
pub fn default_base_instructions_for_model(model: Option<&str>) -> &'static str {
    let doc = prompt_doc_for_model(model);
    extract_markdown_section(doc, "base_instructions")
        .unwrap_or(doc)
        .trim()
}

pub fn prompt_source_model(model: Option<&str>) -> &'static str {
    let key = model.unwrap_or_default().trim().to_ascii_lowercase();
    match key.as_str() {
        "gpt-5" => "gpt-5",
        "gpt-5-codex" => "gpt-5-codex",
        "gpt-5-codex-mini" => "gpt-5-codex-mini",
        "gpt-5.1" => "gpt-5.1",
        "gpt-5.1-codex" => "gpt-5.1-codex",
        "gpt-5.1-codex-max" => "gpt-5.1-codex-max",
        "gpt-5.1-codex-mini" => "gpt-5.1-codex-mini",
        "gpt-5.2" => "gpt-5.2",
        "gpt-5.2-codex" => "gpt-5.2-codex",
        "gpt-5.3-codex" => "gpt-5.3-codex",
        "gpt-5.4" => "gpt-5.4",
        "gpt-5.5" => "gpt-5.5",
        "gpt-oss-20b" => "gpt-oss-20b",
        "gpt-oss-120b" => "gpt-oss-120b",
        _ => FALLBACK_MODEL_PROMPT,
    }
}

fn prompt_doc_for_model(model: Option<&str>) -> &'static str {
    match prompt_source_model(model) {
        "gpt-5" => GPT_5,
        "gpt-5-codex" => GPT_5_CODEX,
        "gpt-5-codex-mini" => GPT_5_CODEX_MINI,
        "gpt-5.1" => GPT_5_1,
        "gpt-5.1-codex" => GPT_5_1_CODEX,
        "gpt-5.1-codex-max" => GPT_5_1_CODEX_MAX,
        "gpt-5.1-codex-mini" => GPT_5_1_CODEX_MINI,
        "gpt-5.2" => GPT_5_2,
        "gpt-5.2-codex" => GPT_5_2_CODEX,
        "gpt-5.3-codex" => GPT_5_3_CODEX,
        "gpt-5.4" => GPT_5_4,
        "gpt-5.5" => GPT_5_5,
        "gpt-oss-20b" => GPT_OSS_20B,
        "gpt-oss-120b" => GPT_OSS_120B,
        _ => GPT_5_5,
    }
}

fn extract_markdown_section(doc: &'static str, section_name: &str) -> Option<&'static str> {
    let marker = format!("## {section_name}");
    let heading_start = doc.find(&marker)?;
    let body_start = doc[heading_start..].find('\n')? + heading_start + 1;
    let body = &doc[body_start..];
    let body_end = body.find("\n## ").unwrap_or(body.len());
    Some(body[..body_end].trim())
}

#[cfg(test)]
mod tests {
    use super::{default_base_instructions_for_model, prompt_source_model};

    #[test]
    fn extracts_only_base_instructions_section() {
        let prompt = default_base_instructions_for_model(Some("gpt-5.5"));

        assert!(prompt.starts_with("You are Codex"));
        assert!(!prompt.contains("## instructions_template"));
        assert!(!prompt.contains("## personality_pragmatic"));
    }

    #[test]
    fn prompt_source_falls_back_to_latest_archive_for_unknown_models() {
        assert_eq!(prompt_source_model(Some("future-model")), "gpt-5.5");
        assert_eq!(prompt_source_model(None), "gpt-5.5");
    }
}
