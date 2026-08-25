use crate::repository_context::{RepositoryContext, WorktreeLocation};
use serde::Serialize;
use std::path::{Path, PathBuf};

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
    pub(crate) related_branches: Vec<BranchRelationshipView>,
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

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BranchRelationshipView {
    pub(crate) name: String,
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
        let repository = RepositoryContext::discover_git().map_err(|error| error.to_string())?;
        let paths = repository
            .worktrees(&selected)
            .map_err(|error| error.to_string())?
            .into_iter()
            .filter_map(|record| match record.location {
                WorktreeLocation::Available(root) => Some(root.path().to_path_buf()),
                WorktreeLocation::Unavailable(_) => None,
            })
            .collect::<Vec<_>>();
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
        let repository = RepositoryContext::discover_git().map_err(|error| error.to_string())?;
        let head = commit(&repository, &self.selected, "HEAD")?;
        let main_head = commit(&repository, &self.main, "HEAD")?;
        let selected_branch = repository
            .current_branch(&self.selected)
            .map_err(|error| error.to_string())?;
        let main_branch = repository
            .current_branch(&self.main)
            .map_err(|error| error.to_string())?;
        let divergence = repository
            .divergence(&self.selected, &main_head.id, &head.id)
            .map_err(|error| error.to_string())?;
        let history = repository
            .revision_list(&self.selected, &main_head.id, &head.id, false, Some(20))
            .map_err(|error| error.to_string())?
            .into_iter()
            .map(|id| commit(&repository, &self.selected, id.as_str()))
            .collect::<Result<Vec<_>, _>>()?;
        let related_branches = related_branches(
            &repository,
            &self.selected,
            selected_branch.as_deref(),
            main_branch.as_deref(),
            &head.id,
        )?;
        Ok(WorktreeBuildContextView {
            name: self.name.clone(),
            branch: selected_branch.clone(),
            detached: selected_branch.is_none(),
            head,
            dirty: dirty(&repository, &self.selected)?,
            main: MainCheckoutView {
                branch: main_branch.clone(),
                detached: main_branch.is_none(),
                head: main_head,
                dirty: dirty(&repository, &self.main)?,
            },
            relationship: RelationshipView {
                ahead: divergence.ahead,
                behind: divergence.behind,
                merge_base: divergence.merge_base.map(|value| value.as_str().to_owned()),
                summary: format!(
                    "{} ahead, {} behind machine main HEAD",
                    divergence.ahead, divergence.behind
                ),
            },
            related_branches,
            history,
            comparison_basis: "The file review compares machine main HEAD with the selected worktree's complete current state. It includes committed divergence plus selected staged, unstaged, and untracked changes. Machine-main uncommitted changes are reported here but are not used as the comparison base.".into(),
        })
    }
}

fn related_branches(
    repository: &RepositoryContext,
    path: &Path,
    selected: Option<&str>,
    main: Option<&str>,
    head: &str,
) -> Result<Vec<BranchRelationshipView>, String> {
    let mut relationships = repository
        .local_branches(path)
        .map_err(|error| error.to_string())?
        .into_iter()
        .filter(|(branch, _)| Some(branch.as_str()) != selected && Some(branch.as_str()) != main)
        .filter_map(|(branch, _)| {
            let divergence = repository.divergence(path, &branch, head).ok()?;
            Some(BranchRelationshipView {
                name: branch,
                ahead: divergence.ahead,
                behind: divergence.behind,
                merge_base: divergence.merge_base.map(|value| value.as_str().to_owned()),
                summary: format!(
                    "{} ahead, {} behind this local branch",
                    divergence.ahead, divergence.behind
                ),
            })
        })
        .collect::<Vec<_>>();
    relationships.sort_by_key(|relationship| relationship.ahead + relationship.behind);
    relationships.truncate(8);
    Ok(relationships)
}

pub(super) fn dirty(repository: &RepositoryContext, path: &Path) -> Result<DirtyView, String> {
    let output = repository
        .status_porcelain(path)
        .map_err(|error| error.to_string())?;
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

fn commit(
    repository: &RepositoryContext,
    path: &Path,
    revision: &str,
) -> Result<CommitView, String> {
    let facts = repository
        .commit_facts(path, revision)
        .map_err(|error| error.to_string())?;
    Ok(CommitView {
        id: facts.id.as_str().to_owned(),
        abbreviated_id: facts.abbreviated_id,
        message: facts.subject,
        committed_at: facts.committed_at,
    })
}

// Compatibility seam for the adjacent runtime-authority adapter. It intentionally recognizes
// only the repository identity queries that adapter uses; it is not a generic Git escape hatch.
pub(super) fn git_text<const N: usize>(path: &Path, args: [&str; N]) -> Result<String, String> {
    let repository = RepositoryContext::discover_git().map_err(|error| error.to_string())?;
    match args.as_slice() {
        ["rev-parse", "HEAD"] => repository
            .resolve_commit(path, "HEAD")
            .map(|value| value.as_str().to_owned())
            .map_err(|error| error.to_string()),
        ["rev-parse", "--git-common-dir"] => repository
            .repository(path)
            .map(|value| value.common_dir.path().to_string_lossy().into_owned())
            .map_err(|error| error.to_string()),
        ["rev-parse", "--verify", revision] => {
            let revision = revision.strip_suffix("^{commit}").unwrap_or(revision);
            repository
                .resolve_commit(path, revision)
                .map(|value| value.as_str().to_owned())
                .map_err(|error| error.to_string())
        }
        _ => Err("That repository identity query is unavailable.".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

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
        run(&main, ["branch", "codex/parent", "HEAD"]);
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
        assert!(context
            .related_branches
            .iter()
            .any(|relationship| relationship.name == "codex/parent"));
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
