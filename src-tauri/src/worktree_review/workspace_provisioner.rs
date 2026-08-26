use super::domain::{
    RepositoryId, ReviewWorkspace, WorkspaceId, WorkspaceLifecycle, WorkspaceOwnership, WorktreeId,
    WorktreeLocation as StoredLocation,
};
use crate::repository_context::{
    GitExecutable, ObjectId, RepositoryContext, RepositoryIdentity, WorktreeLocation,
    WorktreeObservation, WorktreeObservationId,
};
use chrono::Utc;
use std::{
    ffi::{OsStr, OsString},
    fs,
    io::Read,
    path::{Component, Path, PathBuf},
    process::{Command, Stdio},
};

const UNTRACKED_LIST_LIMIT: u64 = 16 * 1024 * 1024;
const SNAPSHOT_FILE_LIMIT: u64 = 64 * 1024 * 1024;

pub(crate) struct ProvisionedWorktree {
    pub(crate) observation: WorktreeObservation,
    pub(crate) path: PathBuf,
}

pub(crate) struct PlannedWorktree {
    pub(crate) worktree_id: WorktreeObservationId,
    pub(crate) path: PathBuf,
}

pub(crate) struct WorktreeProvisioner {
    context: RepositoryContext,
    git: GitExecutable,
    root: PathBuf,
}

impl WorktreeProvisioner {
    pub(crate) fn open(context: RepositoryContext, root: PathBuf) -> Result<Self, String> {
        fs::create_dir_all(&root)
            .map_err(|_| "Worktree Review workspace storage is unavailable.".to_string())?;
        let root = root
            .canonicalize()
            .map_err(|_| "Worktree Review workspace storage is unavailable.".to_string())?;
        Ok(Self {
            git: context.git_executable().clone(),
            context,
            root,
        })
    }

    pub(crate) fn create_at_commit(
        &self,
        repository: &RepositoryIdentity,
        workspace_id: &str,
        object: &ObjectId,
    ) -> Result<ProvisionedWorktree, String> {
        let target = self.target(workspace_id)?;
        self.run_git(
            repository.top_level.path(),
            [
                OsString::from("worktree"),
                OsString::from("add"),
                OsString::from("--detach"),
                external_path(&target),
                OsString::from(object.as_str()),
            ],
        )?;
        self.verify(repository, &target, object)
    }

    pub(crate) fn plan_workspace(
        &self,
        repository: &RepositoryIdentity,
        workspace_id: WorkspaceId,
        ownership: WorkspaceOwnership,
    ) -> Result<ReviewWorkspace, String> {
        let planned = self.plan(repository, workspace_id.as_str())?;
        let now = Utc::now();
        Ok(ReviewWorkspace {
            id: workspace_id,
            repository_id: RepositoryId::new(repository.id.as_str())
                .map_err(|error| error.to_string())?,
            worktree_id: WorktreeId::new(planned.worktree_id.as_str())
                .map_err(|error| error.to_string())?,
            location: StoredLocation::new(planned.path.to_string_lossy().into_owned())
                .map_err(|error| error.to_string())?,
            ownership,
            lifecycle: WorkspaceLifecycle::Unverified,
            created_at: now,
            updated_at: now,
        })
    }

    fn plan(
        &self,
        repository: &RepositoryIdentity,
        workspace_id: &str,
    ) -> Result<PlannedWorktree, String> {
        let path = self.target(workspace_id)?;
        Ok(PlannedWorktree {
            worktree_id: WorktreeObservationId::for_path(&repository.id, &path),
            path,
        })
    }

    pub(crate) fn create_snapshot(
        &self,
        repository: &RepositoryIdentity,
        workspace_id: &str,
        source: &Path,
        expected_fingerprint: &str,
        expected_head: &ObjectId,
    ) -> Result<ProvisionedWorktree, String> {
        let (head_before, fingerprint_before) = self
            .context
            .status()
            .source_fingerprint(source)
            .map_err(|error| error.to_string())?;
        if &head_before != expected_head || fingerprint_before.as_str() != expected_fingerprint {
            return Err("The selected worktree changed before its snapshot was captured.".into());
        }
        let provisioned = self.create_at_commit(repository, workspace_id, expected_head)?;
        let snapshot_root = self.root.join("snapshot-materials");
        fs::create_dir_all(&snapshot_root)
            .map_err(|_| "Snapshot staging storage is unavailable.".to_string())?;
        let patch = snapshot_root.join(format!("{workspace_id}.patch"));
        self.capture_patch(source, &patch)?;
        if patch
            .metadata()
            .map(|metadata| metadata.len())
            .unwrap_or_default()
            > 0
        {
            self.run_git(
                &provisioned.path,
                [
                    OsString::from("apply"),
                    OsString::from("--binary"),
                    OsString::from("--whitespace=nowarn"),
                    external_path(&patch),
                ],
            )?;
        }
        self.copy_untracked(source, &provisioned.path)?;
        let (head_after, fingerprint_after) = self
            .context
            .status()
            .source_fingerprint(source)
            .map_err(|error| error.to_string())?;
        if head_after != head_before || fingerprint_after != fingerprint_before {
            return Err("The selected worktree changed while its snapshot was captured.".into());
        }
        let (materialized_head, materialized_fingerprint) = self
            .context
            .status()
            .source_fingerprint(&provisioned.path)
            .map_err(|error| error.to_string())?;
        if materialized_head != head_before || materialized_fingerprint != fingerprint_before {
            return Err(
                "The retained snapshot does not match the selected worktree source.".into(),
            );
        }
        let _ = fs::remove_file(&patch);
        Ok(provisioned)
    }

    fn target(&self, workspace_id: &str) -> Result<PathBuf, String> {
        if workspace_id.is_empty()
            || workspace_id.len() > 128
            || !workspace_id
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
        {
            return Err("The workspace identity is invalid.".into());
        }
        let target = self.root.join("worktrees").join(workspace_id);
        let parent = target
            .parent()
            .ok_or_else(|| "The workspace target is invalid.".to_string())?;
        fs::create_dir_all(parent)
            .map_err(|_| "Worktree Review workspace storage is unavailable.".to_string())?;
        let parent = parent
            .canonicalize()
            .map_err(|_| "Worktree Review workspace storage is unavailable.".to_string())?;
        if !parent.starts_with(&self.root)
            || target.exists()
            || fs::symlink_metadata(&target).is_ok()
        {
            return Err("The workspace target is not an unused contained directory.".into());
        }
        Ok(target)
    }

    fn verify(
        &self,
        repository: &RepositoryIdentity,
        target: &Path,
        object: &ObjectId,
    ) -> Result<ProvisionedWorktree, String> {
        let target = target
            .canonicalize()
            .map_err(|_| "The created worktree is unavailable.".to_string())?;
        if !target.starts_with(&self.root) {
            return Err("The created worktree escaped Worktree Review storage.".into());
        }
        let identity = self
            .context
            .identities()
            .inspect(&target)
            .map_err(|error| error.to_string())?;
        if identity.id != repository.id
            || identity.common_directory != repository.common_directory
            || identity.top_level.path() != target
        {
            return Err("The created worktree does not belong to the selected repository.".into());
        }
        let observation = self
            .context
            .worktrees()
            .list(&repository.id, repository.top_level.path())
            .map_err(|error| error.to_string())?
            .into_iter()
            .find(|worktree| {
                matches!(
                    &worktree.location,
                    WorktreeLocation::Available(directory) if directory.path() == target
                )
            })
            .ok_or_else(|| "Git did not register the created worktree.".to_string())?;
        if &observation.head != object || observation.head_ref.is_some() {
            return Err("The created worktree does not represent the selected commit.".into());
        }
        Ok(ProvisionedWorktree {
            observation,
            path: target,
        })
    }

    fn capture_patch(&self, source: &Path, destination: &Path) -> Result<(), String> {
        let file = fs::File::create(destination)
            .map_err(|_| "Snapshot patch storage is unavailable.".to_string())?;
        let status = self
            .git_command(source)
            .args([
                "diff",
                "--binary",
                "--full-index",
                "--no-ext-diff",
                "HEAD",
                "--",
            ])
            .stdout(Stdio::from(file))
            .status()
            .map_err(|_| "Git could not capture the worktree snapshot.".to_string())?;
        if !status.success() {
            return Err("Git could not capture the worktree snapshot.".into());
        }
        Ok(())
    }

    fn copy_untracked(&self, source: &Path, target: &Path) -> Result<(), String> {
        let mut child = self
            .git_command(source)
            .args(["ls-files", "--others", "--exclude-standard", "-z", "--"])
            .stdout(Stdio::piped())
            .spawn()
            .map_err(|_| "Git could not enumerate untracked snapshot files.".to_string())?;
        let mut output = Vec::new();
        child
            .stdout
            .take()
            .ok_or_else(|| "Git snapshot output is unavailable.".to_string())?
            .take(UNTRACKED_LIST_LIMIT + 1)
            .read_to_end(&mut output)
            .map_err(|_| "Git snapshot output is unavailable.".to_string())?;
        if output.len() as u64 > UNTRACKED_LIST_LIMIT {
            let _ = child.kill();
            let _ = child.wait();
            return Err("The untracked snapshot file list is too large.".into());
        }
        if !child
            .wait()
            .map_err(|_| "Git snapshot enumeration did not finish.".to_string())?
            .success()
        {
            return Err("Git could not enumerate untracked snapshot files.".into());
        }
        for relative in output
            .split(|byte| *byte == 0)
            .filter(|value| !value.is_empty())
        {
            let relative = std::str::from_utf8(relative)
                .map(PathBuf::from)
                .map_err(|_| "A snapshot path is not valid UTF-8.".to_string())?;
            validate_relative(&relative)?;
            let source_file = source.join(&relative);
            let metadata = fs::symlink_metadata(&source_file)
                .map_err(|_| "An untracked snapshot file is unavailable.".to_string())?;
            if !metadata.is_file()
                || metadata.file_type().is_symlink()
                || metadata.len() > SNAPSHOT_FILE_LIMIT
            {
                return Err("An untracked snapshot entry is not a bounded regular file.".into());
            }
            let target_file = safe_target(target, &relative)?;
            if let Some(parent) = target_file.parent() {
                fs::create_dir_all(parent).map_err(|_| {
                    "The snapshot checkout cannot receive an untracked file.".to_string()
                })?;
            }
            fs::copy(&source_file, &target_file).map_err(|_| {
                "An untracked file could not be copied into the snapshot.".to_string()
            })?;
        }
        Ok(())
    }

    fn run_git<I, S>(&self, root: &Path, arguments: I) -> Result<(), String>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let status = self
            .git_command(root)
            .args(arguments)
            .status()
            .map_err(|_| "Git could not provision the Worktree Review checkout.".to_string())?;
        status
            .success()
            .then_some(())
            .ok_or_else(|| "Git could not provision the Worktree Review checkout.".to_string())
    }

    fn git_command(&self, root: &Path) -> Command {
        let mut command = Command::new(self.git.path());
        command
            .current_dir(root)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GIT_OPTIONAL_LOCKS", "0")
            .env("LC_ALL", "C")
            .arg("--no-pager")
            .arg("--no-replace-objects")
            .arg("--literal-pathspecs")
            .arg("-c")
            .arg("core.fsmonitor=false")
            .arg("-c")
            .arg("credential.interactive=false")
            .arg("-c")
            .arg(format!("core.hooksPath={}", null_device()));
        command
    }
}

fn validate_relative(path: &Path) -> Result<(), String> {
    if path.as_os_str().is_empty()
        || path.is_absolute()
        || path
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
    {
        return Err("A snapshot path escapes the selected worktree.".into());
    }
    Ok(())
}

fn safe_target(root: &Path, relative: &Path) -> Result<PathBuf, String> {
    let mut current = root.to_path_buf();
    let components = relative.components().collect::<Vec<_>>();
    for component in &components[..components.len().saturating_sub(1)] {
        let Component::Normal(component) = component else {
            return Err("A snapshot path is invalid.".into());
        };
        current.push(component);
        if let Ok(metadata) = fs::symlink_metadata(&current) {
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                return Err("A snapshot path crosses a non-directory checkout entry.".into());
            }
        }
    }
    let target = root.join(relative);
    if target.starts_with(root) {
        Ok(target)
    } else {
        Err("A snapshot path escapes its retained checkout.".into())
    }
}

fn external_path(path: &Path) -> OsString {
    let value = path.to_string_lossy();
    if let Some(unc) = value.strip_prefix(r"\\?\UNC\") {
        return OsString::from(format!(r"\\{unc}"));
    }
    OsString::from(value.strip_prefix(r"\\?\").unwrap_or(&value))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_a_retained_detached_checkout_at_the_exact_object() {
        let fixture = RepositoryFixture::new();
        let provisioner = WorktreeProvisioner::open(
            fixture.context.clone(),
            fixture.directory.path().join("review"),
        )
        .unwrap();
        let planned = provisioner
            .plan_workspace(
                &fixture.repository,
                WorkspaceId::new("workspace-exact").unwrap(),
                WorkspaceOwnership::OwnedBuildWorktree {
                    build_id: super::super::domain::ReviewBuildId::new("build-exact").unwrap(),
                },
            )
            .unwrap();

        let created = provisioner
            .create_at_commit(&fixture.repository, "workspace-exact", &fixture.head)
            .unwrap();

        assert!(created.path.is_dir());
        assert_eq!(
            created.observation.id.as_str(),
            planned.worktree_id.as_str()
        );
        assert_eq!(
            created.path,
            PathBuf::from(planned.location.as_str())
                .canonicalize()
                .unwrap()
        );
        assert_eq!(planned.lifecycle, WorkspaceLifecycle::Unverified);
        assert_eq!(created.observation.head, fixture.head);
        assert!(created.observation.head_ref.is_none());
        assert_eq!(
            fs::read_to_string(created.path.join("tracked.txt")).unwrap(),
            "base\n"
        );
        let (_, fingerprint) = fixture
            .context
            .status()
            .source_fingerprint(&created.path)
            .unwrap();
        assert_eq!(
            fingerprint,
            fixture
                .context
                .status()
                .clean_source_fingerprint(&fixture.head)
        );
    }

    #[test]
    fn snapshot_materialization_preserves_tracked_and_untracked_source_bytes() {
        let fixture = RepositoryFixture::new();
        fs::write(fixture.root.join("tracked.txt"), "edited\n").unwrap();
        fs::write(fixture.root.join("new.txt"), "untracked\n").unwrap();
        let (_, fingerprint) = fixture
            .context
            .status()
            .source_fingerprint(&fixture.root)
            .unwrap();
        let provisioner = WorktreeProvisioner::open(
            fixture.context.clone(),
            fixture.directory.path().join("review"),
        )
        .unwrap();

        let snapshot = provisioner
            .create_snapshot(
                &fixture.repository,
                "workspace-snapshot",
                &fixture.root,
                fingerprint.as_str(),
                &fixture.head,
            )
            .unwrap();

        assert_eq!(
            fs::read_to_string(snapshot.path.join("tracked.txt")).unwrap(),
            "edited\n"
        );
        assert_eq!(
            fs::read_to_string(snapshot.path.join("new.txt")).unwrap(),
            "untracked\n"
        );
        let (snapshot_head, snapshot_fingerprint) = fixture
            .context
            .status()
            .source_fingerprint(&snapshot.path)
            .unwrap();
        assert_eq!(snapshot_head, fixture.head);
        assert_eq!(snapshot_fingerprint, fingerprint);
    }

    struct RepositoryFixture {
        directory: tempfile::TempDir,
        root: PathBuf,
        context: RepositoryContext,
        repository: RepositoryIdentity,
        head: ObjectId,
    }

    impl RepositoryFixture {
        fn new() -> Self {
            let directory = tempfile::tempdir().unwrap();
            let root = directory.path().join("repository");
            fs::create_dir(&root).unwrap();
            git(&root, &["init"]);
            git(&root, &["config", "user.name", "Worktree Review Test"]);
            git(
                &root,
                &["config", "user.email", "worktree-review@example.invalid"],
            );
            git(&root, &["config", "core.autocrlf", "false"]);
            fs::write(root.join("tracked.txt"), "base\n").unwrap();
            git(&root, &["add", "tracked.txt"]);
            git(
                &root,
                &["-c", "commit.gpgsign=false", "commit", "-m", "base"],
            );
            let context = RepositoryContext::discover().unwrap();
            let repository = context.identities().inspect(&root).unwrap();
            let head = ObjectId::parse(git_output(&root, &["rev-parse", "HEAD"])).unwrap();
            Self {
                directory,
                root,
                context,
                repository,
                head,
            }
        }
    }

    fn git(root: &Path, arguments: &[&str]) {
        git_output(root, arguments);
    }

    fn git_output(root: &Path, arguments: &[&str]) -> String {
        let output = Command::new("git")
            .current_dir(root)
            .args(arguments)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git {:?} failed: {}",
            arguments,
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).unwrap().trim().to_owned()
    }
}

#[cfg(windows)]
fn null_device() -> &'static str {
    "NUL"
}

#[cfg(not(windows))]
fn null_device() -> &'static str {
    "/dev/null"
}
