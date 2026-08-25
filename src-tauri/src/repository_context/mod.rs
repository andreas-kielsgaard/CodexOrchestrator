mod command;
mod executable;
mod object;
mod path;
mod worktrees;

use command::{GitOutcome, HardenedGitRunner, LARGE_OUTPUT_LIMIT, SMALL_OUTPUT_LIMIT};
use sha2::{Digest, Sha256};
use std::{
    ffi::OsString,
    fmt,
    path::{Path, PathBuf},
};

pub(crate) use executable::GitExecutable;
pub(crate) use object::{FullRefName, ObjectId};
pub(crate) use path::CanonicalDirectory;
#[allow(unused_imports)]
pub(crate) use path::PathIdentity;
pub(crate) use worktrees::{WorktreeLocation, WorktreeRecord};

#[cfg(test)]
pub(crate) fn parse_worktree_porcelain(
    output: &[u8],
) -> Result<Vec<WorktreeRecord>, RepositoryContextError> {
    worktrees::parse(output)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum RepositoryContextErrorKind {
    MissingGit,
    PathUnavailable,
    RepositoryUnavailable,
    InvalidGitOutput,
    OutputLimitExceeded,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RepositoryContextError {
    pub(crate) kind: RepositoryContextErrorKind,
    message: &'static str,
}

impl RepositoryContextError {
    fn new(kind: RepositoryContextErrorKind, message: &'static str) -> Self {
        Self { kind, message }
    }
}

impl fmt::Display for RepositoryContextError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.message)
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) struct RepositoryId(String);

impl RepositoryId {
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RepositoryLocation {
    pub(crate) id: RepositoryId,
    pub(crate) top_level: CanonicalDirectory,
    pub(crate) common_dir: CanonicalDirectory,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RepositoryRef {
    pub(crate) full_name: FullRefName,
    pub(crate) object_id: ObjectId,
    pub(crate) symbolic_target: Option<FullRefName>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct CommitFacts {
    pub(crate) id: ObjectId,
    pub(crate) abbreviated_id: String,
    pub(crate) subject: String,
    pub(crate) body: String,
    pub(crate) author: String,
    pub(crate) committed_at: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Divergence {
    pub(crate) behind: usize,
    pub(crate) ahead: usize,
    pub(crate) merge_base: Option<ObjectId>,
}

pub(crate) struct RepositoryContext {
    runner: HardenedGitRunner,
}

impl RepositoryContext {
    pub(crate) fn discover_git() -> Result<Self, RepositoryContextError> {
        Ok(Self::new(GitExecutable::discover()?))
    }

    pub(crate) fn with_git(path: &Path) -> Result<Self, RepositoryContextError> {
        Ok(Self::new(GitExecutable::resolve(path)?))
    }

    pub(crate) fn new(executable: GitExecutable) -> Self {
        Self {
            runner: HardenedGitRunner::new(executable),
        }
    }

    pub(crate) fn repository(
        &self,
        candidate: &Path,
    ) -> Result<RepositoryLocation, RepositoryContextError> {
        let candidate = CanonicalDirectory::resolve(candidate)?;
        let top_level = self.canonical_git_path(
            candidate.path(),
            ["rev-parse", "--path-format=absolute", "--show-toplevel"],
        )?;
        let common_dir = self.canonical_git_path(
            candidate.path(),
            ["rev-parse", "--path-format=absolute", "--git-common-dir"],
        )?;
        let mut hash = Sha256::new();
        hash.update(b"repository-context/v1");
        hash.update(common_dir.identity().as_str().as_bytes());
        let id = format!("repository-{}", &format!("{:x}", hash.finalize())[..24]);
        Ok(RepositoryLocation {
            id: RepositoryId(id),
            top_level,
            common_dir,
        })
    }

    pub(crate) fn worktrees(
        &self,
        root: &Path,
    ) -> Result<Vec<WorktreeRecord>, RepositoryContextError> {
        let outcome = self.required(
            root,
            ["worktree", "list", "--porcelain"],
            LARGE_OUTPUT_LIMIT,
        )?;
        worktrees::parse(&outcome.stdout)
    }

    pub(crate) fn current_branch(
        &self,
        root: &Path,
    ) -> Result<Option<String>, RepositoryContextError> {
        self.optional_text(root, ["symbolic-ref", "--quiet", "--short", "HEAD"])
    }

    pub(crate) fn symbolic_ref_short(
        &self,
        root: &Path,
        reference: &FullRefName,
    ) -> Result<Option<String>, RepositoryContextError> {
        self.optional_text(
            root,
            ["symbolic-ref", "--quiet", "--short", reference.as_str()],
        )
    }

    pub(crate) fn resolve_commit(
        &self,
        root: &Path,
        revision: &str,
    ) -> Result<ObjectId, RepositoryContextError> {
        validate_revision(revision)?;
        ObjectId::parse(self.required_text(
            root,
            ["rev-parse", "--verify", &format!("{revision}^{{commit}}")],
        )?)
    }

    pub(crate) fn first_parent(
        &self,
        root: &Path,
        commit: &ObjectId,
    ) -> Result<Option<ObjectId>, RepositoryContextError> {
        self.optional_text(
            root,
            ["rev-parse", "--verify", &format!("{}^1", commit.as_str())],
        )?
        .map(ObjectId::parse)
        .transpose()
    }

    pub(crate) fn divergence(
        &self,
        root: &Path,
        left: &str,
        right: &str,
    ) -> Result<Divergence, RepositoryContextError> {
        validate_revision(left)?;
        validate_revision(right)?;
        let range = format!("{left}...{right}");
        let counts = self.required_text(root, ["rev-list", "--left-right", "--count", &range])?;
        let mut counts = counts.split_whitespace();
        let behind = counts
            .next()
            .and_then(|value| value.parse().ok())
            .ok_or_else(invalid)?;
        let ahead = counts
            .next()
            .and_then(|value| value.parse().ok())
            .ok_or_else(invalid)?;
        let merge_base = self
            .optional_text(root, ["merge-base", left, right])?
            .map(ObjectId::parse)
            .transpose()?;
        Ok(Divergence {
            behind,
            ahead,
            merge_base,
        })
    }

    pub(crate) fn is_ancestor(
        &self,
        root: &Path,
        ancestor: &str,
        descendant: &str,
    ) -> Result<bool, RepositoryContextError> {
        validate_revision(ancestor)?;
        validate_revision(descendant)?;
        Ok(self
            .runner
            .run(
                root,
                ["merge-base", "--is-ancestor", ancestor, descendant],
                SMALL_OUTPUT_LIMIT,
            )?
            .success)
    }

    pub(crate) fn local_branches(
        &self,
        root: &Path,
    ) -> Result<Vec<(String, ObjectId)>, RepositoryContextError> {
        let text = self.required_text(
            root,
            [
                "for-each-ref",
                "--format=%(refname:short)%00%(objectname)",
                "refs/heads",
            ],
        )?;
        text.lines()
            .filter(|line| !line.is_empty())
            .map(|line| {
                let (name, id) = line.split_once('\0').ok_or_else(invalid)?;
                validate_revision(name)?;
                Ok((name.to_owned(), ObjectId::parse(id)?))
            })
            .collect()
    }

    pub(crate) fn refs(&self, root: &Path) -> Result<Vec<RepositoryRef>, RepositoryContextError> {
        let text = self.required_text(
            root,
            [
                "for-each-ref",
                "--format=%(refname)%00%(objectname)%00%(symref)",
                "refs/heads",
                "refs/remotes",
                "refs/tags",
            ],
        )?;
        text.lines()
            .filter(|line| !line.is_empty())
            .map(|line| {
                let mut fields = line.split('\0');
                let full_name = FullRefName::parse(fields.next().ok_or_else(invalid)?)?;
                let object_id = ObjectId::parse(fields.next().ok_or_else(invalid)?)?;
                let symbolic_target = fields
                    .next()
                    .filter(|value| !value.is_empty())
                    .map(FullRefName::parse)
                    .transpose()?;
                Ok(RepositoryRef {
                    full_name,
                    object_id,
                    symbolic_target,
                })
            })
            .collect()
    }

    pub(crate) fn revision_list(
        &self,
        root: &Path,
        left: &str,
        right: &str,
        first_parent: bool,
        max_count: Option<usize>,
    ) -> Result<Vec<ObjectId>, RepositoryContextError> {
        validate_revision(left)?;
        validate_revision(right)?;
        let mut args = vec![OsString::from("rev-list")];
        if first_parent {
            args.push(OsString::from("--first-parent"));
        }
        if let Some(max) = max_count {
            args.push(OsString::from(format!("--max-count={max}")));
        }
        args.push(OsString::from(format!("{left}..{right}")));
        self.required(root, args, LARGE_OUTPUT_LIMIT)?
            .stdout
            .split(|byte| *byte == b'\n')
            .filter(|value| !value.is_empty())
            .map(|value| {
                std::str::from_utf8(value)
                    .map_err(|_| invalid())
                    .and_then(ObjectId::parse)
            })
            .collect()
    }

    pub(crate) fn first_parent_history(
        &self,
        root: &Path,
        revision: &str,
    ) -> Result<Vec<ObjectId>, RepositoryContextError> {
        validate_revision(revision)?;
        self.required(
            root,
            ["rev-list", "--first-parent", revision],
            LARGE_OUTPUT_LIMIT,
        )?
        .stdout
        .split(|byte| *byte == b'\n')
        .filter(|value| !value.is_empty())
        .map(|value| {
            std::str::from_utf8(value)
                .map_err(|_| invalid())
                .and_then(ObjectId::parse)
        })
        .collect()
    }

    pub(crate) fn commit_count(
        &self,
        root: &Path,
        left: &str,
        right: &str,
    ) -> Result<usize, RepositoryContextError> {
        validate_revision(left)?;
        validate_revision(right)?;
        self.required_text(root, ["rev-list", "--count", &format!("{left}..{right}")])?
            .parse()
            .map_err(|_| invalid())
    }

    pub(crate) fn commit_facts(
        &self,
        root: &Path,
        revision: &str,
    ) -> Result<CommitFacts, RepositoryContextError> {
        validate_revision(revision)?;
        let text = self.required_text(
            root,
            [
                "show",
                "-s",
                "--format=%H%x1f%h%x1f%an%x1f%aI%x1f%s%x1f%b",
                revision,
            ],
        )?;
        let mut fields = text.splitn(6, '\u{1f}');
        Ok(CommitFacts {
            id: ObjectId::parse(fields.next().ok_or_else(invalid)?)?,
            abbreviated_id: fields.next().ok_or_else(invalid)?.to_owned(),
            author: fields.next().ok_or_else(invalid)?.to_owned(),
            committed_at: fields.next().ok_or_else(invalid)?.to_owned(),
            subject: fields.next().ok_or_else(invalid)?.to_owned(),
            body: fields.next().unwrap_or_default().trim().to_owned(),
        })
    }

    pub(crate) fn commit_numstat(
        &self,
        root: &Path,
        commit: &ObjectId,
        parent: Option<&ObjectId>,
    ) -> Result<Vec<u8>, RepositoryContextError> {
        let args = if let Some(parent) = parent {
            vec![
                "diff",
                "--numstat",
                "-z",
                "--no-renames",
                parent.as_str(),
                commit.as_str(),
            ]
        } else {
            vec![
                "show",
                "--format=",
                "--numstat",
                "-z",
                "--no-renames",
                commit.as_str(),
            ]
        };
        Ok(self.required(root, args, LARGE_OUTPUT_LIMIT)?.stdout)
    }

    pub(crate) fn status_porcelain(&self, root: &Path) -> Result<Vec<u8>, RepositoryContextError> {
        Ok(self
            .required(
                root,
                ["status", "--porcelain=v1", "-z", "--untracked-files=all"],
                LARGE_OUTPUT_LIMIT,
            )?
            .stdout)
    }

    pub(crate) fn changed_paths(
        &self,
        root: &Path,
        left: &str,
        right: &str,
    ) -> Result<Vec<u8>, RepositoryContextError> {
        validate_revision(left)?;
        validate_revision(right)?;
        Ok(self
            .required(
                root,
                [
                    "diff",
                    "--name-only",
                    "-z",
                    "--no-renames",
                    &format!("{left}..{right}"),
                    "--",
                ],
                LARGE_OUTPUT_LIMIT,
            )?
            .stdout)
    }

    pub(crate) fn tracked_changes(&self, root: &Path) -> Result<Vec<u8>, RepositoryContextError> {
        Ok(self
            .required(
                root,
                ["diff", "--name-only", "-z", "--no-renames", "HEAD", "--"],
                LARGE_OUTPUT_LIMIT,
            )?
            .stdout)
    }

    pub(crate) fn untracked_paths(&self, root: &Path) -> Result<Vec<u8>, RepositoryContextError> {
        Ok(self
            .required(
                root,
                ["ls-files", "--others", "--exclude-standard", "-z"],
                LARGE_OUTPUT_LIMIT,
            )?
            .stdout)
    }

    pub(crate) fn path_exists_at(
        &self,
        root: &Path,
        commit: &ObjectId,
        relative: &str,
    ) -> Result<bool, RepositoryContextError> {
        validate_relative_path(relative)?;
        Ok(self
            .runner
            .run(
                root,
                ["cat-file", "-e", &format!("{}:{relative}", commit.as_str())],
                SMALL_OUTPUT_LIMIT,
            )?
            .success)
    }

    pub(crate) fn file_at(
        &self,
        root: &Path,
        commit: &ObjectId,
        relative: &str,
    ) -> Result<Vec<u8>, RepositoryContextError> {
        validate_relative_path(relative)?;
        Ok(self
            .required(
                root,
                ["show", &format!("{}:{relative}", commit.as_str())],
                LARGE_OUTPUT_LIMIT,
            )?
            .stdout)
    }

    pub(crate) fn equivalent_patch_count(
        &self,
        root: &Path,
        baseline: &str,
        selected: &str,
    ) -> Result<usize, RepositoryContextError> {
        validate_revision(baseline)?;
        validate_revision(selected)?;
        Ok(self
            .required_text(root, ["cherry", baseline, selected])?
            .lines()
            .filter(|line| line.starts_with('-'))
            .count())
    }

    fn canonical_git_path<I, S>(
        &self,
        root: &Path,
        args: I,
    ) -> Result<CanonicalDirectory, RepositoryContextError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<std::ffi::OsStr>,
    {
        let text = text(self.required(root, args, SMALL_OUTPUT_LIMIT)?.stdout)?;
        let path = PathBuf::from(text);
        let path = if path.is_absolute() {
            path
        } else {
            root.join(path)
        };
        CanonicalDirectory::resolve(&path)
    }

    fn required_text<I, S>(&self, root: &Path, args: I) -> Result<String, RepositoryContextError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<std::ffi::OsStr>,
    {
        text(self.required(root, args, SMALL_OUTPUT_LIMIT)?.stdout)
    }

    fn optional_text<I, S>(
        &self,
        root: &Path,
        args: I,
    ) -> Result<Option<String>, RepositoryContextError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<std::ffi::OsStr>,
    {
        let outcome = self.runner.run(root, args, SMALL_OUTPUT_LIMIT)?;
        if outcome.success {
            text(outcome.stdout).map(Some)
        } else {
            Ok(None)
        }
    }

    fn required<I, S>(
        &self,
        root: &Path,
        args: I,
        limit: usize,
    ) -> Result<GitOutcome, RepositoryContextError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<std::ffi::OsStr>,
    {
        let outcome = self.runner.run(root, args, limit)?;
        if outcome.success {
            Ok(outcome)
        } else {
            Err(RepositoryContextError::new(
                RepositoryContextErrorKind::RepositoryUnavailable,
                "Git could not inspect the repository.",
            ))
        }
    }
}

fn text(bytes: Vec<u8>) -> Result<String, RepositoryContextError> {
    String::from_utf8(bytes)
        .map(|value| value.trim().to_owned())
        .map_err(|_| invalid())
}

fn validate_revision(value: &str) -> Result<(), RepositoryContextError> {
    if value.is_empty()
        || value.len() > 512
        || value.starts_with('-')
        || value.contains("..")
        || value.contains("@{")
        || value.contains([' ', '~', '^', ':', '?', '*', '[', '\\'])
        || value.chars().any(char::is_control)
    {
        return Err(invalid());
    }
    Ok(())
}

fn validate_relative_path(value: &str) -> Result<(), RepositoryContextError> {
    let path = Path::new(value);
    if value.is_empty()
        || value.starts_with(['/', '\\'])
        || value.contains('\\')
        || value.chars().any(char::is_control)
        || path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                std::path::Component::ParentDir | std::path::Component::RootDir
            )
        })
    {
        return Err(invalid());
    }
    Ok(())
}

fn invalid() -> RepositoryContextError {
    RepositoryContextError::new(
        RepositoryContextErrorKind::InvalidGitOutput,
        "Git returned invalid repository facts.",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_identity_normalizes_windows_extended_prefix_and_separators() {
        let ordinary = super::path::PathIdentity::of(Path::new("C:\\Work\\Repo\\"));
        let extended = super::path::PathIdentity::of(Path::new(r"\\?\C:\Work\Repo"));
        if cfg!(windows) {
            assert_eq!(ordinary, extended);
        }
    }

    #[test]
    fn object_and_ref_types_reject_option_injection() {
        assert!(ObjectId::parse("a".repeat(40)).is_ok());
        assert!(ObjectId::parse("--help").is_err());
        assert!(FullRefName::parse("refs/heads/feature").is_ok());
        assert!(FullRefName::parse("--config").is_err());
    }
}
