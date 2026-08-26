use crate::repository_context::{ObjectId, RepositoryContext};
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct InspectEpicOriginProjectInput {
    path: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EpicOriginProjectView {
    name: String,
    path: String,
    git_detected: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    repository_root: Option<String>,
    branches: Vec<EpicOriginBranchView>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct EpicOriginBranchView {
    name: String,
    revision: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    parent_name: Option<String>,
    relationship: &'static str,
    ahead: usize,
    behind: usize,
    fork_revision: String,
    is_current: bool,
    is_baseline: bool,
}

#[derive(Clone, Debug)]
struct LocalBranch {
    name: String,
    revision: ObjectId,
}

#[tauri::command]
pub(crate) fn choose_epic_origin_project() -> Result<Option<EpicOriginProjectView>, String> {
    choose_project_folder()?
        .map(|path| inspect_project(&path))
        .transpose()
}

#[tauri::command]
pub(crate) fn inspect_epic_origin_project(
    input: InspectEpicOriginProjectInput,
) -> Result<EpicOriginProjectView, String> {
    inspect_project(Path::new(&input.path))
}

fn inspect_project(project_path: &Path) -> Result<EpicOriginProjectView, String> {
    if !project_path.is_dir() {
        return Err("The selected project folder does not exist.".to_owned());
    }
    let project_path = project_path
        .canonicalize()
        .map_err(|error| format!("Unable to resolve the selected project folder: {error}"))?;
    let name = project_path
        .file_name()
        .and_then(|value| value.to_str())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("Project")
        .to_owned();
    let path = project_path.to_string_lossy().into_owned();
    let context = match RepositoryContext::discover() {
        Ok(context) => context,
        Err(_) => {
            return Ok(EpicOriginProjectView {
                name,
                path,
                git_detected: false,
                repository_root: None,
                branches: Vec::new(),
            });
        }
    };
    let repository = match context.identities().inspect(&project_path) {
        Ok(repository) => repository,
        Err(_) => {
            return Ok(EpicOriginProjectView {
                name,
                path,
                git_detected: false,
                repository_root: None,
                branches: Vec::new(),
            });
        }
    };
    let repository_root = repository.top_level.path().to_path_buf();
    let branches = local_branches(&context, &repository_root)?;
    let current = context
        .references()
        .current_head_ref(&repository_root)
        .map_err(|error| error.to_string())?
        .and_then(|reference| reference.branch_name().map(str::to_owned));
    let baseline = baseline_branch(&context, &repository_root, &branches, current.as_deref());
    let views = branches
        .iter()
        .map(|branch| {
            branch_view(
                &context,
                &repository_root,
                branch,
                &branches,
                &baseline,
                current.as_deref(),
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(EpicOriginProjectView {
        name,
        path,
        git_detected: true,
        repository_root: Some(repository_root.to_string_lossy().into_owned()),
        branches: views,
    })
}

fn local_branches(
    context: &RepositoryContext,
    repository_root: &Path,
) -> Result<Vec<LocalBranch>, String> {
    let mut branches = context
        .references()
        .local_branches(repository_root)
        .map_err(|error| error.to_string())?
        .into_iter()
        .filter_map(|branch| {
            Some(LocalBranch {
                name: branch.full_name.branch_name()?.to_owned(),
                revision: branch.object_id,
            })
        })
        .collect::<Vec<_>>();
    branches.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(branches)
}

fn baseline_branch(
    context: &RepositoryContext,
    repository_root: &Path,
    branches: &[LocalBranch],
    current: Option<&str>,
) -> String {
    let origin_head = context
        .references()
        .remote_default_branch(repository_root)
        .ok()
        .flatten()
        .map(|reference| reference.display_name().to_owned())
        .and_then(|value| value.strip_prefix("origin/").map(str::to_owned));
    let baseline = [
        origin_head.as_deref(),
        Some("main"),
        Some("master"),
        current,
    ]
    .into_iter()
    .flatten()
    .find(|candidate| branches.iter().any(|branch| branch.name == *candidate))
    .map(str::to_owned)
    .or_else(|| branches.first().map(|branch| branch.name.clone()))
    .unwrap_or_default();
    baseline
}

fn branch_view(
    context: &RepositoryContext,
    repository_root: &Path,
    branch: &LocalBranch,
    branches: &[LocalBranch],
    baseline: &str,
    current: Option<&str>,
) -> Result<EpicOriginBranchView, String> {
    if branch.name == baseline {
        return Ok(EpicOriginBranchView {
            name: branch.name.clone(),
            revision: abbreviate(branch.revision.as_str()),
            parent_name: None,
            relationship: "related",
            ahead: 0,
            behind: 0,
            fork_revision: abbreviate(branch.revision.as_str()),
            is_current: current == Some(branch.name.as_str()),
            is_baseline: true,
        });
    }
    let baseline_revision = branches
        .iter()
        .find(|candidate| candidate.name == baseline)
        .map(|candidate| &candidate.revision)
        .ok_or_else(|| "The baseline branch is unavailable.".to_owned())?;
    let divergence = context
        .commits()
        .divergence(repository_root, baseline_revision, &branch.revision)
        .map_err(|error| error.to_string())?;
    let Some(fork_revision) = divergence.merge_base else {
        return Ok(EpicOriginBranchView {
            name: branch.name.clone(),
            revision: abbreviate(branch.revision.as_str()),
            parent_name: None,
            relationship: "unrelated",
            ahead: 0,
            behind: 0,
            fork_revision: abbreviate(branch.revision.as_str()),
            is_current: current == Some(branch.name.as_str()),
            is_baseline: false,
        });
    };
    Ok(EpicOriginBranchView {
        name: branch.name.clone(),
        revision: abbreviate(branch.revision.as_str()),
        parent_name: nearest_parent(context, repository_root, branch, branches, baseline),
        relationship: "related",
        ahead: divergence.ahead,
        behind: divergence.behind,
        fork_revision: abbreviate(fork_revision.as_str()),
        is_current: current == Some(branch.name.as_str()),
        is_baseline: false,
    })
}

fn nearest_parent(
    context: &RepositoryContext,
    repository_root: &Path,
    branch: &LocalBranch,
    branches: &[LocalBranch],
    baseline: &str,
) -> Option<String> {
    let mut candidates = branches
        .iter()
        .filter(|candidate| candidate.name != branch.name && candidate.revision != branch.revision)
        .filter_map(|candidate| {
            if !context
                .commits()
                .is_ancestor(repository_root, &candidate.revision, &branch.revision)
                .ok()?
            {
                return None;
            }
            let distance = context
                .commits()
                .commit_count(repository_root, &candidate.revision, &branch.revision)
                .ok()?;
            Some((distance, candidate.name.clone()))
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| left.cmp(right));
    candidates
        .first()
        .map(|(_, name)| name.clone())
        .or_else(|| (!baseline.is_empty()).then(|| baseline.to_owned()))
}

fn abbreviate(revision: &str) -> String {
    revision.chars().take(12).collect()
}

#[cfg(windows)]
fn choose_project_folder() -> Result<Option<PathBuf>, String> {
    use std::os::windows::process::CommandExt;

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    const SCRIPT: &str = r#"
Add-Type -AssemblyName System.Windows.Forms
$dialog = New-Object System.Windows.Forms.FolderBrowserDialog
$dialog.Description = 'Choose a project folder for this Epic'
$dialog.ShowNewFolderButton = $false
if ($dialog.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) {
  [Console]::Out.Write($dialog.SelectedPath)
}
"#;
    let output = Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-STA", "-Command", SCRIPT])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|error| format!("Unable to open the project folder picker: {error}"))?;
    if !output.status.success() {
        return Err("The project folder picker could not be opened.".to_owned());
    }
    let selected = String::from_utf8(output.stdout)
        .map_err(|error| format!("The project folder picker returned invalid text: {error}"))?;
    let selected = selected.trim();
    Ok((!selected.is_empty()).then(|| PathBuf::from(selected)))
}

#[cfg(not(windows))]
fn choose_project_folder() -> Result<Option<PathBuf>, String> {
    Err("Project folder selection is currently available on Windows.".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn inspects_a_repository_and_derives_local_branch_lineage() {
        let repository = TempDir::new().unwrap();
        git_ok(repository.path(), &["init", "-b", "main"]);
        git_ok(
            repository.path(),
            &["config", "user.email", "codex@example.test"],
        );
        git_ok(repository.path(), &["config", "user.name", "Codex"]);
        commit(repository.path(), "root.txt", "root", "root");
        git_ok(repository.path(), &["switch", "-c", "feature/parent"]);
        commit(repository.path(), "parent.txt", "parent", "parent");
        git_ok(repository.path(), &["switch", "-c", "feature/child"]);
        commit(repository.path(), "child.txt", "child", "child");

        let view = inspect_project(repository.path()).unwrap();

        assert!(view.git_detected);
        let main = branch(&view, "main");
        assert!(main.is_baseline);
        assert_eq!(
            branch(&view, "feature/parent").parent_name.as_deref(),
            Some("main")
        );
        let child = branch(&view, "feature/child");
        assert_eq!(child.parent_name.as_deref(), Some("feature/parent"));
        assert!(child.is_current);
        assert_eq!(child.ahead, 2);
    }

    #[test]
    fn reports_a_non_repository_without_inventing_branches() {
        let project = TempDir::new().unwrap();
        let view = inspect_project(project.path()).unwrap();

        assert!(!view.git_detected);
        assert!(view.repository_root.is_none());
        assert!(view.branches.is_empty());
    }

    fn branch<'a>(view: &'a EpicOriginProjectView, name: &str) -> &'a EpicOriginBranchView {
        view.branches
            .iter()
            .find(|branch| branch.name == name)
            .unwrap()
    }

    fn commit(repository: &Path, file: &str, contents: &str, message: &str) {
        fs::write(repository.join(file), contents).unwrap();
        git_ok(repository, &["add", file]);
        git_ok(repository, &["commit", "-m", message]);
    }

    fn git_ok(repository: &Path, args: &[&str]) {
        let output = Command::new("git")
            .arg("-C")
            .arg(repository)
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
