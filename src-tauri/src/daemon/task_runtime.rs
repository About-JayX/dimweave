use std::path::PathBuf;

/// Per-task runtime state. Each active task owns one of these.
/// Future tasks will extend this with provider handles and connection state.
pub struct TaskRuntime {
    pub task_id: String,
    pub workspace_root: PathBuf,
}

impl TaskRuntime {
    pub fn new(task_id: String, workspace_root: PathBuf) -> Self {
        Self {
            task_id,
            workspace_root,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_runtime_construction() {
        let rt = TaskRuntime::new("task_1".into(), PathBuf::from("/ws/tasks/task_1"));
        assert_eq!(rt.task_id, "task_1");
        assert_eq!(rt.workspace_root, PathBuf::from("/ws/tasks/task_1"));
    }
}
