use super::{
    CanonicalDirectory, FullRefName, ObjectId, RepositoryContextError, RepositoryContextErrorKind,
};
use std::path::PathBuf;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum WorktreeLocation {
    Available(CanonicalDirectory),
    Unavailable(PathBuf),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct WorktreeRecord {
    pub(crate) location: WorktreeLocation,
    pub(crate) head: ObjectId,
    pub(crate) head_ref: Option<FullRefName>,
    pub(crate) locked: bool,
    pub(crate) prunable: bool,
}

pub(super) fn parse(output: &[u8]) -> Result<Vec<WorktreeRecord>, RepositoryContextError> {
    let text = std::str::from_utf8(output).map_err(|_| invalid())?;
    let normalized = text.replace("\r\n", "\n");
    let mut records = Vec::new();
    for block in normalized
        .split("\n\n")
        .filter(|block| !block.trim().is_empty())
    {
        let mut path = None;
        let mut head = None;
        let mut head_ref = None;
        let mut locked = false;
        let mut prunable = false;
        for line in block.lines() {
            if let Some(value) = line.strip_prefix("worktree ") {
                path = Some(PathBuf::from(value));
            } else if let Some(value) = line.strip_prefix("HEAD ") {
                head = Some(ObjectId::parse(value)?);
            } else if let Some(value) = line.strip_prefix("branch ") {
                head_ref = Some(FullRefName::parse(value)?);
            } else if line == "locked" || line.starts_with("locked ") {
                locked = true;
            } else if line == "prunable" || line.starts_with("prunable ") {
                prunable = true;
            }
        }
        let path = path.ok_or_else(invalid)?;
        let location = CanonicalDirectory::resolve(&path)
            .map(WorktreeLocation::Available)
            .unwrap_or(WorktreeLocation::Unavailable(path));
        records.push(WorktreeRecord {
            location,
            head: head.ok_or_else(invalid)?,
            head_ref,
            locked,
            prunable,
        });
    }
    if records.is_empty() {
        return Err(invalid());
    }
    Ok(records)
}

fn invalid() -> RepositoryContextError {
    RepositoryContextError::new(
        RepositoryContextErrorKind::InvalidGitOutput,
        "Git returned an invalid worktree list.",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parser_keeps_detached_locked_and_unavailable_records_typed() {
        let directory = tempfile::tempdir().expect("directory");
        let available = directory.path().join("available");
        std::fs::create_dir(&available).expect("available");
        let unavailable = directory.path().join("missing");
        let output = format!(
            "worktree {}\r\nHEAD {}\r\nbranch refs/heads/main\r\nlocked reason\r\n\r\nworktree {}\r\nHEAD {}\r\ndetached\r\nprunable stale\r\n",
            available.display(),
            "a".repeat(40),
            unavailable.display(),
            "b".repeat(40)
        );
        let records = parse(output.as_bytes()).expect("records");
        assert_eq!(records.len(), 2);
        assert!(records[0].locked);
        assert!(records[0].head_ref.is_some());
        assert!(records[1].prunable);
        assert!(records[1].head_ref.is_none());
        assert!(matches!(
            records[1].location,
            WorktreeLocation::Unavailable(_)
        ));
    }

    #[test]
    fn rejects_option_like_or_malformed_refs() {
        assert!(FullRefName::parse("-c").is_err());
        assert!(FullRefName::parse("refs/heads/good").is_ok());
        assert!(FullRefName::parse("refs/heads/bad..name").is_err());
    }
}
