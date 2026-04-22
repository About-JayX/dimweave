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

fn git_rev_parse_path(workspace_root: &Path, arg: &str) -> Result<PathBuf, String> {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "--path-format=absolute", arg])
        .current_dir(workspace_root)
        .output()
        .map_err(|e| format!("git rev-parse {arg} failed: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "git rev-parse {arg} failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let path = stdout.trim();
    if path.is_empty() {
        return Err(format!("git rev-parse {arg} returned an empty path"));
    }
    Ok(PathBuf::from(path))
}

/// Resolve the owner repository root for the selected workspace.
///
/// - When `workspace_root` is the main checkout, owner root is itself.
/// - When `workspace_root` is an existing git worktree, owner root is the
///   parent directory of the repository's common `.git` dir.
fn resolve_owner_repo_root(workspace_root: &Path) -> Result<PathBuf, String> {
    let worktree_root = git_rev_parse_path(workspace_root, "--show-toplevel")?;
    let common_git_dir = git_rev_parse_path(workspace_root, "--git-common-dir")?;
    let owner_root = if common_git_dir == worktree_root.join(".git") {
        worktree_root
    } else {
        common_git_dir
            .parent()
            .ok_or_else(|| {
                format!(
                    "git common dir has no parent: {}",
                    common_git_dir.display()
                )
            })?
            .to_path_buf()
    };
    owner_root
        .canonicalize()
        .map_err(|e| format!("invalid owner repo root {}: {e}", owner_root.display()))
}

/// Creates a git worktree for the given task under the owner repository's
/// `<repo>/.worktrees/tasks/<task_id>`.
///
/// `workspace_root` may be either the main repository checkout or an existing
/// git worktree. The new task worktree is always created under the owner repo's
/// shared `.worktrees/tasks` tree, while the new branch still forks from the
/// selected workspace's current HEAD.
pub fn create_task_worktree(workspace_root: &Path, task_id: &str) -> Result<PathBuf, String> {
    let owner_repo_root = resolve_owner_repo_root(workspace_root)?;
    let worktree_dir = owner_repo_root.join(".worktrees").join("tasks").join(task_id);
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
        .current_dir(workspace_root)
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

/// Remove the task's git worktree **and** its `task/<task_id>` branch.
///
/// - `git worktree remove --force <dir>` — kills the working dir even if
///   dirty (the task's files belong to us, caller is `DeleteTask` and
///   already warned).
/// - `git branch -D task/<task_id>` — hard-delete the ref. We use `-D`
///   (not `-d`) so orphan branches with unmerged commits still get swept;
///   the user explicitly asked for a clean delete.
///
/// Best-effort: if either step fails we return an error from the worktree
/// remove stage (that's the visible dir), but branch deletion is
/// swallowed on failure so a missing/merged branch doesn't spoil the
/// whole cleanup.
pub fn cleanup_task_worktree(workspace_root: &Path, task_id: &str) -> Result<(), String> {
    let owner_repo_root = resolve_owner_repo_root(workspace_root)?;
    let worktree_dir = owner_repo_root.join(".worktrees").join("tasks").join(task_id);
    let branch_name = format!("task/{task_id}");

    if worktree_dir.exists() {
        let output = std::process::Command::new("git")
            .args(["worktree", "remove", "--force"])
            .arg(&worktree_dir)
            .current_dir(&owner_repo_root)
            .output()
            .map_err(|e| format!("git worktree remove failed: {e}"))?;
        if !output.status.success() {
            return Err(format!(
                "git worktree remove failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }
    }

    // Branch cleanup is best-effort; an orphan branch won't pile up as
    // much disk noise as dirs but we still want it swept when possible.
    let _ = std::process::Command::new("git")
        .args(["branch", "-D", &branch_name])
        .current_dir(&owner_repo_root)
        .output();
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

        fn add_worktree(&self, name: &str) -> PathBuf {
            let path = self.path.join(".worktrees").join(name);
            let output = Command::new("git")
                .args(["worktree", "add", "-b", name])
                .arg(&path)
                .current_dir(&self.path)
                .output()
                .expect("git worktree add");
            assert!(
                output.status.success(),
                "git worktree add failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            path.canonicalize().expect("canonicalize worktree path")
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

    #[test]
    fn cleanup_also_deletes_task_branch() {
        let repo = TempRepo::new("branch_cleanup");
        let task_id = "branch_sweep_task";

        create_task_worktree(&repo.path, task_id).expect("create worktree");
        // Sanity: the branch should exist after worktree add -b.
        let before = Command::new("git")
            .args(["branch", "--list", &format!("task/{task_id}")])
            .current_dir(&repo.path)
            .output()
            .unwrap();
        let before_stdout = String::from_utf8_lossy(&before.stdout);
        assert!(
            before_stdout.contains(&format!("task/{task_id}")),
            "task branch should exist before cleanup (got: {before_stdout:?})",
        );

        cleanup_task_worktree(&repo.path, task_id).expect("cleanup");

        let after = Command::new("git")
            .args(["branch", "--list", &format!("task/{task_id}")])
            .current_dir(&repo.path)
            .output()
            .unwrap();
        let after_stdout = String::from_utf8_lossy(&after.stdout);
        assert!(
            !after_stdout.contains(&format!("task/{task_id}")),
            "task branch must be gone after cleanup (got: {after_stdout:?})",
        );
    }

    #[test]
    fn create_task_worktree_from_secondary_worktree_uses_owner_repo_directory() {
        let repo = TempRepo::new("secondary_owner");
        let seed_worktree = repo.add_worktree("seed");
        let task_id = "secondary_task";

        let wt_path = create_task_worktree(&seed_worktree, task_id).expect("create worktree");

        assert_eq!(
            wt_path,
            repo.path
                .join(".worktrees")
                .join("tasks")
                .join(task_id)
                .canonicalize()
                .expect("canonical worktree path")
        );
        assert!(
            !seed_worktree.join(".worktrees").join("tasks").join(task_id).exists(),
            "task worktree must not be nested inside the selected worktree"
        );

        cleanup_task_worktree(&seed_worktree, task_id).expect("cleanup worktree");
    }

    #[test]
    fn cleanup_task_worktree_from_secondary_worktree_removes_owner_repo_directory() {
        let repo = TempRepo::new("secondary_cleanup");
        let seed_worktree = repo.add_worktree("seed");
        let task_id = "secondary_cleanup_task";
        let expected_path = repo.path.join(".worktrees").join("tasks").join(task_id);

        let wt_path = create_task_worktree(&seed_worktree, task_id).expect("create worktree");
        let expected_canonical = expected_path
            .canonicalize()
            .expect("canonical owner repo worktree path");
        assert_eq!(wt_path, expected_canonical);
        assert!(expected_canonical.exists());
        assert!(
            !seed_worktree.join(".worktrees").join("tasks").join(task_id).exists(),
            "task worktree must not be nested inside the selected worktree"
        );

        cleanup_task_worktree(&seed_worktree, task_id).expect("cleanup worktree");

        assert!(
            !expected_path.exists(),
            "owner repo task worktree directory must be removed"
        );
    }
}
