use super::{
    command::{HardenedGitRunner, LARGE_OUTPUT_LIMIT, SMALL_OUTPUT_LIMIT},
    invalid_output, RepositoryContextError,
};
use std::{path::Path, sync::Arc};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) struct ObjectId(String);

impl ObjectId {
    pub(crate) fn parse(value: impl Into<String>) -> Result<Self, RepositoryContextError> {
        let value = value.into().to_ascii_lowercase();
        if matches!(value.len(), 40 | 64) && value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            Ok(Self(value))
        } else {
            Err(invalid_output())
        }
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) struct FullRefName(String);

impl FullRefName {
    pub(crate) fn parse(value: impl Into<String>) -> Result<Self, RepositoryContextError> {
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

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }

    pub(crate) fn branch_name(&self) -> Option<&str> {
        self.0.strip_prefix("refs/heads/")
    }

    pub(crate) fn display_name(&self) -> &str {
        self.0
            .strip_prefix("refs/heads/")
            .or_else(|| self.0.strip_prefix("refs/remotes/"))
            .or_else(|| self.0.strip_prefix("refs/tags/"))
            .unwrap_or(&self.0)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct BranchRef {
    pub(crate) full_name: FullRefName,
    pub(crate) object_id: ObjectId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct BranchSummary {
    pub(crate) branch: BranchRef,
    pub(crate) tip: CommitFacts,
    pub(crate) ahead: usize,
    pub(crate) behind: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct CommitFacts {
    pub(crate) object_id: ObjectId,
    pub(crate) abbreviated_id: String,
    pub(crate) subject: String,
    pub(crate) author: String,
    pub(crate) committed_at: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct CommitDivergence {
    pub(crate) behind: usize,
    pub(crate) ahead: usize,
    pub(crate) merge_base: Option<ObjectId>,
}

#[derive(Clone)]
pub(crate) struct ReferenceReader {
    runner: Arc<HardenedGitRunner>,
}

impl ReferenceReader {
    pub(super) fn new(runner: Arc<HardenedGitRunner>) -> Self {
        Self { runner }
    }

    pub(crate) fn local_branches(
        &self,
        root: &Path,
    ) -> Result<Vec<BranchRef>, RepositoryContextError> {
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
    pub(crate) fn local_branch_summaries(
        &self,
        root: &Path,
        default_branch: Option<&FullRefName>,
    ) -> Result<Vec<BranchSummary>, RepositoryContextError> {
        let direction = default_branch
            .map(|reference| format!("%(ahead-behind:{})", reference.as_str()))
            .unwrap_or_else(|| "0 0".into());
        let format = format!(
            "--format=%(refname)%00%(objectname)%00%(objectname:short)%00%(authorname)%00%(authordate:iso-strict)%00%(subject)%00{direction}"
        );
        let output = self.runner.required(
            root,
            ["for-each-ref", format.as_str(), "refs/heads"],
            LARGE_OUTPUT_LIMIT,
        )?;
        parse_branch_summaries(&output)
    }

    pub(crate) fn current_head_ref(
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

    pub(crate) fn symbolic_target(
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

    pub(crate) fn remote_default_branch(
        &self,
        root: &Path,
    ) -> Result<Option<FullRefName>, RepositoryContextError> {
        let origin_head = FullRefName::parse("refs/remotes/origin/HEAD")?;
        self.symbolic_target(root, &origin_head)
    }

    pub(crate) fn resolve_commit(
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

#[derive(Clone)]
pub(crate) struct CommitReader {
    runner: Arc<HardenedGitRunner>,
}

impl CommitReader {
    pub(super) fn new(runner: Arc<HardenedGitRunner>) -> Self {
        Self { runner }
    }

    pub(crate) fn facts(
        &self,
        root: &Path,
        object: &ObjectId,
    ) -> Result<CommitFacts, RepositoryContextError> {
        let output = self.runner.required(
            root,
            [
                "show",
                "-s",
                "--format=%H%x00%h%x00%an%x00%aI%x00%s",
                object.as_str(),
            ],
            SMALL_OUTPUT_LIMIT,
        )?;
        let fields = output.split(|byte| *byte == 0).collect::<Vec<_>>();
        if fields.len() != 5 {
            return Err(invalid_output());
        }
        Ok(CommitFacts {
            object_id: ObjectId::parse(utf8(fields[0])?)?,
            abbreviated_id: nonempty(fields[1])?,
            author: nonempty(fields[2])?,
            committed_at: nonempty(fields[3])?,
            subject: utf8(fields[4])?.trim_end().to_owned(),
        })
    }

    /// Returns one bounded page of first-parent commit presentation in one Git process.
    pub(crate) fn first_parent_history_page(
        &self,
        root: &Path,
        branch: &FullRefName,
        after: Option<&ObjectId>,
        page_size: usize,
    ) -> Result<(Vec<CommitFacts>, Option<ObjectId>), RepositoryContextError> {
        if !(1..=200).contains(&page_size) {
            return Err(invalid_output());
        }
        let requested = page_size + usize::from(after.is_some()) + 1;
        let limit = format!("--max-count={requested}");
        let revision = after
            .map(ObjectId::as_str)
            .unwrap_or_else(|| branch.as_str());
        let output = self.runner.required(
            root,
            [
                "log",
                "--first-parent",
                limit.as_str(),
                "--format=%H%x00%h%x00%an%x00%aI%x00%s%x00",
                revision,
            ],
            LARGE_OUTPUT_LIMIT,
        )?;
        let mut facts = parse_commit_facts(&output)?;
        if let Some(after) = after {
            if facts.first().map(|facts| &facts.object_id) != Some(after) {
                return Err(invalid_output());
            }
            facts.remove(0);
        }
        let next_cursor = (facts.len() > page_size).then(|| facts[page_size - 1].object_id.clone());
        facts.truncate(page_size);
        Ok((facts, next_cursor))
    }

    pub(crate) fn divergence(
        &self,
        root: &Path,
        baseline: &ObjectId,
        selected: &ObjectId,
    ) -> Result<CommitDivergence, RepositoryContextError> {
        let range = format!("{}...{}", baseline.as_str(), selected.as_str());
        let counts = text(self.runner.required(
            root,
            ["rev-list", "--left-right", "--count", range.as_str()],
            SMALL_OUTPUT_LIMIT,
        )?)?;
        let mut counts = counts.split_whitespace();
        let behind = counts
            .next()
            .and_then(|value| value.parse().ok())
            .ok_or_else(invalid_output)?;
        let ahead = counts
            .next()
            .and_then(|value| value.parse().ok())
            .ok_or_else(invalid_output)?;
        if counts.next().is_some() {
            return Err(invalid_output());
        }
        let merge_base = self
            .runner
            .optional(
                root,
                ["merge-base", baseline.as_str(), selected.as_str()],
                SMALL_OUTPUT_LIMIT,
            )?
            .map(text)
            .transpose()?
            .map(ObjectId::parse)
            .transpose()?;
        Ok(CommitDivergence {
            behind,
            ahead,
            merge_base,
        })
    }

    pub(crate) fn is_ancestor(
        &self,
        root: &Path,
        ancestor: &ObjectId,
        descendant: &ObjectId,
    ) -> Result<bool, RepositoryContextError> {
        Ok(self
            .runner
            .optional(
                root,
                [
                    "merge-base",
                    "--is-ancestor",
                    ancestor.as_str(),
                    descendant.as_str(),
                ],
                SMALL_OUTPUT_LIMIT,
            )?
            .is_some())
    }

    pub(crate) fn commit_count(
        &self,
        root: &Path,
        ancestor: &ObjectId,
        descendant: &ObjectId,
    ) -> Result<usize, RepositoryContextError> {
        let range = format!("{}..{}", ancestor.as_str(), descendant.as_str());
        text(self.runner.required(
            root,
            ["rev-list", "--count", range.as_str()],
            SMALL_OUTPUT_LIMIT,
        )?)?
        .parse()
        .map_err(|_| invalid_output())
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

fn parse_commit_facts(output: &[u8]) -> Result<Vec<CommitFacts>, RepositoryContextError> {
    let fields = output.split(|byte| *byte == 0).collect::<Vec<_>>();
    let mut commits = Vec::new();
    let mut index = 0;
    while index + 4 < fields.len() {
        let object = utf8(fields[index])?.trim_start_matches(['\r', '\n']);
        if object.is_empty() {
            index += 1;
            continue;
        }
        commits.push(CommitFacts {
            object_id: ObjectId::parse(object)?,
            abbreviated_id: nonempty(fields[index + 1])?,
            author: nonempty(fields[index + 2])?,
            committed_at: nonempty(fields[index + 3])?,
            subject: utf8(fields[index + 4])?
                .trim_end_matches(['\r', '\n'])
                .to_owned(),
        });
        index += 5;
    }
    if fields[index..]
        .iter()
        .any(|field| !utf8(field).unwrap_or_default().trim().is_empty())
    {
        return Err(invalid_output());
    }
    Ok(commits)
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

    #[test]
    fn parses_a_commit_page_without_per_commit_processes() {
        let first = "a".repeat(40);
        let second = "b".repeat(40);
        let output = format!(
            "{first}\0aaaaaaa\0Ada\02026-08-26T10:00:00+02:00\0First\0\n{second}\0bbbbbbb\0Grace\02026-08-25T10:00:00+02:00\0Second\0\n"
        );
        let commits = parse_commit_facts(output.as_bytes()).unwrap();

        assert_eq!(commits.len(), 2);
        assert_eq!(commits[0].object_id.as_str(), first);
        assert_eq!(commits[1].subject, "Second");
    }
}
