use super::{
    domain::GitCommitId,
    git::{external_path, git_commit_id, git_path, git_text, GitRunner},
};
use crate::{
    contracts::{RuntimePortError, RuntimePortErrorKind},
    protocol::{
        WorktreeChangeSummary, WorktreeHeadComparison, WorktreeHeadRelation, WorktreeInspection,
        WorktreeSnapshot, WorktreeSnapshotDescriptor, WorktreeSnapshotFile,
    },
    repository_context::GitExecutable,
};
use std::{
    ffi::OsString,
    fs,
    path::{Component, Path, PathBuf},
};

fn unavailable(error: impl std::fmt::Display) -> RuntimePortError {
    RuntimePortError::new(RuntimePortErrorKind::Unavailable, error.to_string())
}

/// Reads the Git facts Orchid needs before planning a sister-worktree move.
pub fn inspect_worktree(
    worktree_root: &str,
    compare_to_head: Option<&str>,
) -> Result<WorktreeInspection, RuntimePortError> {
    let (root, runner) = open_worktree(worktree_root)?;
    inspect(&root, &runner, compare_to_head)
}

/// Captures the source-only commits plus index, working-tree, and nonignored
/// untracked overlays. The returned payload is portable across the local and
/// SSH host command paths; no user ref is pushed or committed.
pub fn capture_worktree_snapshot(
    worktree_root: &str,
    destination_head: Option<&str>,
    snapshot_id: &str,
) -> Result<WorktreeSnapshot, RuntimePortError> {
    validate_snapshot_id(snapshot_id)?;
    let (root, runner) = open_worktree(worktree_root)?;
    let source = inspect(&root, &runner, destination_head)?;

    if let Some(comparison) = &source.comparison {
        if comparison.relation != WorktreeHeadRelation::Ahead
            && comparison.relation != WorktreeHeadRelation::Equal
        {
            return Err(unavailable(
                "A worktree snapshot requires the source HEAD to equal or be ahead of the destination HEAD",
            ));
        }
    }

    let staged_patch = staged_patch(&root, &runner)?;
    let unstaged_patch = unstaged_patch(&root, &runner)?;
    let untracked_files = snapshot_untracked_files(&root, &runner)?;
    let commit_bundle = commit_bundle(&root, &runner, &source.head, destination_head, snapshot_id)?;
    let untracked_bytes = untracked_files.iter().try_fold(0_u64, |total, file| {
        checked_bytes(total, file.content.len())
    })?;
    let commit_bundle_bytes = bytes(commit_bundle.len())?;
    let staged_patch_bytes = bytes(staged_patch.len())?;
    let unstaged_patch_bytes = bytes(unstaged_patch.len())?;
    let total_bytes = [
        commit_bundle_bytes,
        staged_patch_bytes,
        unstaged_patch_bytes,
        untracked_bytes,
    ]
    .into_iter()
    .try_fold(0_u64, |total, value| {
        total
            .checked_add(value)
            .ok_or_else(|| unavailable("The worktree snapshot is too large"))
    })?;

    Ok(WorktreeSnapshot {
        descriptor: WorktreeSnapshotDescriptor {
            id: snapshot_id.into(),
            source_branch_ref: source.branch_ref,
            source_head: source.head,
            destination_head: destination_head.map(Into::into),
            virtual_ref: virtual_ref(snapshot_id),
            commit_bundle_bytes,
            staged_files: source.staged.files,
            staged_patch_bytes,
            unstaged_files: source.unstaged.files,
            unstaged_patch_bytes,
            untracked_files: u32::try_from(untracked_files.len())
                .map_err(|_| unavailable("The worktree snapshot has too many untracked files"))?,
            untracked_bytes,
            total_bytes,
        },
        commit_bundle,
        staged_patch,
        unstaged_patch,
        untracked_files,
    })
}

/// Applies a previously captured Orchid virtual snapshot to a clean target
/// worktree. It updates only a local virtual ref and the target checkout; it
/// does not create a user commit or push a Git ref.
pub fn apply_worktree_snapshot(
    worktree_root: &str,
    snapshot: &WorktreeSnapshot,
) -> Result<WorktreeInspection, RuntimePortError> {
    validate_snapshot(snapshot)?;
    let (root, runner) = open_worktree(worktree_root)?;
    let before = inspect(&root, &runner, None)?;
    if has_changes(&before) {
        return Err(unavailable(
            "The destination worktree must be clean before applying an Orchid snapshot",
        ));
    }
    if snapshot
        .descriptor
        .destination_head
        .as_deref()
        .is_some_and(|expected| expected != before.head)
    {
        return Err(unavailable(
            "The destination HEAD no longer matches the planned worktree snapshot",
        ));
    }

    if !snapshot.commit_bundle.is_empty() {
        let bundle = temporary_path(&snapshot.descriptor.id, "bundle");
        fs::write(&bundle, &snapshot.commit_bundle).map_err(unavailable)?;
        let imported = runner.required(
            &root,
            [
                OsString::from("bundle"),
                OsString::from("unbundle"),
                external_path(&bundle),
            ],
        );
        let _ = fs::remove_file(&bundle);
        imported.map_err(unavailable)?;
    }
    runner
        .required(
            &root,
            [
                "update-ref",
                &snapshot.descriptor.virtual_ref,
                &snapshot.descriptor.source_head,
            ],
        )
        .map_err(unavailable)?;
    runner
        .required(&root, ["reset", "--hard", &snapshot.descriptor.virtual_ref])
        .map_err(unavailable)?;

    apply_patch(
        &root,
        &runner,
        &snapshot.descriptor.id,
        "staged",
        &snapshot.staged_patch,
        true,
    )?;
    apply_patch(
        &root,
        &runner,
        &snapshot.descriptor.id,
        "unstaged",
        &snapshot.unstaged_patch,
        false,
    )?;
    for file in &snapshot.untracked_files {
        let path = safe_relative_path(&file.path)?;
        let target = root.join(path);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(unavailable)?;
        }
        fs::write(&target, &file.content).map_err(unavailable)?;
        set_executable(&target, file.executable).map_err(unavailable)?;
    }

    let completed = inspect(&root, &runner, None)?;
    if completed.head != snapshot.descriptor.source_head
        || completed.staged.files != snapshot.descriptor.staged_files
        || completed.unstaged.files != snapshot.descriptor.unstaged_files
        || completed.untracked.files != snapshot.descriptor.untracked_files
    {
        return Err(unavailable(
            "The destination worktree did not match the applied Orchid snapshot",
        ));
    }
    Ok(completed)
}

fn open_worktree(worktree_root: &str) -> Result<(PathBuf, GitRunner), RuntimePortError> {
    let root = Path::new(worktree_root)
        .canonicalize()
        .map_err(unavailable)?;
    if !root.is_dir() {
        return Err(unavailable("The worktree directory is unavailable"));
    }
    let executable = GitExecutable::discover().map_err(unavailable)?;
    let runner = GitRunner::new(&executable);
    let top_level = git_path(
        &root,
        runner
            .required(&root, ["rev-parse", "--show-toplevel"])
            .map_err(unavailable)?,
    )
    .map_err(unavailable)?;
    if top_level != root {
        return Err(unavailable(
            "Worktree inspection requires the top-level worktree directory",
        ));
    }
    Ok((root, runner))
}

fn inspect(
    root: &Path,
    runner: &GitRunner,
    compare_to_head: Option<&str>,
) -> Result<WorktreeInspection, RuntimePortError> {
    let head: String = git_commit_id(
        runner
            .required(root, ["rev-parse", "--verify", "HEAD^{commit}"])
            .map_err(unavailable)?,
    )
    .map_err(unavailable)?
    .as_str()
    .into();
    let branch_ref = runner
        .optional(root, ["symbolic-ref", "-q", "HEAD"])
        .map_err(unavailable)?
        .map(git_text)
        .transpose()
        .map_err(unavailable)?;
    let comparison = compare_to_head
        .map(|reference_head| compare_heads(root, runner, &head, reference_head))
        .transpose()?;
    Ok(WorktreeInspection {
        path: root.to_string_lossy().into_owned(),
        detached: branch_ref.is_none(),
        branch_ref,
        head,
        staged: change_summary(root, runner, true)?,
        unstaged: change_summary(root, runner, false)?,
        untracked: untracked_summary(root, runner)?,
        comparison,
    })
}

fn compare_heads(
    root: &Path,
    runner: &GitRunner,
    head: &str,
    reference_head: &str,
) -> Result<WorktreeHeadComparison, RuntimePortError> {
    let reference = GitCommitId::new(reference_head).map_err(unavailable)?;
    runner
        .required(
            root,
            [
                "cat-file",
                "-e",
                &format!("{}^{{commit}}", reference.as_str()),
            ],
        )
        .map_err(unavailable)?;
    let relation = if head == reference.as_str() {
        WorktreeHeadRelation::Equal
    } else if is_ancestor(root, runner, reference.as_str(), head)? {
        WorktreeHeadRelation::Ahead
    } else if is_ancestor(root, runner, head, reference.as_str())? {
        WorktreeHeadRelation::Behind
    } else {
        WorktreeHeadRelation::Diverged
    };
    Ok(WorktreeHeadComparison {
        reference_head: reference.as_str().into(),
        relation,
    })
}

fn is_ancestor(
    root: &Path,
    runner: &GitRunner,
    ancestor: &str,
    descendant: &str,
) -> Result<bool, RuntimePortError> {
    runner
        .optional(root, ["merge-base", "--is-ancestor", ancestor, descendant])
        .map(|outcome| outcome.is_some())
        .map_err(unavailable)
}

fn change_summary(
    root: &Path,
    runner: &GitRunner,
    staged: bool,
) -> Result<WorktreeChangeSummary, RuntimePortError> {
    let patch = if staged {
        staged_patch(root, runner)?
    } else {
        unstaged_patch(root, runner)?
    };
    let names = if staged {
        runner.required_snapshot(root, ["diff", "--name-only", "-z", "--cached"])
    } else {
        runner.required_snapshot(root, ["diff", "--name-only", "-z"])
    }
    .map_err(unavailable)?;
    Ok(WorktreeChangeSummary {
        files: count_nul_fields(&names)?,
        bytes: bytes(patch.len())?,
    })
}

fn staged_patch(root: &Path, runner: &GitRunner) -> Result<Vec<u8>, RuntimePortError> {
    runner
        .required_snapshot(
            root,
            [
                "diff",
                "--binary",
                "--full-index",
                "--no-ext-diff",
                "--cached",
            ],
        )
        .map_err(unavailable)
}

fn unstaged_patch(root: &Path, runner: &GitRunner) -> Result<Vec<u8>, RuntimePortError> {
    runner
        .required_snapshot(root, ["diff", "--binary", "--full-index", "--no-ext-diff"])
        .map_err(unavailable)
}

fn untracked_summary(
    root: &Path,
    runner: &GitRunner,
) -> Result<WorktreeChangeSummary, RuntimePortError> {
    let files = untracked_paths(root, runner)?;
    let count = u32::try_from(files.len())
        .map_err(|_| unavailable("The worktree has too many untracked files"))?;
    let bytes = files.iter().try_fold(0_u64, |total, file| {
        total
            .checked_add(file.bytes)
            .ok_or_else(|| unavailable("The worktree snapshot is too large"))
    })?;
    Ok(WorktreeChangeSummary {
        files: count,
        bytes,
    })
}

fn snapshot_untracked_files(
    root: &Path,
    runner: &GitRunner,
) -> Result<Vec<WorktreeSnapshotFile>, RuntimePortError> {
    untracked_paths(root, runner)?
        .into_iter()
        .filter(|file| eligible_snapshot_path(&file.relative_path))
        .map(|file| {
            let content = fs::read(root.join(&file.relative_path)).map_err(unavailable)?;
            Ok(WorktreeSnapshotFile {
                path: file.relative_path.to_string_lossy().into_owned(),
                content,
                executable: file.executable,
            })
        })
        .collect()
}

struct UntrackedPath {
    relative_path: PathBuf,
    bytes: u64,
    executable: bool,
}

fn untracked_paths(
    root: &Path,
    runner: &GitRunner,
) -> Result<Vec<UntrackedPath>, RuntimePortError> {
    let output = runner
        .required_snapshot(root, ["ls-files", "--others", "--exclude-standard", "-z"])
        .map_err(unavailable)?;
    output
        .split(|byte| *byte == 0)
        .filter(|field| !field.is_empty())
        .map(|field| {
            let path = std::str::from_utf8(field)
                .map(PathBuf::from)
                .map_err(|_| unavailable("Git returned a non-UTF-8 untracked path"))?;
            let relative_path = safe_relative_path(&path.to_string_lossy())?;
            let metadata = fs::symlink_metadata(root.join(&relative_path)).map_err(unavailable)?;
            if !metadata.file_type().is_file() {
                return Err(unavailable(
                    "Orchid snapshots support regular untracked files only",
                ));
            }
            Ok(UntrackedPath {
                relative_path,
                bytes: metadata.len(),
                executable: executable(&metadata),
            })
        })
        .collect()
}

fn commit_bundle(
    root: &Path,
    runner: &GitRunner,
    source_head: &str,
    destination_head: Option<&str>,
    snapshot_id: &str,
) -> Result<Vec<u8>, RuntimePortError> {
    if destination_head == Some(source_head) {
        return Ok(Vec::new());
    }
    let path = temporary_path(snapshot_id, "bundle");
    let capture_ref = format!("{}-capture", virtual_ref(snapshot_id));
    runner
        .required(root, ["update-ref", &capture_ref, source_head])
        .map_err(unavailable)?;
    let mut arguments = vec![
        OsString::from("bundle"),
        OsString::from("create"),
        external_path(&path),
        OsString::from(&capture_ref),
    ];
    if let Some(base) = destination_head {
        arguments.push(OsString::from(format!("^{base}")));
    }
    let result = runner.required_snapshot(root, arguments);
    let _ = runner.required(root, ["update-ref", "-d", &capture_ref]);
    let bundle = result
        .map_err(|error| unavailable(format!("Could not create the commit bundle: {error}")))
        .and_then(|_| fs::read(&path).map_err(unavailable));
    let _ = fs::remove_file(path);
    bundle
}

fn apply_patch(
    root: &Path,
    runner: &GitRunner,
    snapshot_id: &str,
    kind: &str,
    patch: &[u8],
    staged: bool,
) -> Result<(), RuntimePortError> {
    if patch.is_empty() {
        return Ok(());
    }
    let path = temporary_path(&format!("{snapshot_id}-{kind}"), "patch");
    fs::write(&path, patch).map_err(unavailable)?;
    let mut arguments = vec![
        OsString::from("apply"),
        OsString::from("--whitespace=nowarn"),
    ];
    if staged {
        arguments.push(OsString::from("--index"));
    }
    arguments.push(OsString::from("--"));
    arguments.push(external_path(&path));
    let applied = runner.required(root, arguments);
    let _ = fs::remove_file(path);
    applied.map(|_| ()).map_err(unavailable)
}

fn validate_snapshot(snapshot: &WorktreeSnapshot) -> Result<(), RuntimePortError> {
    validate_snapshot_id(&snapshot.descriptor.id)?;
    if snapshot.descriptor.virtual_ref != virtual_ref(&snapshot.descriptor.id) {
        return Err(unavailable("The Orchid snapshot virtual ref is invalid"));
    }
    GitCommitId::new(&snapshot.descriptor.source_head).map_err(unavailable)?;
    if let Some(head) = &snapshot.descriptor.destination_head {
        GitCommitId::new(head).map_err(unavailable)?;
    }
    if bytes(snapshot.commit_bundle.len())? != snapshot.descriptor.commit_bundle_bytes
        || bytes(snapshot.staged_patch.len())? != snapshot.descriptor.staged_patch_bytes
        || bytes(snapshot.unstaged_patch.len())? != snapshot.descriptor.unstaged_patch_bytes
    {
        return Err(unavailable(
            "The Orchid snapshot payload does not match its descriptor",
        ));
    }
    let untracked_bytes = snapshot
        .untracked_files
        .iter()
        .try_fold(0_u64, |total, file| {
            safe_relative_path(&file.path)?;
            checked_bytes(total, file.content.len())
        })?;
    if u32::try_from(snapshot.untracked_files.len())
        .map_err(|_| unavailable("The worktree snapshot has too many untracked files"))?
        != snapshot.descriptor.untracked_files
        || untracked_bytes != snapshot.descriptor.untracked_bytes
    {
        return Err(unavailable(
            "The Orchid snapshot payload does not match its descriptor",
        ));
    }
    let total_bytes = [
        snapshot.descriptor.commit_bundle_bytes,
        snapshot.descriptor.staged_patch_bytes,
        snapshot.descriptor.unstaged_patch_bytes,
        snapshot.descriptor.untracked_bytes,
    ]
    .into_iter()
    .try_fold(0_u64, |total, value| {
        total
            .checked_add(value)
            .ok_or_else(|| unavailable("The worktree snapshot is too large"))
    })?;
    if total_bytes != snapshot.descriptor.total_bytes {
        return Err(unavailable(
            "The Orchid snapshot payload does not match its descriptor",
        ));
    }
    Ok(())
}

fn validate_snapshot_id(value: &str) -> Result<(), RuntimePortError> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
    {
        Err(unavailable("The Orchid snapshot identity is invalid"))
    } else {
        Ok(())
    }
}

fn virtual_ref(snapshot_id: &str) -> String {
    format!("refs/orchid/snapshots/{snapshot_id}")
}

fn temporary_path(snapshot_id: &str, suffix: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "orchid-worktree-{snapshot_id}-{}.{suffix}",
        uuid::Uuid::new_v4()
    ))
}

fn safe_relative_path(value: &str) -> Result<PathBuf, RuntimePortError> {
    let path = PathBuf::from(value);
    if path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                Component::CurDir
                    | Component::ParentDir
                    | Component::RootDir
                    | Component::Prefix(_)
            )
        })
    {
        Err(unavailable(
            "The Orchid snapshot contains an invalid relative path",
        ))
    } else {
        Ok(path)
    }
}

fn eligible_snapshot_path(path: &Path) -> bool {
    let excluded_directory = path.components().any(|component| {
        matches!(component, Component::Normal(name) if matches!(name.to_str(), Some(".git" | "node_modules" | "target" | "dist" | ".cache")))
    });
    let excluded_file = path
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name == ".env" || name.starts_with(".env."));
    !excluded_directory && !excluded_file
}

fn count_nul_fields(bytes: &[u8]) -> Result<u32, RuntimePortError> {
    u32::try_from(
        bytes
            .split(|byte| *byte == 0)
            .filter(|field| !field.is_empty())
            .count(),
    )
    .map_err(|_| unavailable("The worktree has too many changed files"))
}

fn bytes(value: usize) -> Result<u64, RuntimePortError> {
    u64::try_from(value).map_err(|_| unavailable("The worktree snapshot is too large"))
}

fn checked_bytes(total: u64, value: usize) -> Result<u64, RuntimePortError> {
    total
        .checked_add(bytes(value)?)
        .ok_or_else(|| unavailable("The worktree snapshot is too large"))
}

fn has_changes(inspection: &WorktreeInspection) -> bool {
    inspection.staged.files > 0 || inspection.unstaged.files > 0 || inspection.untracked.files > 0
}

#[cfg(unix)]
fn executable(metadata: &fs::Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt;
    metadata.permissions().mode() & 0o111 != 0
}

#[cfg(not(unix))]
fn executable(_metadata: &fs::Metadata) -> bool {
    false
}

#[cfg(unix)]
fn set_executable(path: &Path, executable: bool) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut permissions = fs::metadata(path)?.permissions();
    let mode = permissions.mode();
    permissions.set_mode(if executable {
        mode | 0o111
    } else {
        mode & !0o111
    });
    fs::set_permissions(path, permissions)
}

#[cfg(not(unix))]
fn set_executable(_path: &Path, _executable: bool) -> std::io::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

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
    fn capture_and_apply_preserves_commits_index_worktree_and_untracked_files() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("source");
        let destination = directory.path().join("destination");
        fs::create_dir(&source).unwrap();
        git(&source, &["init", "-b", "main"]);
        git(&source, &["config", "user.name", "Fixture"]);
        git(
            &source,
            &["config", "user.email", "fixture@example.invalid"],
        );
        git(&source, &["config", "core.autocrlf", "false"]);
        fs::write(source.join("staged.txt"), "base\n").unwrap();
        fs::write(source.join("unstaged.txt"), "base\n").unwrap();
        git(&source, &["add", "."]);
        git(&source, &["commit", "-m", "base"]);
        let destination_head = git(&source, &["rev-parse", "HEAD"]);
        git(
            &source,
            &[
                "worktree",
                "add",
                "--detach",
                destination.to_str().unwrap(),
                "HEAD",
            ],
        );

        fs::write(source.join("committed.txt"), "local commit\n").unwrap();
        git(&source, &["add", "committed.txt"]);
        git(&source, &["commit", "-m", "local source commit"]);
        fs::write(source.join("staged.txt"), "in index\n").unwrap();
        git(&source, &["add", "staged.txt"]);
        fs::write(source.join("unstaged.txt"), "in worktree\n").unwrap();
        fs::write(source.join("untracked.txt"), "portable\n").unwrap();
        fs::write(source.join(".env"), "not portable\n").unwrap();

        let source_inspection =
            inspect_worktree(source.to_str().unwrap(), Some(&destination_head)).unwrap();
        assert_eq!(
            source_inspection.comparison.unwrap().relation,
            WorktreeHeadRelation::Ahead
        );
        assert_eq!(source_inspection.staged.files, 1);
        assert_eq!(source_inspection.unstaged.files, 1);
        assert_eq!(source_inspection.untracked.files, 2);

        let snapshot =
            capture_worktree_snapshot(source.to_str().unwrap(), Some(&destination_head), "move-1")
                .unwrap();
        assert!(!snapshot.commit_bundle.is_empty());
        assert_eq!(snapshot.descriptor.untracked_files, 1);
        assert_eq!(snapshot.untracked_files[0].path, "untracked.txt");

        let target_before = inspect_worktree(destination.to_str().unwrap(), None).unwrap();
        assert!(
            !has_changes(&target_before),
            "destination should remain clean: {target_before:?}; git diff names: {}",
            git(&destination, &["diff", "--name-only"])
        );
        let applied = apply_worktree_snapshot(destination.to_str().unwrap(), &snapshot).unwrap();
        assert_eq!(applied.head, snapshot.descriptor.source_head);
        assert_eq!(applied.staged.files, 1);
        assert_eq!(applied.unstaged.files, 1);
        assert_eq!(applied.untracked.files, 1);
        assert_eq!(
            fs::read_to_string(destination.join("committed.txt")).unwrap(),
            "local commit\n"
        );
        assert_eq!(
            fs::read_to_string(destination.join("staged.txt")).unwrap(),
            "in index\n"
        );
        assert_eq!(
            fs::read_to_string(destination.join("unstaged.txt")).unwrap(),
            "in worktree\n"
        );
        assert_eq!(
            fs::read_to_string(destination.join("untracked.txt")).unwrap(),
            "portable\n"
        );
        assert!(!destination.join(".env").exists());
        assert_eq!(
            git(&destination, &["diff", "--cached", "--name-only"]),
            "staged.txt"
        );
        assert_eq!(git(&destination, &["diff", "--name-only"]), "unstaged.txt");
    }
}
