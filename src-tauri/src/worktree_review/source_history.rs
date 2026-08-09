use super::{catalog::ReviewWorktreeCatalog, worktree_build::git_text};
use serde::Serialize;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReviewSourceHistoryView {
    pub(crate) branch: String,
    pub(crate) source_label: String,
    pub(crate) revision: String,
    pub(crate) fork_revision: String,
    pub(crate) commit_count: usize,
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
        ["rev-list", "--topo-order", &range],
    )?;
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
    let lineage_markers = identity
        .related_branch_tips
        .into_iter()
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
        commit_count: commits.len(),
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
    let stats = git_text(
        path,
        ["show", "--format=", "--numstat", "--no-renames", commit_id],
    )?;
    let (files_changed, insertions, deletions) = parse_stats(&stats);
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

fn parse_stats(stats: &str) -> (usize, usize, usize) {
    stats
        .lines()
        .filter_map(|line| {
            let mut fields = line.splitn(3, '\t');
            Some((
                fields.next()?,
                fields.next()?,
                fields.next().filter(|value| !value.is_empty())?,
            ))
        })
        .fold((0, 0, 0), |(files, insertions, deletions), row| {
            (
                files + 1,
                insertions + row.0.parse::<usize>().unwrap_or(0),
                deletions + row.1.parse::<usize>().unwrap_or(0),
            )
        })
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
            parse_stats("12\t3\tsrc/app.rs\n-\t-\tassets/image.png\n"),
            (2, 12, 3)
        );
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
