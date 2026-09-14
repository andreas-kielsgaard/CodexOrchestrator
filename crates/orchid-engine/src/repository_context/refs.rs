use super::{
    command::{HardenedGitRunner, LARGE_OUTPUT_LIMIT, SMALL_OUTPUT_LIMIT},
    commits::CommitFacts,
    invalid_output, RepositoryContextError,
};
use std::{path::Path, sync::Arc};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ObjectId(String);

impl ObjectId {
    pub fn parse(value: impl Into<String>) -> Result<Self, RepositoryContextError> {
        let value = value.into().to_ascii_lowercase();
        if matches!(value.len(), 40 | 64) && value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            Ok(Self(value))
        } else {
            Err(invalid_output())
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct FullRefName(String);

impl FullRefName {
    pub fn parse(value: impl Into<String>) -> Result<Self, RepositoryContextError> {
        let value = value.into();
        if value.starts_with("refs/")
            && value.len() <= 512
            && !value.contains("..")
            && !value.contains("@{")
            && !value.contains([' ', '~', '^', ':', '?', '*', '[', '\\'])
            && !value.ends_with(['/', '.'])
            && !value.chars().any(char::is_control)
        {
            Ok(Self(value))
        } else {
            Err(invalid_output())
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn branch_name(&self) -> Option<&str> {
        self.0.strip_prefix("refs/heads/")
    }

    pub fn display_name(&self) -> &str {
        self.0
            .strip_prefix("refs/heads/")
            .or_else(|| self.0.strip_prefix("refs/remotes/"))
            .or_else(|| self.0.strip_prefix("refs/tags/"))
            .unwrap_or(&self.0)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BranchRef {
    pub full_name: FullRefName,
    pub object_id: ObjectId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BranchSummary {
    pub branch: BranchRef,
    pub tip: CommitFacts,
    pub ahead: usize,
    pub behind: usize,
}

#[derive(Clone)]
pub struct ReferenceReader {
    runner: Arc<HardenedGitRunner>,
}

impl ReferenceReader {
    pub(super) fn new(runner: Arc<HardenedGitRunner>) -> Self {
        Self { runner }
    }

    pub fn local_branches(&self, root: &Path) -> Result<Vec<BranchRef>, RepositoryContextError> {
        let output = self.runner.required(
            root,
            [
                "for-each-ref",
                "--format=%(refname)%00%(objectname)",
                "refs/heads",
            ],
            LARGE_OUTPUT_LIMIT,
        )?;
        parse_branches(&output)
    }

    /// Reads the current local-branch direction and tip presentation in one Git process.
    /// Historical commits deliberately live on `CommitReader` instead of this default view.
    pub fn local_branch_summaries(
        &self,
        root: &Path,
        default_branch: Option<&FullRefName>,
    ) -> Result<Vec<BranchSummary>, RepositoryContextError> {
        let direction = default_branch
            .map(|reference| format!("%(ahead-behind:{})", reference.as_str()))
            .unwrap_or_else(|| "0 0".into());
        let format = format!(
            "--format=%(refname)%00%(objectname)%00%(objectname:short)%00%(authorname)%00%(committerdate:iso-strict)%00%(subject)%00{direction}"
        );
        let output = self.runner.required(
            root,
            ["for-each-ref", format.as_str(), "refs/heads"],
            LARGE_OUTPUT_LIMIT,
        )?;
        parse_branch_summaries(&output)
    }

    pub fn current_head_ref(
        &self,
        root: &Path,
    ) -> Result<Option<FullRefName>, RepositoryContextError> {
        self.runner
            .optional(
                root,
                ["symbolic-ref", "--quiet", "HEAD"],
                SMALL_OUTPUT_LIMIT,
            )?
            .map(text)
            .transpose()?
            .map(FullRefName::parse)
            .transpose()
    }

    pub fn symbolic_target(
        &self,
        root: &Path,
        reference: &FullRefName,
    ) -> Result<Option<FullRefName>, RepositoryContextError> {
        self.runner
            .optional(
                root,
                ["symbolic-ref", "--quiet", reference.as_str()],
                SMALL_OUTPUT_LIMIT,
            )?
            .map(text)
            .transpose()?
            .map(FullRefName::parse)
            .transpose()
    }

    pub fn remote_default_branch(
        &self,
        root: &Path,
    ) -> Result<Option<FullRefName>, RepositoryContextError> {
        let origin_head = FullRefName::parse("refs/remotes/origin/HEAD")?;
        self.symbolic_target(root, &origin_head)
    }

    pub fn resolve_commit(
        &self,
        root: &Path,
        reference: &FullRefName,
    ) -> Result<ObjectId, RepositoryContextError> {
        let revision = format!("{}^{{commit}}", reference.as_str());
        ObjectId::parse(text(self.runner.required(
            root,
            ["rev-parse", "--verify", revision.as_str()],
            SMALL_OUTPUT_LIMIT,
        )?)?)
    }
}

fn parse_branches(output: &[u8]) -> Result<Vec<BranchRef>, RepositoryContextError> {
    std::str::from_utf8(output)
        .map_err(|_| invalid_output())?
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| {
            let (name, object) = line.split_once('\0').ok_or_else(invalid_output)?;
            Ok(BranchRef {
                full_name: FullRefName::parse(name)?,
                object_id: ObjectId::parse(object)?,
            })
        })
        .collect()
}

fn parse_branch_summaries(output: &[u8]) -> Result<Vec<BranchSummary>, RepositoryContextError> {
    std::str::from_utf8(output)
        .map_err(|_| invalid_output())?
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| {
            let fields = line.split('\0').collect::<Vec<_>>();
            if fields.len() != 7 {
                return Err(invalid_output());
            }
            let mut direction = fields[6].split_whitespace();
            let ahead = direction
                .next()
                .and_then(|value| value.parse().ok())
                .ok_or_else(invalid_output)?;
            let behind = direction
                .next()
                .and_then(|value| value.parse().ok())
                .ok_or_else(invalid_output)?;
            if direction.next().is_some() {
                return Err(invalid_output());
            }
            let object_id = ObjectId::parse(fields[1])?;
            Ok(BranchSummary {
                branch: BranchRef {
                    full_name: FullRefName::parse(fields[0])?,
                    object_id: object_id.clone(),
                },
                tip: CommitFacts {
                    object_id,
                    abbreviated_id: nonempty(fields[2].as_bytes())?,
                    author: nonempty(fields[3].as_bytes())?,
                    committed_at: nonempty(fields[4].as_bytes())?,
                    subject: fields[5].to_owned(),
                },
                ahead,
                behind,
            })
        })
        .collect()
}

fn text(bytes: Vec<u8>) -> Result<String, RepositoryContextError> {
    let text = std::str::from_utf8(&bytes)
        .map_err(|_| invalid_output())?
        .trim();
    if text.is_empty() || text.contains('\0') {
        Err(invalid_output())
    } else {
        Ok(text.to_owned())
    }
}

fn utf8(bytes: &[u8]) -> Result<&str, RepositoryContextError> {
    std::str::from_utf8(bytes).map_err(|_| invalid_output())
}

fn nonempty(bytes: &[u8]) -> Result<String, RepositoryContextError> {
    let value = utf8(bytes)?;
    (!value.is_empty())
        .then(|| value.to_owned())
        .ok_or_else(invalid_output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_option_like_or_ambiguous_refs() {
        assert!(FullRefName::parse("-c").is_err());
        assert!(FullRefName::parse("refs/heads/feature").is_ok());
        assert!(FullRefName::parse("refs/heads/bad..name").is_err());
        assert!(FullRefName::parse("refs/heads/bad name").is_err());
    }

    #[test]
    fn parses_batched_branch_direction_and_tip_facts() {
        let object = "a".repeat(40);
        let output = format!(
            "refs/heads/feature\0{object}\0aaaaaaa\0Ada\02026-08-26T10:00:00+02:00\0Batch branch reads\03 2\n"
        );
        let summaries = parse_branch_summaries(output.as_bytes()).unwrap();

        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].branch.full_name.as_str(), "refs/heads/feature");
        assert_eq!(summaries[0].tip.object_id.as_str(), object);
        assert_eq!(summaries[0].tip.subject, "Batch branch reads");
        assert_eq!((summaries[0].ahead, summaries[0].behind), (3, 2));
    }
}
