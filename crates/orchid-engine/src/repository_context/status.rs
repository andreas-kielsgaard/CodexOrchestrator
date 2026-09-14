use super::{
    command::{HardenedGitRunner, LARGE_OUTPUT_LIMIT},
    invalid_output, RepositoryContextError,
};
use std::{path::Path, sync::Arc};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RepositoryStatus {
    pub staged_paths: usize,
    pub unstaged_paths: usize,
    pub untracked_paths: usize,
}

#[derive(Clone)]
pub struct RepositoryStatusReader {
    runner: Arc<HardenedGitRunner>,
}

impl RepositoryStatusReader {
    pub(super) fn new(runner: Arc<HardenedGitRunner>) -> Self {
        Self { runner }
    }

    pub fn status(&self, root: &Path) -> Result<RepositoryStatus, RepositoryContextError> {
        let output = self.status_output(root)?;
        parse_status(&output)
    }

    pub fn changed_files(
        &self,
        root: &Path,
    ) -> Result<(RepositoryStatus, Vec<std::path::PathBuf>), RepositoryContextError> {
        let output = self.status_output(root)?;
        let status = parse_status(&output)?;
        let mut fields = output.split(|byte| *byte == 0);
        let mut paths = Vec::new();
        while let Some(field) = fields.next() {
            let columns = match field.first() {
                Some(b'1') => 9,
                Some(b'2') => 10,
                Some(b'u') => 11,
                Some(b'?') => 2,
                _ => continue,
            };
            let name = field
                .splitn(columns, |byte| *byte == b' ')
                .last()
                .ok_or_else(invalid_output)?;
            if let Ok(name) = std::str::from_utf8(name) {
                paths.push(std::path::PathBuf::from(name));
            }
            if field.first() == Some(&b'2') {
                fields.next();
            }
            if paths.len() >= 512 {
                break;
            }
        }
        Ok((status, paths))
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
