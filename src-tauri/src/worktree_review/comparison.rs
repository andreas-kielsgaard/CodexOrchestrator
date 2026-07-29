use super::worktree_build::{git_bytes, git_status, git_text, WorktreeScope};
use serde::Serialize;
use std::{
    collections::BTreeSet,
    path::{Component, Path},
};

const MAX_FILE_BYTES: u64 = 1_500_000;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorktreeComparisonView {
    pub(crate) files: Vec<ComparisonFileView>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ComparisonFileView {
    pub(crate) file_id: String,
    pub(crate) display_path: String,
    pub(crate) change_kind: String,
    pub(crate) additions: usize,
    pub(crate) deletions: usize,
    pub(crate) provenance: Vec<String>,
    pub(crate) content: ComparisonContentView,
    pub(crate) hunks: Vec<ComparisonHunkView>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub(crate) enum ComparisonContentView {
    Text {
        text: String,
        language: Option<String>,
    },
    Markdown {
        text: String,
        language: Option<String>,
    },
    Binary {
        reason: String,
    },
    Unsupported {
        reason: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ComparisonHunkView {
    pub(crate) hunk_id: String,
    pub(crate) header: String,
    pub(crate) lines: Vec<ComparisonLineView>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ComparisonLineView {
    pub(crate) kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) old_line_number: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) new_line_number: Option<usize>,
    pub(crate) text: String,
}

#[tauri::command]
pub(crate) fn worktree_build_comparison() -> Result<WorktreeComparisonView, String> {
    comparison(&WorktreeScope::from_environment()?)
}

pub(super) fn comparison(scope: &WorktreeScope) -> Result<WorktreeComparisonView, String> {
    let main_head = git_text(&scope.main, ["rev-parse", "HEAD"])?;
    let selected_head = git_text(&scope.selected, ["rev-parse", "HEAD"])?;
    let committed = changed_paths(&scope.selected, &main_head, &selected_head)?;
    let uncommitted = uncommitted_paths(&scope.selected)?;
    let mut paths = committed.union(&uncommitted).cloned().collect::<Vec<_>>();
    paths.sort();
    let files = paths
        .into_iter()
        .enumerate()
        .map(|(index, path)| {
            comparison_file(
                scope,
                &main_head,
                &path,
                index,
                committed.contains(&path),
                uncommitted.contains(&path),
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(WorktreeComparisonView { files })
}

fn comparison_file(
    scope: &WorktreeScope,
    main_head: &str,
    path: &str,
    index: usize,
    committed: bool,
    uncommitted: bool,
) -> Result<ComparisonFileView, String> {
    validate_relative(path)?;
    let absolute = scope.selected.join(path);
    let exists = absolute.is_file();
    let main_spec = format!("{main_head}:{path}");
    let main_exists = git_status(&scope.selected, ["cat-file", "-e", &main_spec]);
    let change_kind = match (main_exists, exists) {
        (false, true) => "added",
        (true, false) => "deleted",
        _ => "modified",
    };
    let provenance = [
        committed.then_some("committed-divergence".to_string()),
        uncommitted.then_some("uncommitted".to_string()),
    ]
    .into_iter()
    .flatten()
    .collect();
    if !exists {
        let bytes = git_bytes(&scope.selected, ["show", &main_spec])?;
        if bytes.len() as u64 > MAX_FILE_BYTES {
            return Ok(unsupported(
                index,
                path,
                change_kind,
                provenance,
                "The deleted machine-main file exceeds the bounded review size.",
            ));
        }
        if bytes.contains(&0) {
            return Ok(binary(
                index,
                path,
                change_kind,
                provenance,
                "The deleted machine-main file is binary and is not rendered.",
            ));
        }
        let Ok(text) = String::from_utf8(bytes) else {
            return Ok(binary(
                index,
                path,
                change_kind,
                provenance,
                "The deleted machine-main file is binary and is not rendered.",
            ));
        };
        return Ok(ComparisonFileView {
            file_id: format!("file-{index}"),
            display_path: path.into(),
            change_kind: change_kind.into(),
            additions: 0,
            deletions: content_lines(&text).len(),
            provenance,
            content: ComparisonContentView::Unsupported {
                reason: "Deleted in the selected worktree.".into(),
            },
            hunks: Vec::new(),
        });
    }
    let metadata = std::fs::symlink_metadata(&absolute)
        .map_err(|_| "A changed file is unavailable.".to_string())?;
    if !metadata.file_type().is_file() {
        return Ok(unsupported(
            index,
            path,
            change_kind,
            provenance,
            "This changed entry is not a regular file.",
        ));
    }
    if metadata.len() > MAX_FILE_BYTES {
        return Ok(unsupported(
            index,
            path,
            change_kind,
            provenance,
            "This file exceeds the bounded review size.",
        ));
    }
    let bytes = std::fs::read(&absolute)
        .map_err(|_| "A changed file could not be read safely.".to_string())?;
    if bytes.contains(&0) {
        return Ok(binary(
            index,
            path,
            change_kind,
            provenance,
            "Binary file content is not rendered.",
        ));
    }
    let Ok(text) = String::from_utf8(bytes) else {
        return Ok(binary(
            index,
            path,
            change_kind,
            provenance,
            "Binary file content is not rendered.",
        ));
    };
    let old = if main_exists {
        let bytes = git_bytes(&scope.selected, ["show", &main_spec])?;
        if bytes.len() as u64 > MAX_FILE_BYTES {
            return Ok(unsupported(
                index,
                path,
                change_kind,
                provenance,
                "The machine-main version exceeds the bounded review size.",
            ));
        }
        if bytes.contains(&0) {
            return Ok(unsupported(
                index,
                path,
                change_kind,
                provenance,
                "The machine-main version is binary; a truthful textual diff is unavailable.",
            ));
        }
        let Ok(old) = String::from_utf8(bytes) else {
            return Ok(unsupported(
                index,
                path,
                change_kind,
                provenance,
                "The machine-main version is binary; a truthful textual diff is unavailable.",
            ));
        };
        old
    } else {
        String::new()
    };
    let (hunk, additions, deletions) = complete_hunk(&old, &text, index);
    let language = language(path);
    let content = if path.to_ascii_lowercase().ends_with(".md") {
        ComparisonContentView::Markdown { text, language }
    } else {
        ComparisonContentView::Text { text, language }
    };
    Ok(ComparisonFileView {
        file_id: format!("file-{index}"),
        display_path: path.into(),
        change_kind: change_kind.into(),
        additions,
        deletions,
        provenance,
        content,
        hunks: vec![hunk],
    })
}

fn binary(
    index: usize,
    path: &str,
    change_kind: &str,
    provenance: Vec<String>,
    reason: &str,
) -> ComparisonFileView {
    ComparisonFileView {
        file_id: format!("file-{index}"),
        display_path: path.into(),
        change_kind: change_kind.into(),
        additions: 0,
        deletions: 0,
        provenance,
        content: ComparisonContentView::Binary {
            reason: reason.into(),
        },
        hunks: Vec::new(),
    }
}

fn unsupported(
    index: usize,
    path: &str,
    change_kind: &str,
    provenance: Vec<String>,
    reason: &str,
) -> ComparisonFileView {
    ComparisonFileView {
        file_id: format!("file-{index}"),
        display_path: path.into(),
        change_kind: change_kind.into(),
        additions: 0,
        deletions: 0,
        provenance,
        content: ComparisonContentView::Unsupported {
            reason: reason.into(),
        },
        hunks: Vec::new(),
    }
}

fn changed_paths(path: &Path, left: &str, right: &str) -> Result<BTreeSet<String>, String> {
    let range = format!("{left}..{right}");
    nul_paths(git_bytes(
        path,
        ["diff", "--name-only", "-z", "--no-renames", &range, "--"],
    )?)
}

fn uncommitted_paths(path: &Path) -> Result<BTreeSet<String>, String> {
    let tracked = nul_paths(git_bytes(
        path,
        ["diff", "--name-only", "-z", "--no-renames", "HEAD", "--"],
    )?)?;
    let untracked = nul_paths(git_bytes(
        path,
        ["ls-files", "--others", "--exclude-standard", "-z"],
    )?)?;
    Ok(tracked.union(&untracked).cloned().collect())
}

fn nul_paths(bytes: Vec<u8>) -> Result<BTreeSet<String>, String> {
    bytes
        .split(|byte| *byte == 0)
        .filter(|entry| !entry.is_empty())
        .map(|entry| {
            String::from_utf8(entry.to_vec())
                .map_err(|_| "A changed path is not valid UTF-8.".to_string())
        })
        .collect()
}

fn complete_hunk(old: &str, new: &str, index: usize) -> (ComparisonHunkView, usize, usize) {
    let old_lines = content_lines(old);
    let new_lines = content_lines(new);
    let mut prefix = 0;
    while prefix < old_lines.len()
        && prefix < new_lines.len()
        && old_lines[prefix] == new_lines[prefix]
    {
        prefix += 1;
    }
    let mut suffix = 0;
    while suffix < old_lines.len().saturating_sub(prefix)
        && suffix < new_lines.len().saturating_sub(prefix)
        && old_lines[old_lines.len() - suffix - 1] == new_lines[new_lines.len() - suffix - 1]
    {
        suffix += 1;
    }
    let mut lines = Vec::new();
    for (offset, value) in old_lines[..prefix].iter().enumerate() {
        lines.push(line("context", Some(offset + 1), Some(offset + 1), value));
    }
    for (offset, value) in old_lines[prefix..old_lines.len() - suffix]
        .iter()
        .enumerate()
    {
        lines.push(line("deletion", Some(prefix + offset + 1), None, value));
    }
    for (offset, value) in new_lines[prefix..new_lines.len() - suffix]
        .iter()
        .enumerate()
    {
        lines.push(line("addition", None, Some(prefix + offset + 1), value));
    }
    for offset in 0..suffix {
        let old_number = old_lines.len() - suffix + offset + 1;
        let new_number = new_lines.len() - suffix + offset + 1;
        lines.push(line(
            "context",
            Some(old_number),
            Some(new_number),
            old_lines[old_number - 1],
        ));
    }
    let additions = lines.iter().filter(|line| line.kind == "addition").count();
    let deletions = lines.iter().filter(|line| line.kind == "deletion").count();
    (
        ComparisonHunkView {
            hunk_id: format!("hunk-{index}-0"),
            header: format!(
                "@@ -{},{} +{},{} @@",
                usize::from(!old_lines.is_empty()),
                old_lines.len(),
                usize::from(!new_lines.is_empty()),
                new_lines.len()
            ),
            lines,
        },
        additions,
        deletions,
    )
}

fn line(
    kind: &str,
    old_line_number: Option<usize>,
    new_line_number: Option<usize>,
    text: &str,
) -> ComparisonLineView {
    ComparisonLineView {
        kind: kind.into(),
        old_line_number,
        new_line_number,
        text: text.into(),
    }
}

fn content_lines(text: &str) -> Vec<&str> {
    if text.is_empty() {
        Vec::new()
    } else {
        text.strip_suffix('\n')
            .unwrap_or(text)
            .split('\n')
            .collect()
    }
}

fn language(path: &str) -> Option<String> {
    Path::new(path)
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
}

fn validate_relative(value: &str) -> Result<(), String> {
    let path = Path::new(value);
    if value.is_empty()
        || path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err("Git returned an unsafe changed-file identity.".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    #[test]
    fn complete_hunk_keeps_line_numbers_content_and_counts_coherent() {
        let (hunk, additions, deletions) = complete_hunk("one\nold\n", "one\nnew\nmore\n", 0);
        assert_eq!(hunk.header, "@@ -1,2 +1,3 @@");
        assert_eq!((additions, deletions), (2, 1));
        assert_eq!(
            hunk.lines
                .iter()
                .filter_map(|line| line.new_line_number.map(|_| line.text.as_str()))
                .collect::<Vec<_>>(),
            ["one", "new", "more"]
        );
        let value = serde_json::to_value(&hunk).expect("serialized hunk");
        assert!(value["lines"]
            .as_array()
            .expect("lines")
            .iter()
            .all(|line| line
                .as_object()
                .expect("line")
                .values()
                .all(|value| !value.is_null())));
    }

    #[test]
    fn comparison_combines_committed_and_uncommitted_state_with_explicit_provenance() {
        let directory = tempfile::tempdir().expect("directory");
        let main = directory.path().join("main");
        let selected = directory.path().join("selected");
        std::fs::create_dir(&main).expect("main");
        run(&main, ["init"]);
        run(&main, ["config", "user.email", "fixture@example.test"]);
        run(&main, ["config", "user.name", "Fixture"]);
        std::fs::write(main.join("both.txt"), "base\n").expect("base");
        std::fs::write(main.join("deleted.bin"), [0, 159, 146, 150]).expect("binary");
        run(&main, ["add", "."]);
        run(&main, ["commit", "-m", "base"]);
        run(
            &main,
            [
                "worktree",
                "add",
                "-b",
                "codex/comparison-fixture",
                selected.to_str().expect("selected"),
            ],
        );
        std::fs::write(selected.join("both.txt"), "committed\n").expect("committed");
        std::fs::remove_file(selected.join("deleted.bin")).expect("delete binary");
        run(&selected, ["add", "."]);
        run(&selected, ["commit", "-m", "committed divergence"]);
        std::fs::write(selected.join("both.txt"), "committed\nuncommitted\n").expect("working");
        std::fs::write(selected.join("new.txt"), "new\n").expect("new");

        let scope = WorktreeScope {
            name: "Fixture".into(),
            selected: selected.canonicalize().expect("selected canonical"),
            main: main.canonicalize().expect("main canonical"),
        };
        let snapshot = comparison(&scope).expect("comparison");
        assert_eq!(snapshot.files.len(), 3);
        let both = snapshot
            .files
            .iter()
            .find(|file| file.display_path == "both.txt")
            .expect("both");
        assert_eq!(
            both.provenance,
            [
                "committed-divergence".to_string(),
                "uncommitted".to_string()
            ]
        );
        assert_eq!((both.additions, both.deletions), (2, 1));
        match &both.content {
            ComparisonContentView::Text { text, .. } => {
                assert_eq!(text, "committed\nuncommitted\n")
            }
            _ => panic!("text"),
        }
        let untracked = snapshot
            .files
            .iter()
            .find(|file| file.display_path == "new.txt")
            .expect("untracked");
        assert_eq!(untracked.provenance, ["uncommitted".to_string()]);
        assert_eq!(untracked.change_kind, "added");
        let deleted = snapshot
            .files
            .iter()
            .find(|file| file.display_path == "deleted.bin")
            .expect("deleted binary");
        assert_eq!(deleted.change_kind, "deleted");
        assert_eq!((deleted.additions, deleted.deletions), (0, 0));
        assert!(deleted.hunks.is_empty());
        assert!(matches!(
            deleted.content,
            ComparisonContentView::Binary { .. }
        ));
    }

    fn run<const N: usize>(path: &Path, args: [&str; N]) {
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
