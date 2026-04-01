use super::{
    drain_log_lines, extract_terminal_preview, needs_user_attention, next_attention_event,
    should_auto_confirm_development_prompt, should_emit_attention,
};

#[test]
fn matches_local_agentnexus_development_prompt() {
    let prompt = r#"
      --dangerously-load-development-channels is for local channel development only.
      Please use --channels to run a list of approved channels.
      Channels: server:agentnexus
      ❯ 1. I am using this for local development
        2. Exit
    "#;
    assert!(should_auto_confirm_development_prompt(prompt));
}

#[test]
fn ignores_non_agentnexus_prompts() {
    let prompt = r#"
      Please use --channels to run a list of approved channels.
      Channels: server:someone-else
      1. I am using this for local development
    "#;
    assert!(!should_auto_confirm_development_prompt(prompt));
}

#[test]
fn matches_prompt_with_ansi_sequences() {
    let prompt = "\u{1b}[1mPlease use --channels to run a list of approved channels.\u{1b}[0m\nChannels: server:agentnexus\n\u{1b}[32m1. I am using this for local development\u{1b}[0m";
    assert!(should_auto_confirm_development_prompt(prompt));
}

#[test]
fn matches_prompt_when_terminal_output_collapses_spaces() {
    let prompt = r#"
      WARNING:Loadingdevelopmentchannels
      Pleaseuse--channelstorunalistofapprovedchannels.
      Channels:server:agentnexus
      ❯1.Iamusingthisforlocaldevelopment
      2.Exit
    "#;
    assert!(should_auto_confirm_development_prompt(prompt));
}

#[test]
fn drains_complete_sanitized_log_lines() {
    let mut pending = String::new();
    let lines = drain_log_lines(&mut pending, "\u{1b}[32mhello\u{1b}[0m\nworld\r\npartial");
    assert_eq!(lines, vec!["hello".to_string(), "world".to_string()]);
    assert_eq!(pending, "partial");
}

#[test]
fn detects_interactive_prompt_after_dev_prompt_in_same_tail() {
    let transcript = r#"
      Channels: server:agentnexus
      1. I am using this for local development
      2. Exit

      Select an action:
      1. Continue
      2. Cancel
    "#;

    assert!(needs_user_attention(transcript));
}

#[test]
fn ignores_unicode_tail_without_panicking() {
    let transcript = format!("{}Continue?\n", "─".repeat(197));

    assert!(needs_user_attention(&transcript));
}

#[test]
fn emits_attention_for_dev_prompt_before_auto_confirm() {
    let transcript = r#"
      Please use --channels to run a list of approved channels.
      Channels: server:agentnexus
      1. I am using this for local development
    "#;

    assert!(should_emit_attention(
        transcript,
        false,
        should_auto_confirm_development_prompt(transcript)
    ));
}

#[test]
fn stops_attention_for_dev_prompt_after_auto_confirm() {
    let transcript = r#"
      Please use --channels to run a list of approved channels.
      Channels: server:agentnexus
      1. I am using this for local development
    "#;

    assert!(!should_emit_attention(
        transcript,
        true,
        should_auto_confirm_development_prompt(transcript)
    ));
}

#[test]
fn terminal_preview_drops_empty_frames() {
    assert_eq!(
        extract_terminal_preview("\u{1b}[32m\r\n\r\n\u{1b}[0m"),
        None
    );
}

#[test]
fn terminal_preview_handles_carriage_return_overwrite() {
    let preview = extract_terminal_preview("Thinking...\rDone\n").unwrap();
    assert_eq!(preview, "Done");
}

#[test]
fn terminal_preview_ignores_box_drawing_only_block() {
    let preview = extract_terminal_preview("╭────╮\n│    │\n╰────╯\n");
    assert_eq!(preview, None);
}

#[test]
fn terminal_preview_returns_last_meaningful_block() {
    let transcript = "Booting...\n\nAnalyzing repo\nChecking routes\n";
    let preview = extract_terminal_preview(transcript).unwrap();
    assert_eq!(preview, "Analyzing repo\nChecking routes");
}

#[test]
fn terminal_preview_strips_ansi_from_meaningful_text() {
    let preview = extract_terminal_preview("\u{1b}[32mAnalyzing\u{1b}[0m repo\n").unwrap();
    assert_eq!(preview, "Analyzing repo");
}

#[test]
fn attention_event_debounces_while_same_prompt_stays_visible() {
    let transcript = "Select an action:\n1. Continue\n2. Cancel\n";

    let first = next_attention_event(false, transcript, true, false);
    assert!(first.emit);
    assert!(first.active);

    let second = next_attention_event(first.active, transcript, true, false);
    assert!(!second.emit);
    assert!(second.active);
}

#[test]
fn attention_event_refires_after_prompt_clears_and_returns() {
    let prompt = "Continue?\n";
    let cleared = "Working...\n";

    let first = next_attention_event(false, prompt, true, false);
    assert!(first.emit);
    assert!(first.active);

    let idle = next_attention_event(first.active, cleared, true, false);
    assert!(!idle.emit);
    assert!(!idle.active);

    let second = next_attention_event(idle.active, prompt, true, false);
    assert!(second.emit);
    assert!(second.active);
}
