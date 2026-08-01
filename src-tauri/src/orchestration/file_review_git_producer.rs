use super::repository::{
    FileReviewChangedFileWrite, FileReviewFactsError, FileReviewGitCaptureAuthorization,
    FileReviewGitCaptureAuthorizationError, SqliteOrchestrationRepository, StoreFileReviewFacts,
    StoreFileReviewFactsResult, FILE_REVIEW_ARTIFACT_MAX_BYTES, STORED_FILE_REVIEW_ARTIFACT_V1,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    env, fs,
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

const MAX_CHANGED_FILES: usize = 500;
const MAX_GIT_LIST_BYTES: usize = 256_000;
const MAX_FILE_BYTES: usize = 256_000;
const MAX_TEXT_LINES: usize = 20_000;

/// Internal producer request. The opaque authorization identity is the sole caller input.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ProduceFileReviewFromGit {
    pub(crate) capture_authorization_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ProducedFileReview {
    pub(crate) document_ref_id: String,
    pub(crate) artifact_id: String,
    pub(crate) opaque_reference: String,
    pub(crate) changed_file_count: usize,
    pub(crate) idempotent_replay: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum FileReviewGitProducerError {
    InvalidRequest,
    Unauthorized,
    InvalidAuthorization,
    RepositoryUnavailable,
    RepositoryMismatch,
    GitObjectUnavailable,
    InvalidGitState,
    LimitsExceeded,
    IncompleteArtifact,
    Conflict,
    Unavailable,
}

pub(crate) fn produce_file_review_from_git(
    repository: &SqliteOrchestrationRepository,
    request: ProduceFileReviewFromGit,
) -> Result<ProducedFileReview, FileReviewGitProducerError> {
    let authorization_id = request.capture_authorization_id;
    if authorization_id.trim().is_empty() || authorization_id.len() > 4_000 {
        return Err(FileReviewGitProducerError::InvalidRequest);
    }
    let authorization = repository
        .load_file_review_git_capture_authorization(&authorization_id)
        .map_err(map_authorization_error)?
        .ok_or(FileReviewGitProducerError::Unauthorized)?;
    let captured = capture_git_facts(&authorization)?;
    if captured.is_empty() {
        return Err(FileReviewGitProducerError::InvalidGitState);
    }

    let identity_seed = stable_identity_seed(&authorization);
    let document_ref_id = stable_id("file-review-document", &identity_seed);
    let artifact_id = stable_id("file-review-artifact", &identity_seed);
    let opaque_reference = stable_id("file-review-source", &identity_seed);
    let idempotency_key = stable_id("file-review-capture", &identity_seed);
    let (changed_files, files) = build_files(captured)?;
    let artifact = StoredArtifact {
        contract_version: STORED_FILE_REVIEW_ARTIFACT_V1,
        document_ref_id: &document_ref_id,
        artifact_id: &artifact_id,
        files,
    };
    validate_complete_artifact(&artifact, &changed_files)?;
    let payload = serde_json::to_vec(&artifact)
        .map_err(|_| FileReviewGitProducerError::IncompleteArtifact)?;
    if payload.is_empty() || payload.len() > FILE_REVIEW_ARTIFACT_MAX_BYTES {
        return Err(FileReviewGitProducerError::LimitsExceeded);
    }

    let count = changed_files.len();
    let outcome = repository
        .store_file_review_facts(StoreFileReviewFacts {
            document_ref_id: document_ref_id.clone(),
            epic_id: authorization.epic_id,
            sprint_id: authorization.sprint_id,
            provenance_id: authorization.provenance_id,
            opaque_reference: opaque_reference.clone(),
            title: "Changed files".into(),
            summary: Some(format!(
                "{count} changed files between immutable Git objects {} and {}.",
                short_object_id(&authorization.baseline_object_id),
                short_object_id(&authorization.current_object_id)
            )),
            artifact_id: artifact_id.clone(),
            payload,
            idempotency_key,
            changed_files,
        })
        .map_err(map_store_error)?;

    Ok(ProducedFileReview {
        document_ref_id,
        artifact_id,
        opaque_reference,
        changed_file_count: count,
        idempotent_replay: outcome == StoreFileReviewFactsResult::IdempotentReplay,
    })
}

fn map_authorization_error(
    error: FileReviewGitCaptureAuthorizationError,
) -> FileReviewGitProducerError {
    match error {
        FileReviewGitCaptureAuthorizationError::Invalid => {
            FileReviewGitProducerError::InvalidAuthorization
        }
        FileReviewGitCaptureAuthorizationError::Forbidden => {
            FileReviewGitProducerError::Unauthorized
        }
        FileReviewGitCaptureAuthorizationError::Conflict => FileReviewGitProducerError::Conflict,
        FileReviewGitCaptureAuthorizationError::Unavailable => {
            FileReviewGitProducerError::Unavailable
        }
    }
}

fn map_store_error(error: FileReviewFactsError) -> FileReviewGitProducerError {
    match error {
        FileReviewFactsError::Invalid => FileReviewGitProducerError::IncompleteArtifact,
        FileReviewFactsError::Forbidden => FileReviewGitProducerError::Unauthorized,
        FileReviewFactsError::Conflict => FileReviewGitProducerError::Conflict,
        FileReviewFactsError::Unavailable(_) => FileReviewGitProducerError::Unavailable,
    }
}

#[derive(Clone, Debug)]
struct CapturedChange {
    kind: ChangeKind,
    display_path: String,
    previous_display_path: Option<String>,
    old: Option<GitEntry>,
    new: Option<GitEntry>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ChangeKind {
    Added,
    Modified,
    Deleted,
    Renamed,
}

impl ChangeKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Added => "added",
            Self::Modified => "modified",
            Self::Deleted => "deleted",
            Self::Renamed => "renamed",
        }
    }
}

#[derive(Clone, Debug)]
enum GitEntry {
    Regular(Vec<u8>),
    Symlink(Vec<u8>),
    Gitlink(String),
}

fn capture_git_facts(
    authorization: &FileReviewGitCaptureAuthorization,
) -> Result<Vec<CapturedChange>, FileReviewGitProducerError> {
    let repository_root = authorized_canonical_root(&authorization.repository_root)?;
    let worktree_root = authorized_canonical_root(&authorization.worktree_root)?;
    verify_git_identity(&repository_root, &worktree_root)?;
    verify_commit(&worktree_root, &authorization.baseline_object_id)?;
    verify_commit(&worktree_root, &authorization.current_object_id)?;

    let output = run_git(
        &worktree_root,
        &[
            "diff-tree",
            "-r",
            "--no-commit-id",
            "--name-status",
            "-z",
            "--find-renames=50%",
            "--diff-filter=AMDRT",
            &authorization.baseline_object_id,
            &authorization.current_object_id,
        ],
        MAX_GIT_LIST_BYTES,
    )?;
    let mut changes = parse_name_status(&output)?;
    if changes.len() > MAX_CHANGED_FILES {
        return Err(FileReviewGitProducerError::LimitsExceeded);
    }
    for change in &mut changes {
        change.old = match change.kind {
            ChangeKind::Added => None,
            ChangeKind::Modified | ChangeKind::Deleted => Some(load_entry(
                &worktree_root,
                &authorization.baseline_object_id,
                &change.display_path,
            )?),
            ChangeKind::Renamed => Some(load_entry(
                &worktree_root,
                &authorization.baseline_object_id,
                change
                    .previous_display_path
                    .as_deref()
                    .ok_or(FileReviewGitProducerError::InvalidGitState)?,
            )?),
        };
        change.new = match change.kind {
            ChangeKind::Deleted => None,
            ChangeKind::Added | ChangeKind::Modified | ChangeKind::Renamed => Some(load_entry(
                &worktree_root,
                &authorization.current_object_id,
                &change.display_path,
            )?),
        };
    }
    changes.sort_by(|left, right| {
        left.display_path
            .as_bytes()
            .cmp(right.display_path.as_bytes())
            .then_with(|| {
                left.previous_display_path
                    .as_deref()
                    .unwrap_or("")
                    .as_bytes()
                    .cmp(
                        right
                            .previous_display_path
                            .as_deref()
                            .unwrap_or("")
                            .as_bytes(),
                    )
            })
    });
    Ok(changes)
}

fn authorized_canonical_root(value: &str) -> Result<PathBuf, FileReviewGitProducerError> {
    let supplied = Path::new(value);
    if !supplied.is_absolute()
        || supplied.components().any(|component| {
            matches!(
                component,
                std::path::Component::CurDir | std::path::Component::ParentDir
            )
        })
    {
        return Err(FileReviewGitProducerError::InvalidAuthorization);
    }
    let canonical = fs::canonicalize(supplied)
        .map_err(|_| FileReviewGitProducerError::RepositoryUnavailable)?;
    if path_identity(supplied) != path_identity(&canonical) {
        return Err(FileReviewGitProducerError::RepositoryMismatch);
    }
    Ok(canonical)
}

fn path_identity(path: &Path) -> String {
    let value = path.to_string_lossy();
    #[cfg(windows)]
    {
        value
            .strip_prefix(r"\\?\")
            .unwrap_or(&value)
            .replace('/', "\\")
            .trim_end_matches('\\')
            .to_lowercase()
    }
    #[cfg(not(windows))]
    {
        value.trim_end_matches('/').to_string()
    }
}

fn verify_git_identity(
    repository_root: &Path,
    worktree_root: &Path,
) -> Result<(), FileReviewGitProducerError> {
    let repository_top = git_path(repository_root, &["rev-parse", "--show-toplevel"])?;
    let worktree_top = git_path(worktree_root, &["rev-parse", "--show-toplevel"])?;
    if path_identity(&repository_top) != path_identity(repository_root)
        || path_identity(&worktree_top) != path_identity(worktree_root)
    {
        return Err(FileReviewGitProducerError::RepositoryMismatch);
    }
    let repository_common = git_path(
        repository_root,
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
    )?;
    let worktree_common = git_path(
        worktree_root,
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
    )?;
    if path_identity(&repository_common) != path_identity(&worktree_common) {
        return Err(FileReviewGitProducerError::RepositoryMismatch);
    }
    Ok(())
}

fn git_path(root: &Path, args: &[&str]) -> Result<PathBuf, FileReviewGitProducerError> {
    let output = run_git(root, args, 8_192)?;
    let value = std::str::from_utf8(&output)
        .map_err(|_| FileReviewGitProducerError::RepositoryMismatch)?
        .trim_end_matches(['\r', '\n']);
    let canonical =
        fs::canonicalize(value).map_err(|_| FileReviewGitProducerError::RepositoryMismatch)?;
    Ok(canonical)
}

fn verify_commit(root: &Path, object_id: &str) -> Result<(), FileReviewGitProducerError> {
    let typed = run_git(root, &["cat-file", "-t", object_id], 32).map_err(|error| match error {
        FileReviewGitProducerError::LimitsExceeded => error,
        _ => FileReviewGitProducerError::GitObjectUnavailable,
    })?;
    if typed != b"commit\n" && typed != b"commit\r\n" {
        return Err(FileReviewGitProducerError::GitObjectUnavailable);
    }
    Ok(())
}

fn parse_name_status(output: &[u8]) -> Result<Vec<CapturedChange>, FileReviewGitProducerError> {
    if output.is_empty() {
        return Ok(Vec::new());
    }
    if !output.ends_with(&[0]) {
        return Err(FileReviewGitProducerError::InvalidGitState);
    }
    let fields = output[..output.len() - 1]
        .split(|byte| *byte == 0)
        .collect::<Vec<_>>();
    let mut index = 0;
    let mut changes = Vec::new();
    let mut paths = HashSet::new();
    while index < fields.len() {
        let status = std::str::from_utf8(fields[index])
            .map_err(|_| FileReviewGitProducerError::InvalidGitState)?;
        index += 1;
        let (kind, previous_display_path) = match status.as_bytes().first().copied() {
            Some(b'A') => (ChangeKind::Added, None),
            Some(b'M') | Some(b'T') => (ChangeKind::Modified, None),
            Some(b'D') => (ChangeKind::Deleted, None),
            Some(b'R') if status[1..].bytes().all(|byte| byte.is_ascii_digit()) => {
                let old = fields
                    .get(index)
                    .ok_or(FileReviewGitProducerError::InvalidGitState)?;
                index += 1;
                (ChangeKind::Renamed, Some(safe_git_path(old)?))
            }
            _ => return Err(FileReviewGitProducerError::InvalidGitState),
        };
        let path = fields
            .get(index)
            .ok_or(FileReviewGitProducerError::InvalidGitState)?;
        index += 1;
        let display_path = safe_git_path(path)?;
        if !paths.insert(display_path.clone())
            || previous_display_path
                .as_ref()
                .is_some_and(|previous| previous == &display_path)
        {
            return Err(FileReviewGitProducerError::InvalidGitState);
        }
        changes.push(CapturedChange {
            kind,
            display_path,
            previous_display_path,
            old: None,
            new: None,
        });
    }
    Ok(changes)
}

fn safe_git_path(bytes: &[u8]) -> Result<String, FileReviewGitProducerError> {
    let value = std::str::from_utf8(bytes)
        .map_err(|_| FileReviewGitProducerError::InvalidGitState)?
        .to_string();
    if value.is_empty()
        || value.len() > 4_000
        || value.starts_with('/')
        || value.starts_with('\\')
        || value.starts_with("~/")
        || (value.len() >= 3
            && value.as_bytes()[0].is_ascii_alphabetic()
            && value.as_bytes()[1] == b':'
            && value.as_bytes()[2] == b'/')
        || value.contains('\\')
        || value.chars().any(char::is_control)
        || value
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
    {
        return Err(FileReviewGitProducerError::InvalidGitState);
    }
    Ok(value)
}

fn load_entry(
    root: &Path,
    commit: &str,
    path: &str,
) -> Result<GitEntry, FileReviewGitProducerError> {
    let literal_pathspec = format!(":(literal){path}");
    let output = run_git(
        root,
        &[
            "ls-tree",
            "-z",
            "--full-tree",
            commit,
            "--",
            &literal_pathspec,
        ],
        8_192,
    )?;
    if !output.ends_with(&[0]) || output[..output.len() - 1].contains(&0) {
        return Err(FileReviewGitProducerError::InvalidGitState);
    }
    let record = &output[..output.len() - 1];
    let tab = record
        .iter()
        .position(|byte| *byte == b'\t')
        .ok_or(FileReviewGitProducerError::InvalidGitState)?;
    if record.get(tab + 1..) != Some(path.as_bytes()) {
        return Err(FileReviewGitProducerError::InvalidGitState);
    }
    let metadata = std::str::from_utf8(&record[..tab])
        .map_err(|_| FileReviewGitProducerError::InvalidGitState)?;
    let parts = metadata.split(' ').collect::<Vec<_>>();
    if parts.len() != 3 || parts[2].len() < 40 {
        return Err(FileReviewGitProducerError::InvalidGitState);
    }
    match (parts[0], parts[1]) {
        ("120000", "blob") => Ok(GitEntry::Symlink(run_git(
            root,
            &["cat-file", "blob", parts[2]],
            MAX_FILE_BYTES,
        )?)),
        ("160000", "commit") => Ok(GitEntry::Gitlink(parts[2].to_string())),
        (mode, "blob") if mode.starts_with("100") => Ok(GitEntry::Regular(run_git(
            root,
            &["cat-file", "blob", parts[2]],
            MAX_FILE_BYTES,
        )?)),
        _ => Err(FileReviewGitProducerError::InvalidGitState),
    }
}

fn run_git(
    root: &Path,
    args: &[&str],
    limit: usize,
) -> Result<Vec<u8>, FileReviewGitProducerError> {
    let executable = resolve_git_executable()?;
    let executable_directory = executable
        .parent()
        .ok_or(FileReviewGitProducerError::RepositoryUnavailable)?;
    let child_path = minimal_child_path(executable_directory)?;
    let mut command = Command::new(executable);
    command
        .env_clear()
        .env("PATH", child_path)
        .env("LC_ALL", "C")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", null_device())
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .args([
            "--no-pager",
            "--no-replace-objects",
            "-c",
            "core.fsmonitor=false",
            "-c",
            "credential.interactive=false",
            "-c",
            "diff.external=",
        ])
        .arg("-c")
        .arg(format!("core.hooksPath={}", null_device()))
        .args(args)
        .current_dir(root)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    add_platform_child_environment(&mut command);
    let mut child = command
        .spawn()
        .map_err(|_| FileReviewGitProducerError::RepositoryUnavailable)?;
    let mut output = Vec::new();
    child
        .stdout
        .take()
        .ok_or(FileReviewGitProducerError::RepositoryUnavailable)?
        .take(limit as u64 + 1)
        .read_to_end(&mut output)
        .map_err(|_| FileReviewGitProducerError::RepositoryUnavailable)?;
    if output.len() > limit {
        let _ = child.kill();
        let _ = child.wait();
        return Err(FileReviewGitProducerError::LimitsExceeded);
    }
    if !child
        .wait()
        .map_err(|_| FileReviewGitProducerError::RepositoryUnavailable)?
        .success()
    {
        return Err(FileReviewGitProducerError::InvalidGitState);
    }
    Ok(output)
}

fn resolve_git_executable() -> Result<PathBuf, FileReviewGitProducerError> {
    let path = env::var_os("PATH").ok_or(FileReviewGitProducerError::RepositoryUnavailable)?;
    #[cfg(windows)]
    let names = ["git.exe", "git"];
    #[cfg(not(windows))]
    let names = ["git"];
    for directory in env::split_paths(&path) {
        for name in names {
            let candidate = directory.join(name);
            if candidate.is_file() {
                return fs::canonicalize(candidate)
                    .map_err(|_| FileReviewGitProducerError::RepositoryUnavailable);
            }
        }
    }
    Err(FileReviewGitProducerError::RepositoryUnavailable)
}

fn minimal_child_path(
    executable_directory: &Path,
) -> Result<std::ffi::OsString, FileReviewGitProducerError> {
    let mut directories = vec![executable_directory.to_path_buf()];
    #[cfg(windows)]
    if let Some(system_root) = env::var_os("SystemRoot") {
        let system32 = PathBuf::from(system_root).join("System32");
        if system32.is_dir() {
            directories.push(system32);
        }
    }
    env::join_paths(directories).map_err(|_| FileReviewGitProducerError::RepositoryUnavailable)
}

#[cfg(windows)]
fn add_platform_child_environment(command: &mut Command) {
    if let Some(system_root) = env::var_os("SystemRoot") {
        command
            .env("SystemRoot", &system_root)
            .env("WINDIR", system_root);
    }
}

#[cfg(not(windows))]
fn add_platform_child_environment(_command: &mut Command) {}

#[cfg(windows)]
fn null_device() -> &'static str {
    "NUL"
}

#[cfg(not(windows))]
fn null_device() -> &'static str {
    "/dev/null"
}

fn stable_identity_seed(authorization: &FileReviewGitCaptureAuthorization) -> String {
    stable_hash(&[
        &authorization.capture_authorization_id,
        &authorization.epic_id,
        &authorization.sprint_id,
        &authorization.provenance_id,
        &authorization.repository_id,
        &authorization.repository_root,
        &authorization.worktree_id,
        &authorization.worktree_root,
        &authorization.baseline_object_id,
        &authorization.current_object_id,
    ])
}

fn stable_id(label: &str, seed: &str) -> String {
    format!("{label}-{}", stable_hash(&[label, seed]))
}

fn stable_hash(parts: &[&str]) -> String {
    let mut hash = Sha256::new();
    for part in parts {
        hash.update((part.len() as u64).to_be_bytes());
        hash.update(part.as_bytes());
    }
    format!("{:x}", hash.finalize())
}

fn short_object_id(value: &str) -> &str {
    &value[..12.min(value.len())]
}

fn build_files(
    changes: Vec<CapturedChange>,
) -> Result<(Vec<FileReviewChangedFileWrite>, Vec<StoredFile>), FileReviewGitProducerError> {
    let mut memberships = Vec::with_capacity(changes.len());
    let mut files = Vec::with_capacity(changes.len());
    for change in changes {
        let old_identity = entry_identity(change.old.as_ref());
        let new_identity = entry_identity(change.new.as_ref());
        let changed_file_reference_id = stable_id(
            "file-review-change",
            &stable_hash(&[
                change.kind.as_str(),
                change.previous_display_path.as_deref().unwrap_or(""),
                &change.display_path,
                &old_identity,
                &new_identity,
            ]),
        );
        let (content, hunks, complete_text) =
            build_file_payload(change.old.as_ref(), change.new.as_ref())?;
        memberships.push(FileReviewChangedFileWrite {
            changed_file_reference_id: changed_file_reference_id.clone(),
            display_name: change.display_path,
            change_kind: change.kind.as_str().into(),
            previous_display_name: change.previous_display_path,
        });
        files.push(StoredFile {
            changed_file_reference_id,
            content,
            hunks,
            complete_text,
        });
    }
    Ok((memberships, files))
}

fn entry_identity(entry: Option<&GitEntry>) -> String {
    match entry {
        None => "absent".into(),
        Some(GitEntry::Regular(bytes)) => format!("blob:{}", stable_hash_bytes(bytes)),
        Some(GitEntry::Symlink(bytes)) => format!("symlink:{}", stable_hash_bytes(bytes)),
        Some(GitEntry::Gitlink(object)) => format!("gitlink:{object}"),
    }
}

fn stable_hash_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn build_file_payload(
    old: Option<&GitEntry>,
    new: Option<&GitEntry>,
) -> Result<(StoredContent, Vec<StoredHunk>, Option<CompleteText>), FileReviewGitProducerError> {
    if let Some((old_text, new_text)) = text_sides(old, new) {
        let old_lines = content_lines(old_text);
        let new_lines = content_lines(new_text);
        if old_lines.len() > MAX_TEXT_LINES || new_lines.len() > MAX_TEXT_LINES {
            return Err(FileReviewGitProducerError::LimitsExceeded);
        }
        let mut lines = Vec::with_capacity(old_lines.len() + new_lines.len());
        if old_lines == new_lines {
            lines.extend(
                old_lines
                    .iter()
                    .enumerate()
                    .map(|(index, text)| StoredLine {
                        kind: "context",
                        old_line_number: Some(index + 1),
                        new_line_number: Some(index + 1),
                        text: text.clone(),
                    }),
            );
        } else {
            lines.extend(
                old_lines
                    .iter()
                    .enumerate()
                    .map(|(index, text)| StoredLine {
                        kind: "deletion",
                        old_line_number: Some(index + 1),
                        new_line_number: None,
                        text: text.clone(),
                    }),
            );
            lines.extend(
                new_lines
                    .iter()
                    .enumerate()
                    .map(|(index, text)| StoredLine {
                        kind: "addition",
                        old_line_number: None,
                        new_line_number: Some(index + 1),
                        text: text.clone(),
                    }),
            );
        }
        let header = format!(
            "@@ -{},{} +{},{} @@",
            if old_lines.is_empty() { 0 } else { 1 },
            old_lines.len(),
            if new_lines.is_empty() { 0 } else { 1 },
            new_lines.len()
        );
        let new_bytes = new_text.as_bytes().to_vec();
        return Ok((
            StoredContent {
                encoding: "utf-8",
                bytes_base64: base64(&new_bytes),
            },
            vec![StoredHunk { header, lines }],
            Some(CompleteText {
                old_lines,
                new_lines,
            }),
        ));
    }

    let (encoding, bytes) = unsupported_content(old, new);
    Ok((
        StoredContent {
            encoding,
            bytes_base64: base64(bytes),
        },
        Vec::new(),
        None,
    ))
}

fn text_sides<'a>(
    old: Option<&'a GitEntry>,
    new: Option<&'a GitEntry>,
) -> Option<(&'a str, &'a str)> {
    fn regular_text(entry: Option<&GitEntry>) -> Option<Option<&str>> {
        match entry {
            None => Some(Some("")),
            Some(GitEntry::Regular(bytes)) if !looks_binary(bytes) => {
                Some(std::str::from_utf8(bytes).ok())
            }
            _ => None,
        }
    }
    Some((regular_text(old)??, regular_text(new)??))
}

fn unsupported_content<'a>(
    old: Option<&'a GitEntry>,
    new: Option<&'a GitEntry>,
) -> (&'static str, &'a [u8]) {
    for entry in [new, old].into_iter().flatten() {
        if let GitEntry::Regular(bytes) = entry {
            if looks_binary(bytes) {
                return ("binary", bytes);
            }
        }
    }
    for entry in [new, old].into_iter().flatten() {
        match entry {
            GitEntry::Regular(bytes) => return ("unsupported", bytes),
            GitEntry::Symlink(bytes) => return ("symlink", bytes),
            GitEntry::Gitlink(object) => return ("gitlink", object.as_bytes()),
        }
    }
    ("unsupported", &[])
}

fn looks_binary(bytes: &[u8]) -> bool {
    if bytes.contains(&0) {
        return true;
    }
    let sample = &bytes[..bytes.len().min(8_192)];
    let controls = sample
        .iter()
        .filter(|byte| **byte < 32 && !matches!(**byte, 9 | 10 | 13))
        .count();
    !sample.is_empty() && controls * 10 > sample.len()
}

fn content_lines(value: &str) -> Vec<String> {
    if value.is_empty() {
        return Vec::new();
    }
    let without_final_newline = value.strip_suffix('\n').unwrap_or(value);
    without_final_newline
        .split('\n')
        .map(str::to_string)
        .collect()
}

fn base64(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut output = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let first = chunk[0];
        let second = chunk.get(1).copied().unwrap_or(0);
        let third = chunk.get(2).copied().unwrap_or(0);
        output.push(TABLE[(first >> 2) as usize] as char);
        output.push(TABLE[(((first & 0x03) << 4) | (second >> 4)) as usize] as char);
        output.push(if chunk.len() > 1 {
            TABLE[(((second & 0x0f) << 2) | (third >> 6)) as usize] as char
        } else {
            '='
        });
        output.push(if chunk.len() > 2 {
            TABLE[(third & 0x3f) as usize] as char
        } else {
            '='
        });
    }
    output
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StoredArtifact<'a> {
    contract_version: &'static str,
    document_ref_id: &'a str,
    artifact_id: &'a str,
    files: Vec<StoredFile>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StoredFile {
    changed_file_reference_id: String,
    content: StoredContent,
    hunks: Vec<StoredHunk>,
    #[serde(skip)]
    complete_text: Option<CompleteText>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StoredContent {
    encoding: &'static str,
    bytes_base64: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StoredHunk {
    header: String,
    lines: Vec<StoredLine>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StoredLine {
    kind: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    old_line_number: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    new_line_number: Option<usize>,
    text: String,
}

struct CompleteText {
    old_lines: Vec<String>,
    new_lines: Vec<String>,
}

fn validate_complete_artifact(
    artifact: &StoredArtifact<'_>,
    memberships: &[FileReviewChangedFileWrite],
) -> Result<(), FileReviewGitProducerError> {
    if artifact.files.len() != memberships.len() || artifact.files.is_empty() {
        return Err(FileReviewGitProducerError::IncompleteArtifact);
    }
    for (file, membership) in artifact.files.iter().zip(memberships) {
        if file.changed_file_reference_id != membership.changed_file_reference_id {
            return Err(FileReviewGitProducerError::IncompleteArtifact);
        }
        let Some(complete) = &file.complete_text else {
            if !file.hunks.is_empty() {
                return Err(FileReviewGitProducerError::IncompleteArtifact);
            }
            continue;
        };
        if file.content.encoding != "utf-8" || file.hunks.len() != 1 {
            return Err(FileReviewGitProducerError::IncompleteArtifact);
        }
        let hunk = &file.hunks[0];
        let expected_header = format!(
            "@@ -{},{} +{},{} @@",
            if complete.old_lines.is_empty() { 0 } else { 1 },
            complete.old_lines.len(),
            if complete.new_lines.is_empty() { 0 } else { 1 },
            complete.new_lines.len()
        );
        if hunk.header != expected_header {
            return Err(FileReviewGitProducerError::IncompleteArtifact);
        }
        let old = hunk
            .lines
            .iter()
            .filter_map(|line| line.old_line_number.map(|number| (number, &line.text)))
            .collect::<Vec<_>>();
        let new = hunk
            .lines
            .iter()
            .filter_map(|line| line.new_line_number.map(|number| (number, &line.text)))
            .collect::<Vec<_>>();
        if old.len() != complete.old_lines.len()
            || new.len() != complete.new_lines.len()
            || old.iter().enumerate().any(|(index, (number, text))| {
                *number != index + 1 || *text != &complete.old_lines[index]
            })
            || new.iter().enumerate().any(|(index, (number, text))| {
                *number != index + 1 || *text != &complete.new_lines[index]
            })
        {
            return Err(FileReviewGitProducerError::IncompleteArtifact);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_unsafe_or_injected_git_paths() {
        for output in [
            b"M\0../escape.txt\0".as_slice(),
            b"M\0/absolute.txt\0".as_slice(),
            b"M\0C:/absolute.txt\0".as_slice(),
            b"M\0~/home-relative.txt\0".as_slice(),
            b"M\0safe\\ambiguous.txt\0".as_slice(),
            b"R100\0old.txt\0../new.txt\0".as_slice(),
        ] {
            assert!(matches!(
                parse_name_status(output),
                Err(FileReviewGitProducerError::InvalidGitState)
            ));
        }
    }

    #[test]
    fn rejects_incomplete_or_incoherent_payload_membership() {
        let artifact = StoredArtifact {
            contract_version: STORED_FILE_REVIEW_ARTIFACT_V1,
            document_ref_id: "document",
            artifact_id: "artifact",
            files: vec![StoredFile {
                changed_file_reference_id: "payload-file".into(),
                content: StoredContent {
                    encoding: "utf-8",
                    bytes_base64: String::new(),
                },
                hunks: Vec::new(),
                complete_text: Some(CompleteText {
                    old_lines: Vec::new(),
                    new_lines: Vec::new(),
                }),
            }],
        };
        let memberships = vec![FileReviewChangedFileWrite {
            changed_file_reference_id: "different-file".into(),
            display_name: "file.txt".into(),
            change_kind: "modified".into(),
            previous_display_name: None,
        }];
        assert_eq!(
            validate_complete_artifact(&artifact, &memberships),
            Err(FileReviewGitProducerError::IncompleteArtifact)
        );
    }
}
