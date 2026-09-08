use super::{
    domain::{
        GitCommitId, VirtualCommitCaptureRequest, VirtualCommitCaptureResult,
        WorktreeApplicationError, WorktreeApplicationErrorKind,
    },
    git::{git_commit_id, git_text, GitRunner},
};
use crate::repository_context::GitExecutable;
use std::{
    fs,
    path::{Path, PathBuf},
};

const VIRTUAL_COMMIT_IDENTITY: &str = "Codex Orchestrator";
const VIRTUAL_COMMIT_EMAIL: &str = "codex-orchestrator@local.invalid";
const VIRTUAL_COMMIT_DATE: &str = "2000-01-01T00:00:00Z";
const VIRTUAL_COMMIT_MESSAGE: &str = "Codex Orchestrator virtual worktree capture";

pub(super) fn capture_virtual_commit(
    git: &GitExecutable,
    request: &VirtualCommitCaptureRequest,
) -> Result<VirtualCommitCaptureResult, WorktreeApplicationError> {
    let root = canonical_worktree(&request.worktree_root)?;
    let runner = GitRunner::new(git);
    let top_level = super::git::git_path(
        &root,
        runner.required(&root, ["rev-parse", "--show-toplevel"])?,
    )?;
    if top_level != root {
        return Err(source_changed());
    }
    let head_before =
        git_commit_id(runner.required(&root, ["rev-parse", "--verify", "HEAD^{commit}"])?)?;
    if head_before != request.expected_head {
        return Err(source_changed());
    }
    let baseline_tree =
        git_text(runner.required(&root, ["rev-parse", "--verify", "HEAD^{tree}"])?)?;
    let first_tree = capture_tree(&runner, &root, &request.expected_head)?;
    let second_tree = capture_tree(&runner, &root, &request.expected_head)?;
    let head_after =
        git_commit_id(runner.required(&root, ["rev-parse", "--verify", "HEAD^{commit}"])?)?;
    if head_after != head_before || first_tree != second_tree {
        return Err(source_changed());
    }
    let captured_changes = first_tree != baseline_tree;
    let virtual_commit = if captured_changes {
        let parent = request.expected_head.as_str();
        Some(git_commit_id(runner.required_with_commit_identity(
            &root,
            [
                "commit-tree",
                first_tree.as_str(),
                "-p",
                parent,
                "-m",
                VIRTUAL_COMMIT_MESSAGE,
            ],
            VIRTUAL_COMMIT_IDENTITY,
            VIRTUAL_COMMIT_EMAIL,
            VIRTUAL_COMMIT_DATE,
        )?)?)
    } else {
        None
    };
    Ok(VirtualCommitCaptureResult {
        worktree_root: root,
        baseline_commit: head_before,
        captured_tree: first_tree,
        captured_changes,
        virtual_commit,
    })
}

fn capture_tree(
    runner: &GitRunner,
    root: &Path,
    baseline: &GitCommitId,
) -> Result<String, WorktreeApplicationError> {
    let scratch = CaptureScratch::create()?;
    let index = scratch.path().join("index");
    runner.required_with_index(root, ["read-tree", baseline.as_str()], &index)?;
    runner.required_with_index(root, ["add", "--all", "--"], &index)?;
    git_text(runner.required_with_index(root, ["write-tree"], &index)?)
}

struct CaptureScratch(PathBuf);

impl CaptureScratch {
    fn create() -> Result<Self, WorktreeApplicationError> {
        for _ in 0..4 {
            let path = std::env::temp_dir()
                .join(format!("codex-worktree-capture-{}", uuid::Uuid::new_v4()));
            match fs::create_dir(&path) {
                Ok(()) => return Ok(Self(path)),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(_) => return Err(capture_unavailable()),
            }
        }
        Err(capture_unavailable())
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for CaptureScratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn canonical_worktree(path: &Path) -> Result<std::path::PathBuf, WorktreeApplicationError> {
    fs::canonicalize(path)
        .ok()
        .filter(|path| path.is_dir())
        .ok_or_else(|| {
            WorktreeApplicationError::new(
                WorktreeApplicationErrorKind::WorktreeUnavailable,
                "The source physical worktree is unavailable.",
            )
        })
}

fn source_changed() -> WorktreeApplicationError {
    WorktreeApplicationError::new(
        WorktreeApplicationErrorKind::SourceChanged,
        "The source worktree changed while its exact content was captured.",
    )
}

fn capture_unavailable() -> WorktreeApplicationError {
    WorktreeApplicationError::new(
        WorktreeApplicationErrorKind::GitUnavailable,
        "An isolated Git index could not be created for source capture.",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    #[test]
    fn dirty_capture_is_repeatable_and_does_not_modify_the_user_index() {
        let fixture = fixture();
        fs::write(fixture.path().join("tracked.txt"), "changed\n").unwrap();
        fs::write(fixture.path().join("untracked.txt"), "untracked\n").unwrap();
        let git = GitExecutable::discover().unwrap();
        let head = GitCommitId::new(text(fixture.path(), &["rev-parse", "HEAD"])).unwrap();
        let request = VirtualCommitCaptureRequest::new(fixture.path().to_path_buf(), head).unwrap();

        let first = capture_virtual_commit(&git, &request).unwrap();
        let second = capture_virtual_commit(&git, &request).unwrap();

        assert!(first.captured_changes);
        assert_eq!(first.virtual_commit, second.virtual_commit);
        assert_eq!(
            text(
                fixture.path(),
                &[
                    "show",
                    &format!("{}:untracked.txt", first.captured_commit().as_str())
                ]
            ),
            "untracked"
        );
        assert!(Command::new(git.path())
            .args(["diff", "--cached", "--quiet"])
            .current_dir(fixture.path())
            .status()
            .unwrap()
            .success());
    }

    #[test]
    fn clean_capture_uses_head_without_manufacturing_a_commit() {
        let fixture = fixture();
        let git = GitExecutable::discover().unwrap();
        let head = GitCommitId::new(text(fixture.path(), &["rev-parse", "HEAD"])).unwrap();
        let request =
            VirtualCommitCaptureRequest::new(fixture.path().to_path_buf(), head.clone()).unwrap();

        let result = capture_virtual_commit(&git, &request).unwrap();

        assert!(!result.captured_changes);
        assert_eq!(result.virtual_commit, None);
        assert_eq!(result.captured_commit(), &head);
    }

    fn fixture() -> tempfile::TempDir {
        let directory = tempfile::tempdir().unwrap();
        run(directory.path(), &["init", "-b", "main"]);
        run(directory.path(), &["config", "user.name", "Fixture"]);
        run(
            directory.path(),
            &["config", "user.email", "fixture@example.invalid"],
        );
        fs::write(directory.path().join("tracked.txt"), "baseline\n").unwrap();
        run(directory.path(), &["add", "--all"]);
        run(directory.path(), &["commit", "-m", "baseline"]);
        directory
    }

    fn run(root: &Path, arguments: &[&str]) {
        assert!(Command::new("git")
            .args(arguments)
            .current_dir(root)
            .status()
            .unwrap()
            .success());
    }

    fn text(root: &Path, arguments: &[&str]) -> String {
        let output = Command::new("git")
            .args(arguments)
            .current_dir(root)
            .output()
            .unwrap();
        assert!(output.status.success());
        String::from_utf8(output.stdout).unwrap().trim().to_owned()
    }
}
