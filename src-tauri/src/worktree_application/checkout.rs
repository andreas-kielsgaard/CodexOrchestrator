//! Desktop adapter for the shared exact-checkout primitive.
use super::domain::*;
use crate::repository_context::GitExecutable;
use orchid_engine::workspaces::{self, domain as engine};
#[cfg(test)]
use std::{fs, path::PathBuf};
pub(super) fn materialize_checkout(
    git: &GitExecutable,
    request: &PhysicalWorktreeCheckoutRequest,
) -> Result<PhysicalWorktreeCheckoutResult, WorktreeApplicationError> {
    let attachment = match &request.attachment {
        PhysicalWorktreeAttachment::Detached => engine::PhysicalWorktreeAttachment::Detached,
        PhysicalWorktreeAttachment::ExistingBranch { branch_ref } => {
            engine::PhysicalWorktreeAttachment::ExistingBranch {
                branch_ref: branch_ref.clone(),
            }
        }
        PhysicalWorktreeAttachment::NewBranch { branch_name } => {
            engine::PhysicalWorktreeAttachment::NewBranch {
                branch_name: branch_name.clone(),
            }
        }
    };
    let commit_id = engine::GitCommitId::new(request.commit_id.as_str()).map_err(map_error)?;
    let request = engine::PhysicalWorktreeCheckoutRequest::new(
        request.repository_root.clone(),
        request.worktree_root.clone(),
        commit_id,
        attachment,
    )
    .map_err(map_error)?;
    let result = workspaces::checkout::materialize_checkout(git, &request).map_err(map_error)?;
    Ok(PhysicalWorktreeCheckoutResult {
        repository_root: result.repository_root,
        worktree_root: result.worktree_root,
        commit_id: GitCommitId::new(result.commit_id.as_str())?,
        head_ref: result.head_ref,
    })
}
fn map_error(error: engine::CheckoutError) -> WorktreeApplicationError {
    let kind = match error.kind {
        engine::CheckoutErrorKind::InvalidRequest => WorktreeApplicationErrorKind::InvalidRequest,
        engine::CheckoutErrorKind::GitUnavailable => WorktreeApplicationErrorKind::GitUnavailable,
        engine::CheckoutErrorKind::CheckoutConflict => {
            WorktreeApplicationErrorKind::CheckoutConflict
        }
        engine::CheckoutErrorKind::CheckoutUnavailable => {
            WorktreeApplicationErrorKind::CheckoutUnavailable
        }
    };
    WorktreeApplicationError::new(kind, error.message)
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::worktree_application::GitCommitId;
    use std::{path::Path, process::Command};

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

    #[cfg(windows)]
    #[test]
    fn creates_and_adopts_checkout_with_files_beyond_windows_legacy_path_limit() {
        let fixture = fixture();
        let relative = PathBuf::from("nested/another-source-directory/one-more-directory")
            .join("source-file-with-a-long-name.txt");
        fs::create_dir_all(fixture.repository.join(relative.parent().unwrap())).unwrap();
        fs::write(fixture.repository.join(&relative), "retained source\n").unwrap();
        run(&fixture.repository, &["add", "--all"]);
        run(&fixture.repository, &["commit", "-m", "nested source"]);
        // The checkout root itself fits; its tracked files exceed MAX_PATH.
        let base = fixture.directory.path().join("retained-output");
        let padding = 210 - base.to_string_lossy().len() - "/checkout".len() - 1;
        let parent = base.join("w".repeat(padding));
        fs::create_dir_all(&parent).unwrap();
        let target = parent.join("checkout");
        assert!(target.join(&relative).to_string_lossy().len() > 260);
        let commit = GitCommitId::new(text(&fixture.repository, &["rev-parse", "HEAD"])).unwrap();
        let request = PhysicalWorktreeCheckoutRequest::new(
            fixture.repository.canonicalize().unwrap(),
            target,
            commit,
            PhysicalWorktreeAttachment::Detached,
        )
        .unwrap();
        let git = GitExecutable::discover().unwrap();
        let created = materialize_checkout(&git, &request).unwrap();
        assert_eq!(
            fs::read_to_string(created.worktree_root.join(relative)).unwrap(),
            "retained source\n"
        );
        assert_eq!(materialize_checkout(&git, &request).unwrap(), created);
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
