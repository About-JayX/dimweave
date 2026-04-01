use super::describe_exit;
use portable_pty::CommandBuilder;

fn argv_strings(cmd: &CommandBuilder) -> Vec<String> {
    cmd.get_argv()
        .iter()
        .map(|arg| arg.to_string_lossy().to_string())
        .collect()
}

#[test]
fn describe_exit_formats_successful_exit() {
    let (level, code, summary) = describe_exit(Ok(portable_pty::ExitStatus::with_exit_code(0)));
    assert_eq!(level, "info");
    assert_eq!(code, Some(0));
    assert!(summary.contains("code 0"));
}

#[test]
fn describe_exit_formats_signaled_exit() {
    let (level, code, summary) = describe_exit(Ok(portable_pty::ExitStatus::with_signal("TERM")));
    assert_eq!(level, "warn");
    assert_eq!(code, None);
    assert!(summary.contains("signal TERM"));
}

#[test]
fn claude_launch_argv_defaults_to_bypass_permissions() {
    let cmd = super::build_claude_command(
        "/tmp/project",
        std::path::Path::new("/usr/local/bin/claude"),
        &[],
    );
    let argv = argv_strings(&cmd);
    assert!(
        argv.iter()
            .any(|arg| arg == "--dangerously-skip-permissions"),
        "expected launch argv to include --dangerously-skip-permissions, got {argv:?}"
    );
}
