use super::{
    command::{HardenedGitRunner, LARGE_OUTPUT_LIMIT, SMALL_OUTPUT_LIMIT},
    invalid_output, ObjectId, RepositoryContextError, RepositoryContextErrorKind,
};
use sha2::{Digest, Sha256};
use std::{fs::File, io::Read, path::Path, sync::Arc};

const SOURCE_CONTENT_LIMIT: u64 = 64 * 1024 * 1024;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct RepositoryStatus {
    pub(crate) staged_paths: usize,
    pub(crate) unstaged_paths: usize,
    pub(crate) untracked_paths: usize,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) struct SourceStateFingerprint(String);

impl SourceStateFingerprint {
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone)]
pub(crate) struct RepositoryStatusReader {
    runner: Arc<HardenedGitRunner>,
}

impl RepositoryStatusReader {
    pub(super) fn new(runner: Arc<HardenedGitRunner>) -> Self {
        Self { runner }
    }

    pub(crate) fn status(&self, root: &Path) -> Result<RepositoryStatus, RepositoryContextError> {
        let output = self.status_output(root)?;
        parse_status(&output)
    }

    /// Fingerprints the checked-out source bytes, including tracked changes and untracked files.
    /// Staging is exposed separately by `status` and does not alter build input identity.
    pub(crate) fn source_fingerprint(
        &self,
        root: &Path,
    ) -> Result<(ObjectId, SourceStateFingerprint), RepositoryContextError> {
        let head = ObjectId::parse(text(self.runner.required(
            root,
            ["rev-parse", "--verify", "HEAD^{commit}"],
            SMALL_OUTPUT_LIMIT,
        )?)?)?;
        let tracked = self.runner.required(
            root,
            [
                "diff",
                "--binary",
                "--no-ext-diff",
                "--no-textconv",
                "HEAD",
                "--",
            ],
            LARGE_OUTPUT_LIMIT,
        )?;
        let untracked = self.runner.required(
            root,
            ["ls-files", "--others", "--exclude-standard", "-z", "--"],
            LARGE_OUTPUT_LIMIT,
        )?;
        let mut hash = Sha256::new();
        hash.update(b"codex-orchestrator/source-state/v1");
        write_field(&mut hash, head.as_str().as_bytes());
        write_field(&mut hash, &tracked);
        for relative in untracked
            .split(|byte| *byte == 0)
            .filter(|value| !value.is_empty())
        {
            let relative_text = std::str::from_utf8(relative).map_err(|_| invalid_output())?;
            let path = root.join(relative_text);
            let metadata = path.metadata().map_err(|_| unavailable())?;
            if !metadata.is_file() {
                continue;
            }
            if metadata.len() > SOURCE_CONTENT_LIMIT {
                return Err(limit_exceeded());
            }
            write_field(&mut hash, relative);
            hash.update(metadata.len().to_be_bytes());
            let mut file = File::open(path).map_err(|_| unavailable())?;
            let mut remaining = metadata.len();
            let mut buffer = [0_u8; 64 * 1024];
            while remaining > 0 {
                let count = file.read(&mut buffer).map_err(|_| unavailable())?;
                if count == 0 {
                    return Err(unavailable());
                }
                hash.update(&buffer[..count]);
                remaining = remaining.saturating_sub(count as u64);
            }
        }
        Ok((
            head,
            SourceStateFingerprint(format!("sha256:{:x}", hash.finalize())),
        ))
    }

    pub(crate) fn clean_source_fingerprint(&self, head: &ObjectId) -> SourceStateFingerprint {
        let mut hash = Sha256::new();
        hash.update(b"codex-orchestrator/source-state/v1");
        write_field(&mut hash, head.as_str().as_bytes());
        write_field(&mut hash, &[]);
        SourceStateFingerprint(format!("sha256:{:x}", hash.finalize()))
    }

    fn status_output(&self, root: &Path) -> Result<Vec<u8>, RepositoryContextError> {
        self.runner.required(
            root,
            ["status", "--porcelain=v2", "-z", "--untracked-files=all"],
            LARGE_OUTPUT_LIMIT,
        )
    }
}

fn parse_status(output: &[u8]) -> Result<RepositoryStatus, RepositoryContextError> {
    let mut status = RepositoryStatus::default();
    let mut fields = output.split(|byte| *byte == 0);
    while let Some(field) = fields.next() {
        if field.is_empty() {
            continue;
        }
        match field.first().copied() {
            Some(b'1' | b'2' | b'u') => {
                if field.get(1) != Some(&b' ') || field.len() < 4 {
                    return Err(invalid_output());
                }
                let staged = field[2];
                let unstaged = field[3];
                if staged != b'.' {
                    status.staged_paths += 1;
                }
                if unstaged != b'.' {
                    status.unstaged_paths += 1;
                }
                if field[0] == b'2' {
                    fields.next().ok_or_else(invalid_output)?;
                }
            }
            Some(b'?') if field.get(1) == Some(&b' ') => status.untracked_paths += 1,
            Some(b'!') if field.get(1) == Some(&b' ') => {}
            _ => return Err(invalid_output()),
        }
    }
    Ok(status)
}

fn write_field(hash: &mut Sha256, value: &[u8]) {
    hash.update((value.len() as u64).to_be_bytes());
    hash.update(value);
}

fn text(bytes: Vec<u8>) -> Result<String, RepositoryContextError> {
    let value = std::str::from_utf8(&bytes)
        .map_err(|_| invalid_output())?
        .trim();
    if value.is_empty() {
        Err(invalid_output())
    } else {
        Ok(value.to_owned())
    }
}

fn unavailable() -> RepositoryContextError {
    RepositoryContextError::new(
        RepositoryContextErrorKind::PathUnavailable,
        "Repository source content changed while it was inspected.",
    )
}

fn limit_exceeded() -> RepositoryContextError {
    RepositoryContextError::new(
        RepositoryContextErrorKind::OutputLimitExceeded,
        "Repository source content exceeds the inspection limit.",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_staged_unstaged_and_untracked_facts_separately() {
        let output = b"1 M. N... 100644 100644 100644 a a staged.txt\0\
                       1 .M N... 100644 100644 100644 a a unstaged.txt\0\
                       1 MM N... 100644 100644 100644 a a both.txt\0\
                       ? new.txt\0";
        assert_eq!(
            parse_status(output).unwrap(),
            RepositoryStatus {
                staged_paths: 2,
                unstaged_paths: 2,
                untracked_paths: 1,
            }
        );
    }
}
