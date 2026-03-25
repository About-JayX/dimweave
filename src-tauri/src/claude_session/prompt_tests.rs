use super::{drain_log_lines, should_auto_confirm_development_prompt};

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
