use super::{
    command::{HardenedGitRunner, LARGE_OUTPUT_LIMIT},
    invalid_output, CanonicalDirectory, FullRefName, ObjectId, PathIdentity,
    RepositoryContextError, RepositoryId,
};
use sha2::{Digest, Sha256};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct WorktreeObservationId(String);

impl WorktreeObservationId {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn for_path(repository_id: &RepositoryId, path: &Path) -> Self {
        let mut hash = Sha256::new();
        hash.update(b"codex-orchestrator/worktree-observation/v1");
        hash.update(repository_id.as_str().as_bytes());
        hash.update(PathIdentity::of(path).as_str().as_bytes());
        Self(format!(
            "worktree-observation-{}",
            &format!("{:x}", hash.finalize())[..24]
        ))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WorktreeLocation {
    Available(CanonicalDirectory),
    Unavailable(PathBuf),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorktreeObservation {
    pub id: WorktreeObservationId,
    pub location: WorktreeLocation,
    pub head: ObjectId,
    pub head_ref: Option<FullRefName>,
    pub locked: bool,
    pub prunable: bool,
}

#[derive(Clone)]
pub struct WorktreeInventoryReader {
    runner: Arc<HardenedGitRunner>,
}

impl WorktreeInventoryReader {
    pub(super) fn new(runner: Arc<HardenedGitRunner>) -> Self {
        Self { runner }
    }

    /// Git's unfiltered worktree list starts with the main working tree.
    pub fn main_working_tree(
        &self,
        repository_id: &RepositoryId,
        root: &Path,
    ) -> Result<CanonicalDirectory, RepositoryContextError> {
        match self
            .list(repository_id, root)?
            .into_iter()
            .next()
            .map(|entry| entry.location)
        {
            Some(WorktreeLocation::Available(directory)) => Ok(directory),
            _ => Err(invalid_output()),
        }
    }

    pub fn list(
        &self,
        repository_id: &RepositoryId,
        root: &Path,
    ) -> Result<Vec<WorktreeObservation>, RepositoryContextError> {
        let output = self.runner.required(
            root,
            ["worktree", "list", "--porcelain", "-z"],
            LARGE_OUTPUT_LIMIT,
        )?;
        parse(repository_id, &output)
    }
}

fn parse(
    repository_id: &RepositoryId,
    output: &[u8],
) -> Result<Vec<WorktreeObservation>, RepositoryContextError> {
    let mut observations = Vec::new();
    let mut fields = Vec::new();
    for field in output.split(|byte| *byte == 0) {
        if field.is_empty() {
            if !fields.is_empty() {
                observations.push(parse_record(repository_id, &fields)?);
                fields.clear();
            }
        } else {
            fields.push(field);
        }
    }
    if !fields.is_empty() {
        observations.push(parse_record(repository_id, &fields)?);
    }
    if observations.is_empty() {
        Err(invalid_output())
    } else {
        Ok(observations)
    }
}

fn parse_record(
    repository_id: &RepositoryId,
    fields: &[&[u8]],
) -> Result<WorktreeObservation, RepositoryContextError> {
    let mut path = None;
    let mut head = None;
    let mut head_ref = None;
    let mut locked = false;
    let mut prunable = false;
    for field in fields {
        let field = std::str::from_utf8(field).map_err(|_| invalid_output())?;
        if let Some(value) = field.strip_prefix("worktree ") {
            path = Some(PathBuf::from(value));
        } else if let Some(value) = field.strip_prefix("HEAD ") {
            head = Some(ObjectId::parse(value)?);
        } else if let Some(value) = field.strip_prefix("branch ") {
            head_ref = Some(FullRefName::parse(value)?);
        } else if field == "locked" || field.starts_with("locked ") {
            locked = true;
        } else if field == "prunable" || field.starts_with("prunable ") {
            prunable = true;
        }
    }
    let path = path.ok_or_else(invalid_output)?;
    let id = WorktreeObservationId::for_path(repository_id, &path);
    let location = CanonicalDirectory::resolve(&path)
        .map(WorktreeLocation::Available)
        .unwrap_or_else(|_| WorktreeLocation::Unavailable(path));
    Ok(WorktreeObservation {
        id,
        location,
        head: head.ok_or_else(invalid_output)?,
        head_ref,
        locked,
        prunable,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parser_preserves_multiple_same_commit_worktrees_and_detached_state() {
        let directory = tempfile::tempdir().expect("directory");
        let first = directory.path().join("first");
        let second = directory.path().join("second");
        std::fs::create_dir(&first).expect("first");
        std::fs::create_dir(&second).expect("second");
        let output =
            format!(
            "worktree {}\0HEAD {}\0branch refs/heads/main\0\0worktree {}\0HEAD {}\0detached\0\0",
            first.display(), "a".repeat(40), second.display(), "a".repeat(40)
        );
        let repository_id = RepositoryId::test_value("repository-test");
        let records = parse(&repository_id, output.as_bytes()).expect("records");
        assert_eq!(records.len(), 2);
        assert_ne!(records[0].id, records[1].id);
        assert!(records[0].head_ref.is_some());
        assert!(records[1].head_ref.is_none());
    }
}
