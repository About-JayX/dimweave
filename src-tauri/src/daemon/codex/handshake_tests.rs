use super::build_thread_start_params;
use crate::daemon::codex::session::SessionOpts;

#[test]
fn thread_start_params_without_network_uses_sandbox_string() {
    let params = build_thread_start_params(&SessionOpts {
        role_id: "coder".into(),
        cwd: "/tmp/project".into(),
        model: Some("gpt-5.4".into()),
        effort: Some("xhigh".into()),
        sandbox_mode: Some("workspace-write".into()),
        network_access: false,
        base_instructions: Some("follow role".into()),
    });

    assert_eq!(params["cwd"], "/tmp/project");
    assert_eq!(params["model"], "gpt-5.4");
    assert_eq!(params["effort"], "xhigh");
    assert_eq!(params["sandbox"], "workspace-write");
    assert!(params.get("sandboxPolicy").is_none());
    assert_eq!(params["baseInstructions"], "follow role");
}

#[test]
fn thread_start_params_with_network_uses_sandbox_policy() {
    let params = build_thread_start_params(&SessionOpts {
        role_id: "lead".into(),
        cwd: "/tmp/project".into(),
        model: Some("o3".into()),
        effort: Some("high".into()),
        sandbox_mode: Some("workspace-write".into()),
        network_access: true,
        base_instructions: None,
    });

    assert!(params.get("sandbox").is_none());
    assert_eq!(params["sandboxPolicy"]["type"], "workspace-write");
    assert_eq!(params["sandboxPolicy"]["networkAccess"], true);
}

#[test]
fn thread_start_params_omit_effort_when_absent() {
    let params = build_thread_start_params(&SessionOpts {
        role_id: "coder".into(),
        cwd: "/tmp/project".into(),
        model: None,
        effort: None,
        sandbox_mode: Some("read-only".into()),
        network_access: false,
        base_instructions: None,
    });

    assert!(params.get("effort").is_none());
    assert!(params.get("model").is_none());
    assert_eq!(params["sandbox"], "read-only");
}
