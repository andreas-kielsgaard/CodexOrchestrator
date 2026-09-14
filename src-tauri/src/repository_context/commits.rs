use super::{
    command::{HardenedGitRunner, LARGE_OUTPUT_LIMIT, SMALL_OUTPUT_LIMIT},
    invalid_output, ObjectId, RepositoryContextError,
};
use std::{path::Path, sync::Arc};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct CommitParents {
    pub(crate) object_id: ObjectId,
    pub(crate) parents: Vec<ObjectId>,
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
                "--format=%H%x00%h%x00%an%x00%cI%x00%s",
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

    /// Pinned endpoints and an offset keep merged paths in the same topological traversal.
    pub(crate) fn history_page(
        &self,
        root: &Path,
        tip: &ObjectId,
        excluded: Option<&ObjectId>,
        offset: usize,
        page_size: usize,
    ) -> Result<(Vec<CommitFacts>, usize), RepositoryContextError> {
        if !(1..=200).contains(&page_size) || offset > 1_000_000 {
            return Err(invalid_output());
        }
        let revision = excluded
            .map(|base| format!("{}..{}", base.as_str(), tip.as_str()))
            .unwrap_or_else(|| tip.as_str().to_owned());
        let total = text(self.runner.required(
            root,
            ["rev-list", "--count", &revision],
            SMALL_OUTPUT_LIMIT,
        )?)?
        .parse()
        .map_err(|_| invalid_output())?;
        let output = self.runner.required(
            root,
            [
                "log",
                "--topo-order",
                &format!("--skip={offset}"),
                &format!("--max-count={page_size}"),
                "--format=%H%x00%h%x00%an%x00%cI%x00%s%x00",
                &revision,
                "--",
            ],
            LARGE_OUTPUT_LIMIT,
        )?;
        Ok((parse_commit_facts(&output)?, total))
    }

    pub(crate) fn facts_for_objects(
        &self,
        root: &Path,
        objects: &[ObjectId],
    ) -> Result<Vec<CommitFacts>, RepositoryContextError> {
        if objects.is_empty() {
            return Ok(Vec::new());
        }
        if objects.len() > 200 {
            return Err(invalid_output());
        }
        let mut args = vec![
            "log".to_owned(),
            "--no-walk=unsorted".into(),
            "--format=%H%x00%h%x00%an%x00%cI%x00%s%x00".into(),
        ];
        args.extend(objects.iter().map(|object| object.as_str().to_owned()));
        args.push("--".into());
        parse_commit_facts(&self.runner.required(root, args, LARGE_OUTPUT_LIMIT)?)
    }

    pub(crate) fn graph_page(
        &self,
        root: &Path,
        tips: &[ObjectId],
        limit: usize,
    ) -> Result<(Vec<CommitParents>, bool), RepositoryContextError> {
        if tips.is_empty() {
            return Ok((Vec::new(), false));
        }
        if tips.len() > 512 || !(1..=20_000).contains(&limit) {
            return Err(invalid_output());
        }
        let mut args = vec![
            "rev-list".to_owned(),
            "--topo-order".into(),
            "--parents".into(),
            format!("--max-count={}", limit + 1),
        ];
        args.extend(tips.iter().map(|id| id.as_str().to_owned()));
        args.push("--".into());
        let output = self.runner.required(root, args, LARGE_OUTPUT_LIMIT)?;
        let mut commits = utf8(&output)?
            .lines()
            .map(|line| {
                let mut parts = line.split_whitespace();
                let object_id = ObjectId::parse(parts.next().ok_or_else(invalid_output)?)?;
                let parents = parts.map(ObjectId::parse).collect::<Result<Vec<_>, _>>()?;
                Ok(CommitParents { object_id, parents })
            })
            .collect::<Result<Vec<_>, RepositoryContextError>>()?;
        let has_more = commits.len() > limit;
        commits.truncate(limit);
        Ok((commits, has_more))
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
