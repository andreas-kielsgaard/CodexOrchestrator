use super::worktree_build::git_bytes;
use super::{catalog::ReviewWorktreeCatalog, worktree_build::git_text};
use serde::Serialize;
use std::collections::HashSet;

const MAX_VISIBLE_COMMITS: usize = 250;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReviewSourceHistoryView {
    pub(crate) branch: String,
    pub(crate) source_label: String,
    pub(crate) revision: String,
    pub(crate) fork_revision: String,
    pub(crate) commit_count: usize,
    pub(crate) truncated: bool,
    pub(crate) commits: Vec<ReviewCommitView>,
    pub(crate) lineage_markers: Vec<ReviewLineageMarkerView>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReviewCommitView {
    pub(crate) id: String,
    pub(crate) abbreviated_id: String,
    pub(crate) subject: String,
    pub(crate) description: String,
    pub(crate) author: String,
    pub(crate) committed_at: String,
    pub(crate) files_changed: usize,
    pub(crate) insertions: usize,
    pub(crate) deletions: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReviewLineageMarkerView {
    pub(crate) branch: String,
    pub(crate) commit_id: String,
    pub(crate) abbreviated_id: String,
}

pub(crate) fn read(
    catalog: &ReviewWorktreeCatalog,
    source_ref: &str,
) -> Result<ReviewSourceHistoryView, String> {
    let identity = catalog.source_history_identity(source_ref)?;
    let range = format!(
        "{}..{}",
        identity.baseline_object_id, identity.selected_object_id
    );
    let commit_ids = git_text(
        &identity.selected_root,
        [
            "rev-list",
            "--topo-order",
            &format!("--max-count={MAX_VISIBLE_COMMITS}"),
            &range,
        ],
    )?;
    let commit_count = git_text(&identity.selected_root, ["rev-list", "--count", &range])?
        .parse::<usize>()
        .map_err(|_| "Git returned an invalid branch commit count.".to_string())?;
    let commits = commit_ids
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|commit_id| commit(&identity.selected_root, commit_id))
        .collect::<Result<Vec<_>, _>>()?;
    let fork_revision = git_text(
        &identity.selected_root,
        [
            "merge-base",
            &identity.baseline_object_id,
            &identity.selected_object_id,
        ],
    )?;
    let first_parent_commits = git_text(
        &identity.selected_root,
        ["rev-list", "--first-parent", &range],
    )?
    .lines()
    .map(str::to_owned)
    .collect::<HashSet<_>>();
    let lineage_markers = identity
        .related_branch_tips
        .into_iter()
        .filter(|(_, commit_id)| first_parent_commits.contains(commit_id))
        .filter_map(|(branch, commit_id)| {
            commits
                .iter()
                .find(|commit| commit.id == commit_id)
                .map(|commit| ReviewLineageMarkerView {
                    branch,
                    commit_id: commit.id.clone(),
                    abbreviated_id: commit.abbreviated_id.clone(),
                })
        })
        .collect::<Vec<_>>();

    Ok(ReviewSourceHistoryView {
        branch: identity.branch,
        source_label: identity.source_label,
        revision: abbreviated(&identity.selected_object_id),
        fork_revision: abbreviated(&fork_revision),
        commit_count,
        truncated: commit_count > commits.len(),
        commits,
        lineage_markers,
    })
}

fn commit(path: &std::path::Path, commit_id: &str) -> Result<ReviewCommitView, String> {
    let facts = git_text(
        path,
        [
            "show",
            "-s",
            "--format=%H%x1f%h%x1f%an%x1f%aI%x1f%s%x1f%b",
            commit_id,
        ],
    )?;
    let mut fields = facts.splitn(6, '\u{1f}');
    let id = fields
        .next()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "Git returned an invalid commit identity.".to_string())?;
    let abbreviated_id = fields
        .next()
        .ok_or_else(|| "Git returned an invalid abbreviated commit identity.".to_string())?;
    let author = fields
        .next()
        .ok_or_else(|| "Git returned an invalid commit author.".to_string())?;
    let committed_at = fields
        .next()
        .ok_or_else(|| "Git returned an invalid commit timestamp.".to_string())?;
    let subject = fields
        .next()
        .ok_or_else(|| "Git returned an invalid commit subject.".to_string())?;
    let description = fields.next().unwrap_or_default().trim().to_owned();
    let first_parent = git_text(path, ["rev-parse", "--verify", &format!("{commit_id}^1")]).ok();
    let stats = if let Some(parent) = first_parent {
        git_bytes(
            path,
            [
                "diff",
                "--numstat",
                "-z",
                "--no-renames",
                &parent,
                commit_id,
            ],
        )?
    } else {
        git_bytes(
            path,
            [
                "show",
                "--format=",
                "--numstat",
                "-z",
                "--no-renames",
                commit_id,
            ],
        )?
    };
    let (files_changed, insertions, deletions) = parse_stats(&stats)?;
    Ok(ReviewCommitView {
        id: id.into(),
        abbreviated_id: abbreviated_id.into(),
        subject: subject.into(),
        description,
        author: author.into(),
        committed_at: committed_at.into(),
        files_changed,
        insertions,
        deletions,
    })
}

fn parse_stats(stats: &[u8]) -> Result<(usize, usize, usize), String> {
    let mut files = 0;
    let mut insertions = 0;
    let mut deletions = 0;
    for record in stats
        .split(|byte| *byte == 0)
        .filter(|record| !record.is_empty())
    {
        let mut fields = record.splitn(3, |byte| *byte == b'\t');
        let inserted = fields
            .next()
            .ok_or_else(|| "Git returned invalid commit statistics.".to_string())?;
        let deleted = fields
            .next()
            .ok_or_else(|| "Git returned invalid commit statistics.".to_string())?;
        fields
            .next()
            .filter(|value| !value.is_empty())
            .ok_or_else(|| "Git returned invalid commit statistics.".to_string())?;
        files += 1;
        insertions += parse_line_count(inserted)?;
        deletions += parse_line_count(deleted)?;
    }
    Ok((files, insertions, deletions))
}

fn parse_line_count(value: &[u8]) -> Result<usize, String> {
    if value == b"-" {
        return Ok(0);
    }
    std::str::from_utf8(value)
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .ok_or_else(|| "Git returned invalid commit statistics.".to_string())
}

fn abbreviated(value: &str) -> String {
    value.chars().take(12).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, path::Path, process::Command};

    #[test]
    fn numstat_parser_counts_files_and_ignores_binary_line_counts() {
        assert_eq!(
            parse_stats(b"12\t3\tsrc/app.rs\0-\t-\tassets/image.png\0").expect("stats"),
            (2, 12, 3),
        );
        assert!(parse_stats(b"invalid\0").is_err());
    }

    #[test]
    fn named_branch_history_is_newest_first_and_marks_a_registered_parent_tip() {
        let directory = tempfile::tempdir().expect("directory");
        let main = directory.path().join("main");
        let parent = directory.path().join("parent");
        let child = directory.path().join("child");
        run(directory.path(), &["init", main.to_str().expect("main")]);
        run(&main, &["config", "user.email", "fixture@example.test"]);
        run(&main, &["config", "user.name", "Fixture"]);
        fs::write(main.join("base.txt"), "base\n").expect("base");
        run(&main, &["add", "."]);
        run(&main, &["commit", "-m", "Base"]);
        run(
            &main,
            &[
                "worktree",
                "add",
                "-b",
                "codex/parent",
                parent.to_str().expect("parent"),
            ],
        );
        fs::write(parent.join("parent.txt"), "parent\n").expect("parent file");
        run(&parent, &["add", "."]);
        run(
            &parent,
            &[
                "commit",
                "-m",
                "Parent foundation",
                "-m",
                "Parent branch description",
            ],
        );
        run(
            &main,
            &[
                "worktree",
                "add",
                "-b",
                "codex/child",
                child.to_str().expect("child"),
                "codex/parent",
            ],
        );
        fs::write(child.join("child.txt"), "child\n").expect("child file");
        run(&child, &["add", "."]);
        run(
            &child,
            &[
                "commit",
                "-m",
                "Newest feature commit",
                "-m",
                "Newest description",
            ],
        );

        let catalog = ReviewWorktreeCatalog::discover(&main, Path::new("git")).expect("catalog");
        let child_ref = catalog
            .options()
            .iter()
            .find(|option| option.branch.as_deref() == Some("codex/child"))
            .expect("child source")
            .source_ref
            .clone();
        let history = read(&catalog, &child_ref).expect("history");

        assert_eq!(history.commit_count, 2);
        assert_eq!(history.commits[0].subject, "Newest feature commit");
        assert_eq!(history.commits[0].description, "Newest description");
        assert_eq!(history.commits[0].files_changed, 1);
        assert_eq!(history.lineage_markers.len(), 1);
        assert_eq!(history.lineage_markers[0].branch, "codex/parent");
        assert_eq!(history.lineage_markers[0].commit_id, history.commits[1].id);

        fs::write(parent.join("ændring.txt"), "ændring\n").expect("unicode path");
        run(&parent, &["add", "."]);
        run(&parent, &["commit", "-m", "Forælder update"]);
        run(
            &child,
            &[
                "merge",
                "--no-ff",
                "codex/parent",
                "-m",
                "Merge parent update",
            ],
        );

        let refreshed = catalog.live_options().expect("live source facts");
        let refreshed_child = refreshed
            .iter()
            .find(|option| option.branch.as_deref() == Some("codex/child"))
            .expect("refreshed child");
        assert_ne!(refreshed_child.revision, history.revision);
        let merged = read(&catalog, &child_ref).expect("merged history");
        assert_eq!(merged.commit_count, 4);
        assert_eq!(merged.commits[0].subject, "Merge parent update");
        assert_eq!(merged.commits[0].files_changed, 1);
        let unicode = merged
            .commits
            .iter()
            .find(|commit| commit.subject == "Forælder update")
            .expect("unicode commit");
        assert!(unicode.description.is_empty());
        assert!(merged.lineage_markers.is_empty());
    }

    fn run(path: &Path, args: &[&str]) {
        let output = Command::new("git")
            .arg("-C")
            .arg(path)
            .args(args)
            .output()
            .expect("git");
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
