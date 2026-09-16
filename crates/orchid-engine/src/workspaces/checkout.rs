use super::{
    domain::{
        CheckoutError, CheckoutErrorKind, PhysicalWorktreeAttachment,
        PhysicalWorktreeCheckoutRequest, PhysicalWorktreeCheckoutResult,
    },
    git::{external_path, git_commit_id, git_path, git_text, GitRunner},
};
use crate::repository_context::GitExecutable;
use std::{
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
};

pub fn materialize_checkout(
    git: &GitExecutable,
    request: &PhysicalWorktreeCheckoutRequest,
) -> Result<PhysicalWorktreeCheckoutResult, CheckoutError> {
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
) -> Result<(), CheckoutError> {
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
        PhysicalWorktreeAttachment::ExistingBranch { branch_ref } => branch_ref
            .strip_prefix("refs/heads/")
            .expect("existing branch attachment is validated"),
        _ => request.commit_id.as_str(),
    }));
    arguments
}

fn verify_checkout(
    runner: &GitRunner,
    repository: &Path,
    target: &Path,
    request: &PhysicalWorktreeCheckoutRequest,
) -> Result<PhysicalWorktreeCheckoutResult, CheckoutError> {
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
) -> Result<bool, CheckoutError> {
    let output = runner.required(repository, ["worktree", "list", "--porcelain", "-z"])?;
    Ok(output.split(|byte| *byte == 0).any(|field| {
        field
            .strip_prefix(b"worktree ")
            .and_then(|value| String::from_utf8(value.to_vec()).ok())
            .and_then(|value| PathBuf::from(value).canonicalize().ok())
            .is_some_and(|candidate| candidate == target)
    }))
}

fn canonical_repository(path: &Path) -> Result<PathBuf, CheckoutError> {
    path.canonicalize()
        .ok()
        .filter(|path| path.is_dir())
        .ok_or_else(|| checkout_unavailable("The source repository is unavailable."))
}

fn normalized_unused_target(repository: &Path, requested: &Path) -> Result<PathBuf, CheckoutError> {
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

fn checkout_unavailable(message: &'static str) -> CheckoutError {
    CheckoutError::new(CheckoutErrorKind::CheckoutUnavailable, message)
}

fn checkout_conflict(message: &'static str) -> CheckoutError {
    CheckoutError::new(CheckoutErrorKind::CheckoutConflict, message)
}
