use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    process::{Command, Output},
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
    revision: String,
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
    let repository_root = match git_text(&project_path, &["rev-parse", "--show-toplevel"]) {
        Ok(value) => {
            let reported_root = PathBuf::from(value);
            reported_root.canonicalize().unwrap_or(reported_root)
        }
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
    let branches = local_branches(&repository_root)?;
    let current = git_text(
        &repository_root,
        &["symbolic-ref", "--quiet", "--short", "HEAD"],
    )
    .ok();
    let baseline = baseline_branch(&repository_root, &branches, current.as_deref());
    let views = branches
        .iter()
        .map(|branch| {
            branch_view(
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

fn local_branches(repository_root: &Path) -> Result<Vec<LocalBranch>, String> {
    let output = git_text(
        repository_root,
        &[
            "for-each-ref",
            "--format=%(refname:short)%09%(objectname)",
            "refs/heads",
        ],
    )?;
    let mut branches = output
        .lines()
        .filter_map(|line| {
            let (name, revision) = line.split_once('\t')?;
            Some(LocalBranch {
                name: name.to_owned(),
                revision: revision.to_owned(),
            })
        })
        .collect::<Vec<_>>();
    branches.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(branches)
}

fn baseline_branch(
    repository_root: &Path,
    branches: &[LocalBranch],
    current: Option<&str>,
) -> String {
    let origin_head = git_text(
        repository_root,
        &[
            "symbolic-ref",
            "--quiet",
            "--short",
            "refs/remotes/origin/HEAD",
        ],
    )
    .ok()
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
    repository_root: &Path,
    branch: &LocalBranch,
    branches: &[LocalBranch],
    baseline: &str,
    current: Option<&str>,
) -> Result<EpicOriginBranchView, String> {
    if branch.name == baseline {
        return Ok(EpicOriginBranchView {
            name: branch.name.clone(),
            revision: abbreviate(&branch.revision),
            parent_name: None,
            relationship: "related",
            ahead: 0,
            behind: 0,
            fork_revision: abbreviate(&branch.revision),
            is_current: current == Some(branch.name.as_str()),
            is_baseline: true,
        });
    }
    let merge_base = git_text(repository_root, &["merge-base", baseline, &branch.name]).ok();
    let Some(fork_revision) = merge_base else {
        return Ok(EpicOriginBranchView {
            name: branch.name.clone(),
            revision: abbreviate(&branch.revision),
            parent_name: None,
            relationship: "unrelated",
            ahead: 0,
            behind: 0,
            fork_revision: abbreviate(&branch.revision),
            is_current: current == Some(branch.name.as_str()),
            is_baseline: false,
        });
    };
    let (behind, ahead) = ahead_behind(repository_root, baseline, &branch.name)?;
    Ok(EpicOriginBranchView {
        name: branch.name.clone(),
        revision: abbreviate(&branch.revision),
        parent_name: nearest_parent(repository_root, branch, branches, baseline),
        relationship: "related",
        ahead,
        behind,
        fork_revision: abbreviate(&fork_revision),
        is_current: current == Some(branch.name.as_str()),
        is_baseline: false,
    })
}

fn nearest_parent(
    repository_root: &Path,
    branch: &LocalBranch,
    branches: &[LocalBranch],
    baseline: &str,
) -> Option<String> {
    let mut candidates = branches
        .iter()
        .filter(|candidate| candidate.name != branch.name && candidate.revision != branch.revision)
        .filter_map(|candidate| {
            let output = git_output(
                repository_root,
                &["merge-base", "--is-ancestor", &candidate.name, &branch.name],
            )
            .ok()?;
            if !output.status.success() {
                return None;
            }
            let distance = git_text(
                repository_root,
                &[
                    "rev-list",
                    "--count",
                    &format!("{}..{}", candidate.name, branch.name),
                ],
            )
            .ok()?
            .parse::<usize>()
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

fn ahead_behind(
    repository_root: &Path,
    baseline: &str,
    branch: &str,
) -> Result<(usize, usize), String> {
    let range = format!("{baseline}...{branch}");
    let value = git_text(
        repository_root,
        &["rev-list", "--left-right", "--count", &range],
    )?;
    let mut parts = value.split_whitespace();
    let behind = parts
        .next()
        .and_then(|part| part.parse::<usize>().ok())
        .ok_or_else(|| "Git returned an invalid behind count.".to_owned())?;
    let ahead = parts
        .next()
        .and_then(|part| part.parse::<usize>().ok())
        .ok_or_else(|| "Git returned an invalid ahead count.".to_owned())?;
    Ok((behind, ahead))
}

fn git_text(repository_root: &Path, args: &[&str]) -> Result<String, String> {
    let output = git_output(repository_root, args)?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_owned());
    }
    String::from_utf8(output.stdout)
        .map(|value| value.trim().to_owned())
        .map_err(|error| format!("Git returned invalid text: {error}"))
}

fn git_output(repository_root: &Path, args: &[&str]) -> Result<Output, String> {
    Command::new("git")
        .arg("-C")
        .arg(repository_root)
        .args(args)
        .output()
        .map_err(|error| format!("Unable to run Git: {error}"))
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
        let output = git_output(repository, args).unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
