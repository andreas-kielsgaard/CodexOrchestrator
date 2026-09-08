use super::{
    domain::{
        PhysicalWorktreeAttachment, PhysicalWorktreeCheckoutRequest,
        PhysicalWorktreeCheckoutResult, WorktreeApplicationError, WorktreeApplicationErrorKind,
    },
    git::{external_path, git_commit_id, git_path, git_text, GitRunner},
};
use crate::repository_context::GitExecutable;
use std::{
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
};

pub(super) fn materialize_checkout(
    git: &GitExecutable,
    request: &PhysicalWorktreeCheckoutRequest,
) -> Result<PhysicalWorktreeCheckoutResult, WorktreeApplicationError> {
    let repository = canonical_repository(&request.repository_root)?;
    let runner = GitRunner::new(git);
    if git_path(
        &repository,
        runner.required(&repository, ["rev-parse", "--show-toplevel"])?,
    )? != repository
    {
        return Err(checkout_unavailable(
            "The supplied repository root is not a top-level worktree.",
        ));
    }
    let verified_commit = git_commit_id(runner.required(
        &repository,
        [
            "rev-parse",
            "--verify",
            &format!("{}^{{commit}}", request.commit_id.as_str()),
        ],
    )?)?;
    if verified_commit != request.commit_id {
        return Err(checkout_unavailable(
            "The requested exact commit is unavailable in the repository.",
        ));
    }
    preflight_attachment(&runner, &repository, request)?;
    let target = normalized_unused_target(&repository, &request.worktree_root)?;
    if fs::symlink_metadata(&target).is_ok() {
        return verify_checkout(&runner, &repository, &target, request)
            .map_err(|_| checkout_conflict("The existing checkout does not match its plan."));
    }
    let arguments = checkout_arguments(request, &target);
    runner
        .required(&repository, arguments)
        .map_err(|_| checkout_conflict("Git could not create the exact physical worktree."))?;
    verify_checkout(&runner, &repository, &target, request)
}

fn preflight_attachment(
    runner: &GitRunner,
    repository: &Path,
    request: &PhysicalWorktreeCheckoutRequest,
) -> Result<(), WorktreeApplicationError> {
    match &request.attachment {
        PhysicalWorktreeAttachment::Detached => Ok(()),
        PhysicalWorktreeAttachment::ExistingBranch { branch_ref } => {
            let commit = git_commit_id(runner.required(
                repository,
                ["rev-parse", "--verify", &format!("{branch_ref}^{{commit}}")],
            )?)?;
            if commit == request.commit_id {
                Ok(())
            } else {
                Err(checkout_conflict(
                    "The named branch no longer identifies the planned commit.",
                ))
            }
        }
        PhysicalWorktreeAttachment::NewBranch { branch_name } => {
            runner.required(repository, ["check-ref-format", "--branch", branch_name])?;
            let full_ref = format!("refs/heads/{branch_name}");
            match runner.optional(repository, ["show-ref", "--verify", "--quiet", &full_ref])? {
                None => Ok(()),
                Some(_) if fs::symlink_metadata(&request.worktree_root).is_ok() => Ok(()),
                Some(_) => Err(checkout_conflict(
                    "The branch planned for this worktree already exists.",
                )),
            }
        }
    }
}

fn checkout_arguments(request: &PhysicalWorktreeCheckoutRequest, target: &Path) -> Vec<OsString> {
    let mut arguments = vec![OsString::from("worktree"), OsString::from("add")];
    match &request.attachment {
        PhysicalWorktreeAttachment::Detached => arguments.push(OsString::from("--detach")),
        PhysicalWorktreeAttachment::ExistingBranch { .. } => {}
        PhysicalWorktreeAttachment::NewBranch { branch_name } => {
            arguments.push(OsString::from("-b"));
            arguments.push(OsString::from(branch_name));
        }
    }
    arguments.push(OsString::from("--"));
    arguments.push(external_path(target));
    arguments.push(OsString::from(match &request.attachment {
        PhysicalWorktreeAttachment::ExistingBranch { branch_ref } => branch_ref,
        _ => request.commit_id.as_str(),
    }));
    arguments
}

fn verify_checkout(
    runner: &GitRunner,
    repository: &Path,
    target: &Path,
    request: &PhysicalWorktreeCheckoutRequest,
) -> Result<PhysicalWorktreeCheckoutResult, WorktreeApplicationError> {
    let target = target
        .canonicalize()
        .map_err(|_| checkout_unavailable("The created physical worktree is unavailable."))?;
    if !target.is_dir() {
        return Err(checkout_unavailable(
            "The created physical worktree is not a directory.",
        ));
    }
    let top_level = git_path(
        &target,
        runner.required(&target, ["rev-parse", "--show-toplevel"])?,
    )?;
    let repository_common = git_path(
        repository,
        runner.required(repository, ["rev-parse", "--git-common-dir"])?,
    )?;
    let target_common = git_path(
        &target,
        runner.required(&target, ["rev-parse", "--git-common-dir"])?,
    )?;
    let commit =
        git_commit_id(runner.required(&target, ["rev-parse", "--verify", "HEAD^{commit}"])?)?;
    let head_ref = runner
        .optional(&target, ["symbolic-ref", "-q", "HEAD"])?
        .map(git_text)
        .transpose()?;
    let status = runner.required(
        &target,
        ["status", "--porcelain=v1", "--untracked-files=all"],
    )?;
    if top_level != target
        || repository_common != target_common
        || commit != request.commit_id
        || head_ref != request.attachment.expected_head_ref()
        || !status.is_empty()
        || !is_registered(runner, repository, &target)?
    {
        return Err(checkout_conflict(
            "The created worktree does not match the exact checkout plan.",
        ));
    }
    Ok(PhysicalWorktreeCheckoutResult {
        repository_root: repository.to_path_buf(),
        worktree_root: target,
        commit_id: commit,
        head_ref,
    })
}

fn is_registered(
    runner: &GitRunner,
    repository: &Path,
    target: &Path,
) -> Result<bool, WorktreeApplicationError> {
    let output = runner.required(repository, ["worktree", "list", "--porcelain", "-z"])?;
    Ok(output.split(|byte| *byte == 0).any(|field| {
        field
            .strip_prefix(b"worktree ")
            .and_then(|value| String::from_utf8(value.to_vec()).ok())
            .and_then(|value| PathBuf::from(value).canonicalize().ok())
            .is_some_and(|candidate| candidate == target)
    }))
}

fn canonical_repository(path: &Path) -> Result<PathBuf, WorktreeApplicationError> {
    path.canonicalize()
        .ok()
        .filter(|path| path.is_dir())
        .ok_or_else(|| checkout_unavailable("The source repository is unavailable."))
}

fn normalized_unused_target(
    repository: &Path,
    requested: &Path,
) -> Result<PathBuf, WorktreeApplicationError> {
    let parent = requested
        .parent()
        .ok_or_else(|| checkout_conflict("The physical worktree target has no parent."))?;
    fs::create_dir_all(parent)
        .map_err(|_| checkout_unavailable("The physical worktree parent is unavailable."))?;
    let parent = parent
        .canonicalize()
        .map_err(|_| checkout_unavailable("The physical worktree parent is unavailable."))?;
    let name = requested
        .file_name()
        .filter(|name| !name.is_empty())
        .ok_or_else(|| checkout_conflict("The physical worktree target is invalid."))?;
    let target = parent.join(name);
    if target.starts_with(repository) {
        return Err(checkout_conflict(
            "The physical worktree target is inside the source worktree.",
        ));
    }
    Ok(target)
}

fn checkout_unavailable(message: &'static str) -> WorktreeApplicationError {
    WorktreeApplicationError::new(WorktreeApplicationErrorKind::CheckoutUnavailable, message)
}

fn checkout_conflict(message: &'static str) -> WorktreeApplicationError {
    WorktreeApplicationError::new(WorktreeApplicationErrorKind::CheckoutConflict, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::worktree_application::GitCommitId;
    use std::process::Command;

    #[test]
    fn creates_and_adopts_an_exact_detached_checkout() {
        let fixture = fixture();
        let git = GitExecutable::discover().unwrap();
        let commit = GitCommitId::new(text(&fixture.repository, &["rev-parse", "HEAD"])).unwrap();
        let target = fixture.directory.path().join("detached-checkout");
        let request = PhysicalWorktreeCheckoutRequest::new(
            fixture.repository.canonicalize().unwrap(),
            target,
            commit.clone(),
            PhysicalWorktreeAttachment::Detached,
        )
        .unwrap();

        let created = materialize_checkout(&git, &request).unwrap();
        let adopted = materialize_checkout(&git, &request).unwrap();

        assert_eq!(created, adopted);
        assert_eq!(created.commit_id, commit);
        assert_eq!(created.head_ref, None);
    }

    #[test]
    fn creates_an_exact_new_named_branch_checkout() {
        let fixture = fixture();
        let git = GitExecutable::discover().unwrap();
        let commit = GitCommitId::new(text(&fixture.repository, &["rev-parse", "HEAD"])).unwrap();
        let target = fixture.directory.path().join("named-checkout");
        let request = PhysicalWorktreeCheckoutRequest::new(
            fixture.repository.canonicalize().unwrap(),
            target,
            commit.clone(),
            PhysicalWorktreeAttachment::new_branch("feature/exact").unwrap(),
        )
        .unwrap();

        let result = materialize_checkout(&git, &request).unwrap();

        assert_eq!(result.commit_id, commit);
        assert_eq!(result.head_ref.as_deref(), Some("refs/heads/feature/exact"));
    }

    struct Fixture {
        directory: tempfile::TempDir,
        repository: PathBuf,
    }

    fn fixture() -> Fixture {
        let directory = tempfile::tempdir().unwrap();
        let repository = directory.path().join("repository");
        fs::create_dir(&repository).unwrap();
        run(&repository, &["init", "-b", "main"]);
        run(&repository, &["config", "user.name", "Fixture"]);
        run(
            &repository,
            &["config", "user.email", "fixture@example.invalid"],
        );
        fs::write(repository.join("baseline.txt"), "baseline\n").unwrap();
        run(&repository, &["add", "--all"]);
        run(&repository, &["commit", "-m", "baseline"]);
        Fixture {
            directory,
            repository,
        }
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
