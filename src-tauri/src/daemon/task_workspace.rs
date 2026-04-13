use std::path::{Path, PathBuf};

/// Validates that the given path is a git repository root.
pub fn validate_git_root(path: &str) -> Result<PathBuf, String> {
    let p = Path::new(path);
    if !p.join(".git").exists() {
        return Err(format!("not a git repository root: {path}"));
    }
    p.canonicalize()
        .map_err(|e| format!("invalid path {path}: {e}"))
}

/// Creates a git worktree for the given task under `<repo>/.worktrees/tasks/<task_id>`.
/// Returns the absolute path to the new worktree.
pub fn create_task_worktree(repo_root: &Path, task_id: &str) -> Result<PathBuf, String> {
    let worktree_dir = repo_root.join(".worktrees").join("tasks").join(task_id);
    if worktree_dir.exists() {
        return Err(format!(
            "worktree already exists: {}",
            worktree_dir.display()
        ));
    }

    let branch_name = format!("task/{task_id}");
    let output = std::process::Command::new("git")
        .args(["worktree", "add", "-b", &branch_name])
        .arg(&worktree_dir)
        .current_dir(repo_root)
        .output()
        .map_err(|e| format!("git worktree add failed: {e}"))?;

    if !output.status.success() {
        return Err(format!(
            "git worktree add failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    worktree_dir
        .canonicalize()
        .map_err(|e| format!("worktree path error: {e}"))
}

/// Removes a git worktree for the given task.
pub fn cleanup_task_worktree(repo_root: &Path, task_id: &str) -> Result<(), String> {
    let worktree_dir = repo_root.join(".worktrees").join("tasks").join(task_id);
    if !worktree_dir.exists() {
        return Ok(());
    }

    let output = std::process::Command::new("git")
        .args(["worktree", "remove", "--force"])
        .arg(&worktree_dir)
        .current_dir(repo_root)
        .output()
        .map_err(|e| format!("git worktree remove failed: {e}"))?;

    if !output.status.success() {
        return Err(format!(
            "git worktree remove failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    struct TempRepo {
        path: PathBuf,
    }

    impl TempRepo {
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir().join(format!(
                "dimweave_tw_{name}_{}_{}", std::process::id(), chrono::Utc::now().timestamp_millis()
            ));
            std::fs::create_dir_all(&path).unwrap();
            Command::new("git")
                .args(["init", "--initial-branch=main"])
                .current_dir(&path)
                .output()
                .expect("git init");
            Command::new("git")
                .args(["commit", "--allow-empty", "-m", "init"])
                .current_dir(&path)
                .output()
                .expect("git commit");
            Self { path }
        }
    }

    impl Drop for TempRepo {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    #[test]
    fn validate_git_root_succeeds_for_repo() {
        let repo = TempRepo::new("valid");
        assert!(validate_git_root(repo.path.to_str().unwrap()).is_ok());
    }

    #[test]
    fn validate_git_root_fails_for_non_repo() {
        let dir = std::env::temp_dir().join(format!(
            "dimweave_tw_nonrepo_{}_{}", std::process::id(), chrono::Utc::now().timestamp_millis()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let result = validate_git_root(dir.to_str().unwrap());
        let _ = std::fs::remove_dir_all(&dir);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not a git repository root"));
    }

    #[test]
    fn create_and_cleanup_task_worktree() {
        let repo = TempRepo::new("worktree_test");
        let task_id = "test_task_1";

        let wt_path = create_task_worktree(&repo.path, task_id).expect("create worktree");
        assert!(wt_path.exists());
        assert!(wt_path.join(".git").exists());

        cleanup_task_worktree(&repo.path, task_id).expect("cleanup worktree");
        assert!(!repo.path.join(".worktrees").join("tasks").join(task_id).exists());
    }

    #[test]
    fn create_worktree_fails_if_already_exists() {
        let repo = TempRepo::new("dup_wt");
        let task_id = "dup_task";

        create_task_worktree(&repo.path, task_id).expect("first create");
        let result = create_task_worktree(&repo.path, task_id);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("already exists"));

        cleanup_task_worktree(&repo.path, task_id).ok();
    }

    #[test]
    fn cleanup_nonexistent_worktree_is_noop() {
        let repo = TempRepo::new("noop_cleanup");
        assert!(cleanup_task_worktree(&repo.path, "nonexistent").is_ok());
    }
}
