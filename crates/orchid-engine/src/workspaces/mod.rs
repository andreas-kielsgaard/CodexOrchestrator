//! Destination-owned published revisions and exact physical checkouts.
pub mod checkout;
pub mod domain;
mod git;

use crate::{
    contracts::{RuntimePortError, RuntimePortErrorKind},
    protocol::WorktreeInstance,
    repository_context::GitExecutable,
};
use domain::*;
use git::{git_text, GitRunner};
use std::path::Path;

fn unavailable(error: impl std::fmt::Display) -> RuntimePortError {
    RuntimePortError::new(RuntimePortErrorKind::Unavailable, error.to_string())
}

fn published_remote(
    runner: &GitRunner,
    root: &Path,
    branch: &str,
) -> Result<String, RuntimePortError> {
    let name = branch
        .strip_prefix("refs/heads/")
        .ok_or_else(|| unavailable("Select a published branch"))?;
    PhysicalWorktreeAttachment::existing_branch(branch).map_err(unavailable)?;
    let configured = runner
        .optional(root, ["config", "--get", &format!("branch.{name}.remote")])
        .map_err(unavailable)?;
    let remote = configured
        .map(git_text)
        .transpose()
        .map_err(unavailable)?
        .unwrap_or_else(|| "origin".into());
    if remote == "." {
        return Err(unavailable("The branch has no published remote"));
    }
    runner
        .required(root, ["remote", "get-url", &remote])
        .map_err(unavailable)?;
    Ok(remote)
}

/// Queries the remote, without fetching objects or changing local refs.
pub fn published_commit(root: &str, branch: &str) -> Result<String, RuntimePortError> {
    let executable = GitExecutable::discover().map_err(unavailable)?;
    let runner = GitRunner::new(&executable);
    let root = Path::new(root);
    let remote = published_remote(&runner, root, branch)?;
    let name = branch
        .strip_prefix("refs/heads/")
        .expect("validated branch");
    let published_ref = runner
        .optional(root, ["config", "--get", &format!("branch.{name}.merge")])
        .map_err(unavailable)?
        .map(git_text)
        .transpose()
        .map_err(unavailable)?
        .unwrap_or_else(|| branch.into());
    let output = git_text(
        runner
            .required(
                root,
                [
                    "ls-remote",
                    "--exit-code",
                    "--refs",
                    &remote,
                    &published_ref,
                ],
            )
            .map_err(unavailable)?,
    )
    .map_err(unavailable)?;
    let commit = output
        .lines()
        .find_map(|line| {
            let (sha, reference) = line.split_once('\t')?;
            (reference == published_ref).then_some(sha)
        })
        .ok_or_else(|| unavailable("The selected branch has no published commit"))?;
    Ok(GitCommitId::new(commit)
        .map_err(unavailable)?
        .as_str()
        .into())
}

pub fn materialize_worktree(
    root: &str,
    branch: &str,
    commit: &str,
    instance_id: &str,
) -> Result<WorktreeInstance, RuntimePortError> {
    if instance_id.is_empty()
        || !instance_id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return Err(unavailable("Invalid worktree instance identity"));
    }
    let root = Path::new(root).canonicalize().map_err(unavailable)?;
    let executable = GitExecutable::discover().map_err(unavailable)?;
    let runner = GitRunner::new(&executable);
    let commit = GitCommitId::new(commit).map_err(unavailable)?;
    PhysicalWorktreeAttachment::existing_branch(branch).map_err(unavailable)?;
    if runner
        .required(
            &root,
            ["cat-file", "-e", &format!("{}^{{commit}}", commit.as_str())],
        )
        .is_err()
    {
        let remote = published_remote(&runner, &root, branch)?;
        runner
            .required(&root, ["fetch", "--no-tags", &remote, commit.as_str()])
            .map_err(unavailable)?;
    }
    let parent = root
        .parent()
        .ok_or_else(|| unavailable("Repository has no worktree parent"))?;
    let name = root
        .file_name()
        .ok_or_else(|| unavailable("Repository has no name"))?
        .to_string_lossy();
    let target = parent
        .join(format!("{name}.orchid-worktrees"))
        .join(instance_id);
    let branch_exists = runner
        .optional(&root, ["show-ref", "--verify", "--quiet", branch])
        .map_err(unavailable)?
        .is_some();
    let attachment = if branch_exists {
        PhysicalWorktreeAttachment::existing_branch(branch)
    } else {
        PhysicalWorktreeAttachment::new_branch(
            branch
                .strip_prefix("refs/heads/")
                .expect("validated branch"),
        )
    }
    .map_err(unavailable)?;
    let request = PhysicalWorktreeCheckoutRequest::new(root.clone(), target, commit, attachment)
        .map_err(unavailable)?;
    let result = checkout::materialize_checkout(&executable, &request).map_err(unavailable)?;
    crate::host::list_worktrees(
        root.to_str()
            .ok_or_else(|| unavailable("Repository path is not UTF-8"))?,
        Some(branch),
    )?
    .into_iter()
    .find(|instance| {
        Path::new(&instance.path).canonicalize().ok().as_ref() == Some(&result.worktree_root)
    })
    .ok_or_else(|| unavailable("The created worktree was not registered"))
}

pub fn auxiliary_workspace(base: &Path, session_id: &str) -> Result<String, RuntimePortError> {
    if session_id.is_empty()
        || !session_id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return Err(unavailable("Invalid session identity"));
    }
    let target = base.join("session-workspaces").join(session_id);
    std::fs::create_dir_all(&target).map_err(unavailable)?;
    Ok(target
        .canonicalize()
        .map_err(unavailable)?
        .to_string_lossy()
        .into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, process::Command};
    fn git(root: &Path, args: &[&str]) -> String {
        let output = Command::new("git")
            .current_dir(root)
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).unwrap().trim().into()
    }
    #[test]
    fn published_tip_ignores_unpublished_work_and_materialization_keeps_confirmed_commit() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source");
        fs::create_dir(&source).unwrap();
        git(&source, &["init", "-b", "main"]);
        git(&source, &["config", "user.name", "Fixture"]);
        git(
            &source,
            &["config", "user.email", "fixture@example.invalid"],
        );
        fs::write(source.join("README.md"), "published").unwrap();
        git(&source, &["add", "."]);
        git(&source, &["commit", "-m", "published"]);
        git(dir.path(), &["clone", "--bare", "source", "published.git"]);
        git(&source, &["remote", "add", "origin", "../published.git"]);
        git(&source, &["branch", "feature/confirmed"]);
        git(&source, &["push", "origin", "feature/confirmed"]);
        let confirmed =
            published_commit(source.to_str().unwrap(), "refs/heads/feature/confirmed").unwrap();
        fs::write(source.join("README.md"), "later").unwrap();
        git(&source, &["commit", "-am", "unpublished"]);
        assert_eq!(
            published_commit(source.to_str().unwrap(), "refs/heads/feature/confirmed").unwrap(),
            confirmed
        );
        git(
            &source,
            &["push", "origin", "HEAD:refs/heads/feature/confirmed"],
        );
        assert_ne!(
            published_commit(source.to_str().unwrap(), "refs/heads/feature/confirmed").unwrap(),
            confirmed
        );
        let instance = materialize_worktree(
            source.to_str().unwrap(),
            "refs/heads/feature/confirmed",
            &confirmed,
            "fixture-instance",
        )
        .unwrap();
        assert_eq!(instance.head.as_deref(), Some(confirmed.as_str()));
        let again = materialize_worktree(
            source.to_str().unwrap(),
            "refs/heads/feature/confirmed",
            &confirmed,
            "fixture-instance",
        )
        .unwrap();
        assert_eq!(again, instance);
        assert_eq!(
            fs::read_to_string(Path::new(&instance.path).join("README.md")).unwrap(),
            "published"
        );
    }
}
