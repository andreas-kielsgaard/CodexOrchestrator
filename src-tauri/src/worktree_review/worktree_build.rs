use serde::Serialize;
use std::{
    path::{Path, PathBuf},
    process::{Command, Output},
};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorktreeBuildContextView {
    pub(crate) name: String,
    pub(crate) branch: Option<String>,
    pub(crate) detached: bool,
    pub(crate) head: CommitView,
    pub(crate) dirty: DirtyView,
    pub(crate) main: MainCheckoutView,
    pub(crate) relationship: RelationshipView,
    pub(crate) history: Vec<CommitView>,
    pub(crate) comparison_basis: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CommitView {
    pub(crate) id: String,
    pub(crate) abbreviated_id: String,
    pub(crate) message: String,
    pub(crate) committed_at: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DirtyView {
    pub(crate) dirty: bool,
    pub(crate) staged: usize,
    pub(crate) unstaged: usize,
    pub(crate) untracked: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MainCheckoutView {
    pub(crate) branch: Option<String>,
    pub(crate) detached: bool,
    pub(crate) head: CommitView,
    pub(crate) dirty: DirtyView,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RelationshipView {
    pub(crate) ahead: usize,
    pub(crate) behind: usize,
    pub(crate) merge_base: Option<String>,
    pub(crate) summary: String,
}

#[tauri::command]
pub(crate) fn worktree_build_context() -> Result<WorktreeBuildContextView, String> {
    WorktreeScope::from_environment()?.context()
}

pub(super) struct WorktreeScope {
    pub(super) name: String,
    pub(super) selected: PathBuf,
    pub(super) main: PathBuf,
}

impl WorktreeScope {
    pub(super) fn from_environment() -> Result<Self, String> {
        let name = std::env::var("CODEX_ORCHESTRATOR_WORKTREE_BUILD_NAME")
            .map_err(|_| "This is not an isolated worktree build.".to_string())?;
        let selected = std::env::var_os("CODEX_ORCHESTRATOR_WORKTREE_BUILD_PATH")
            .map(PathBuf::from)
            .ok_or_else(|| "Worktree identity is unavailable.".to_string())?
            .canonicalize()
            .map_err(|_| "The selected worktree is unavailable.".to_string())?;
        let worktrees = git_text(&selected, ["worktree", "list", "--porcelain"])?;
        let paths = worktrees
            .split("\n\n")
            .filter_map(|block| {
                block
                    .lines()
                    .find_map(|line| line.strip_prefix("worktree "))
                    .map(PathBuf::from)
            })
            .map(|path| {
                path.canonicalize()
                    .map_err(|_| "A registered Git worktree is unavailable.".to_string())
            })
            .collect::<Result<Vec<_>, _>>()?;
        if !paths.contains(&selected) {
            return Err("The selected worktree is no longer registered with Git.".into());
        }
        let main = paths
            .first()
            .cloned()
            .ok_or_else(|| "The machine main checkout is unavailable.".to_string())?;
        Ok(Self {
            name,
            selected,
            main,
        })
    }

    pub(crate) fn context(&self) -> Result<WorktreeBuildContextView, String> {
        let head = commit(&self.selected, "HEAD")?;
        let main_head = commit(&self.main, "HEAD")?;
        let selected_branch = branch(&self.selected)?;
        let main_branch = branch(&self.main)?;
        let range = format!("{}...{}", main_head.id, head.id);
        let counts = git_text(
            &self.selected,
            ["rev-list", "--left-right", "--count", &range],
        )?;
        let mut counts = counts.split_whitespace();
        let behind = counts
            .next()
            .and_then(|value| value.parse().ok())
            .unwrap_or(0);
        let ahead = counts
            .next()
            .and_then(|value| value.parse().ok())
            .unwrap_or(0);
        let merge_base = git_text(&self.selected, ["merge-base", &main_head.id, &head.id])
            .ok()
            .filter(|value| !value.is_empty());
        let history_range = format!("{}..{}", main_head.id, head.id);
        let history = git_text(
            &self.selected,
            [
                "log",
                "-n",
                "20",
                "--format=%H%x1f%h%x1f%s%x1f%cI",
                &history_range,
            ],
        )?
        .lines()
        .filter_map(parse_commit_line)
        .collect();
        Ok(WorktreeBuildContextView {
            name: self.name.clone(),
            branch: selected_branch.clone(),
            detached: selected_branch.is_none(),
            head,
            dirty: dirty(&self.selected)?,
            main: MainCheckoutView {
                branch: main_branch.clone(),
                detached: main_branch.is_none(),
                head: main_head,
                dirty: dirty(&self.main)?,
            },
            relationship: RelationshipView {
                ahead,
                behind,
                merge_base,
                summary: format!("{ahead} ahead, {behind} behind machine main HEAD"),
            },
            history,
            comparison_basis: "The file review compares machine main HEAD with the selected worktree's complete current state. It includes committed divergence plus selected staged, unstaged, and untracked changes. Machine-main uncommitted changes are reported here but are not used as the comparison base.".into(),
        })
    }
}

pub(super) fn branch(path: &Path) -> Result<Option<String>, String> {
    let output = git_output(path, ["symbolic-ref", "--quiet", "--short", "HEAD"])?;
    if output.status.success() {
        text(output.stdout).map(Some)
    } else {
        Ok(None)
    }
}

pub(super) fn dirty(path: &Path) -> Result<DirtyView, String> {
    let output = git_bytes(
        path,
        ["status", "--porcelain=v1", "-z", "--untracked-files=all"],
    )?;
    let mut staged = 0;
    let mut unstaged = 0;
    let mut untracked = 0;
    for entry in output
        .split(|byte| *byte == 0)
        .filter(|entry| !entry.is_empty())
    {
        if entry.len() < 3 {
            return Err("Git returned invalid worktree status.".into());
        }
        if &entry[..2] == b"??" {
            untracked += 1;
        } else {
            staged += usize::from(entry[0] != b' ');
            unstaged += usize::from(entry[1] != b' ');
        }
    }
    Ok(DirtyView {
        dirty: staged + unstaged + untracked > 0,
        staged,
        unstaged,
        untracked,
    })
}

fn commit(path: &Path, revision: &str) -> Result<CommitView, String> {
    let output = git_text(
        path,
        ["show", "-s", "--format=%H%x1f%h%x1f%s%x1f%cI", revision],
    )?;
    parse_commit_line(&output).ok_or_else(|| "Git returned invalid commit facts.".to_string())
}

fn parse_commit_line(line: &str) -> Option<CommitView> {
    let mut fields = line.trim().split('\u{1f}');
    Some(CommitView {
        id: fields.next()?.into(),
        abbreviated_id: fields.next()?.into(),
        message: fields.next()?.into(),
        committed_at: fields.next()?.into(),
    })
}

pub(super) fn git_text<const N: usize>(path: &Path, args: [&str; N]) -> Result<String, String> {
    let output = git_output(path, args)?;
    if !output.status.success() {
        return Err("Git could not inspect the scoped worktree.".into());
    }
    text(output.stdout)
}

pub(super) fn git_bytes<const N: usize>(path: &Path, args: [&str; N]) -> Result<Vec<u8>, String> {
    let output = git_output(path, args)?;
    if !output.status.success() {
        return Err("Git could not inspect the scoped worktree.".into());
    }
    Ok(output.stdout)
}

pub(super) fn git_status<const N: usize>(path: &Path, args: [&str; N]) -> bool {
    git_output(path, args)
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn git_output<const N: usize>(path: &Path, args: [&str; N]) -> Result<Output, String> {
    Command::new("git")
        .arg("-C")
        .arg(path)
        .args(args)
        .env("GIT_OPTIONAL_LOCKS", "0")
        .output()
        .map_err(|_| "Git is unavailable for worktree details.".to_string())
}

fn text(bytes: Vec<u8>) -> Result<String, String> {
    String::from_utf8(bytes)
        .map(|value| value.trim().to_owned())
        .map_err(|_| "Git returned non-UTF-8 identity facts.".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn context_separates_branch_dirty_main_and_merge_base_relationship_facts() {
        let directory = tempfile::tempdir().expect("directory");
        let main = directory.path().join("main");
        let selected = directory.path().join("selected");
        std::fs::create_dir(&main).expect("main");
        run(&main, ["init"]);
        run(&main, ["config", "user.email", "fixture@example.test"]);
        run(&main, ["config", "user.name", "Fixture"]);
        std::fs::write(main.join("shared.txt"), "base\n").expect("base");
        run(&main, ["add", "."]);
        run(&main, ["commit", "-m", "base"]);
        run(
            &main,
            [
                "worktree",
                "add",
                "-b",
                "codex/context-fixture",
                selected.to_str().expect("selected"),
            ],
        );
        std::fs::write(selected.join("feature.txt"), "feature\n").expect("feature");
        run(&selected, ["add", "."]);
        run(&selected, ["commit", "-m", "feature commit"]);
        std::fs::write(main.join("main.txt"), "main\n").expect("main change");
        run(&main, ["add", "."]);
        run(&main, ["commit", "-m", "main commit"]);
        std::fs::write(selected.join("staged.txt"), "staged\n").expect("staged");
        run(&selected, ["add", "staged.txt"]);
        std::fs::write(selected.join("feature.txt"), "feature\nunstaged\n").expect("unstaged");
        std::fs::write(selected.join("untracked.txt"), "untracked\n").expect("untracked");

        let scope = WorktreeScope {
            name: "Fixture".into(),
            selected: selected.canonicalize().expect("selected canonical"),
            main: main.canonicalize().expect("main canonical"),
        };
        let context = scope.context().expect("context");
        assert_eq!(context.branch.as_deref(), Some("codex/context-fixture"));
        assert!(!context.detached);
        assert_eq!(
            (
                context.dirty.staged,
                context.dirty.unstaged,
                context.dirty.untracked
            ),
            (1, 1, 1)
        );
        assert_eq!(
            (context.relationship.ahead, context.relationship.behind),
            (1, 1)
        );
        assert_ne!(
            context.relationship.merge_base.as_deref(),
            Some(context.main.head.id.as_str())
        );
        assert_eq!(context.history.len(), 1);
        assert_eq!(context.history[0].message, "feature commit");
        assert!(context.comparison_basis.contains("machine main HEAD"));
        assert!(context.comparison_basis.contains("untracked"));

        run(&selected, ["checkout", "--detach"]);
        let detached = scope.context().expect("detached context");
        assert!(detached.detached);
        assert_eq!(detached.branch, None);
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
