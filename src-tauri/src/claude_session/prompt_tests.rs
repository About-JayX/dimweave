use super::{
    drain_log_lines, needs_user_attention, should_auto_confirm_development_prompt,
    should_emit_attention,
};

#[test]
fn matches_local_agentbridge_development_prompt() {
    let prompt = r#"
      --dangerously-load-development-channels is for local channel development only.
      Please use --channels to run a list of approved channels.
      Channels: server:agentbridge
      ❯ 1. I am using this for local development
        2. Exit
    "#;
    assert!(should_auto_confirm_development_prompt(prompt));
}

#[test]
fn ignores_non_agentbridge_prompts() {
    let prompt = r#"
      Please use --channels to run a list of approved channels.
      Channels: server:someone-else
      1. I am using this for local development
    "#;
    assert!(!should_auto_confirm_development_prompt(prompt));
}

#[test]
fn matches_prompt_with_ansi_sequences() {
    let prompt = "\u{1b}[1mPlease use --channels to run a list of approved channels.\u{1b}[0m\nChannels: server:agentbridge\n\u{1b}[32m1. I am using this for local development\u{1b}[0m";
    assert!(should_auto_confirm_development_prompt(prompt));
}

#[test]
fn matches_prompt_when_terminal_output_collapses_spaces() {
    let prompt = r#"
      WARNING:Loadingdevelopmentchannels
      Pleaseuse--channelstorunalistofapprovedchannels.
      Channels:server:agentbridge
      ❯1.Iamusingthisforlocaldevelopment
      2.Exit
    "#;
    assert!(should_auto_confirm_development_prompt(prompt));
}

#[test]
fn drains_complete_sanitized_log_lines() {
    let mut pending = String::new();
    let lines = drain_log_lines(
        &mut pending,
        "\u{1b}[32mhello\u{1b}[0m\nworld\r\npartial",
    );
    assert_eq!(lines, vec!["hello".to_string(), "world".to_string()]);
    assert_eq!(pending, "partial");
}

#[test]
fn detects_interactive_prompt_after_dev_prompt_in_same_tail() {
    let transcript = r#"
      Channels: server:agentbridge
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
      Channels: server:agentbridge
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
      Channels: server:agentbridge
      1. I am using this for local development
    "#;

    assert!(!should_emit_attention(
        transcript,
        true,
        should_auto_confirm_development_prompt(transcript)
    ));
}
