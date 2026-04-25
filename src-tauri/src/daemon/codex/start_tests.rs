use super::*;
use std::sync::{Arc, Mutex};

#[tokio::test]
async fn ensure_port_available_runs_cleanup_before_failing() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let holder = Arc::new(Mutex::new(Some(listener)));
    let cleanup_called = Arc::new(Mutex::new(0usize));

    let holder_for_cleanup = holder.clone();
    let cleanup_called_for_cleanup = cleanup_called.clone();
    ensure_port_available(port, std::time::Duration::from_millis(250), move |_| {
        let holder_for_cleanup = holder_for_cleanup.clone();
        let cleanup_called_for_cleanup = cleanup_called_for_cleanup.clone();
        async move {
            *cleanup_called_for_cleanup.lock().unwrap() += 1;
            holder_for_cleanup.lock().unwrap().take();
        }
    })
    .await
    .unwrap();

    assert_eq!(*cleanup_called.lock().unwrap(), 1);
}

#[tokio::test]
async fn ensure_port_available_times_out_when_cleanup_cannot_free_port() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();

    let err = ensure_port_available(port, std::time::Duration::from_millis(250), |_| async {})
        .await
        .unwrap_err()
        .to_string();

    assert!(err.contains(&format!("Port {port} still in use")));
}

#[test]
fn resolve_role_launch_config_uses_selected_model_prompt() {
    let (_, _, _, prompt) = resolve_role_launch_config("coder", Some("gpt-5.5"));
    let prompt = prompt.unwrap();
    assert!(prompt.contains("You have a vivid inner life as Codex"));
    assert!(prompt.contains("# Dimweave Multi-Agent System"));

    let (_, _, _, prompt) = resolve_role_launch_config("coder", Some("gpt-5.4"));
    let prompt = prompt.unwrap();
    assert!(prompt.contains("You are a deeply pragmatic, effective software engineer"));
    assert!(!prompt.contains("You have a vivid inner life as Codex"));
}
