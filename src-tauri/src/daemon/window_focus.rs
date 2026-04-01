use tauri::{AppHandle, Manager, Runtime};

pub(super) trait AttentionWindowOps {
    fn show_attention_window(&self) -> Result<(), String>;
    fn unminimize_attention_window(&self) -> Result<(), String>;
    fn focus_attention_window(&self) -> Result<(), String>;
}

impl<R: Runtime> AttentionWindowOps for tauri::WebviewWindow<R> {
    fn show_attention_window(&self) -> Result<(), String> {
        self.show().map_err(|err| err.to_string())
    }

    fn unminimize_attention_window(&self) -> Result<(), String> {
        self.unminimize().map_err(|err| err.to_string())
    }

    fn focus_attention_window(&self) -> Result<(), String> {
        self.set_focus().map_err(|err| err.to_string())
    }
}

pub(super) fn focus_attention_window(window: &impl AttentionWindowOps) {
    let _ = window.show_attention_window();
    let _ = window.unminimize_attention_window();
    let _ = window.focus_attention_window();
}

pub(super) fn focus_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        focus_attention_window(&window);
    }
}

#[cfg(test)]
mod tests {
    use super::{focus_attention_window, AttentionWindowOps};

    struct MockWindow {
        calls: std::sync::Mutex<Vec<&'static str>>,
        show_ok: bool,
        unminimize_ok: bool,
        focus_ok: bool,
    }

    impl MockWindow {
        fn new(show_ok: bool, unminimize_ok: bool, focus_ok: bool) -> Self {
            Self {
                calls: std::sync::Mutex::new(Vec::new()),
                show_ok,
                unminimize_ok,
                focus_ok,
            }
        }

        fn calls(&self) -> Vec<&'static str> {
            self.calls.lock().unwrap().clone()
        }
    }

    impl AttentionWindowOps for MockWindow {
        fn show_attention_window(&self) -> Result<(), String> {
            self.calls.lock().unwrap().push("show");
            self.show_ok
                .then_some(())
                .ok_or_else(|| "show failed".into())
        }

        fn unminimize_attention_window(&self) -> Result<(), String> {
            self.calls.lock().unwrap().push("unminimize");
            self.unminimize_ok
                .then_some(())
                .ok_or_else(|| "unminimize failed".into())
        }

        fn focus_attention_window(&self) -> Result<(), String> {
            self.calls.lock().unwrap().push("focus");
            self.focus_ok
                .then_some(())
                .ok_or_else(|| "focus failed".into())
        }
    }

    #[test]
    fn focus_attention_window_attempts_all_steps_in_order() {
        let window = MockWindow::new(true, true, true);

        focus_attention_window(&window);

        assert_eq!(window.calls(), vec!["show", "unminimize", "focus"]);
    }

    #[test]
    fn focus_attention_window_keeps_going_after_nonfatal_errors() {
        let window = MockWindow::new(false, false, true);

        focus_attention_window(&window);

        assert_eq!(window.calls(), vec!["show", "unminimize", "focus"]);
    }
}
