use super::application::SessionNavigationService;
use serde::{Deserialize, Serialize};
use std::{path::Path, process::Command};

#[derive(Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum GitAction {
    Create,
    Rename,
    Delete,
    Fetch,
    Merge,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct GitActionInput {
    pub(crate) repository_id: String,
    pub(crate) branch_ref: Option<String>,
    pub(crate) action: GitAction,
    pub(crate) value: String,
    pub(crate) start_point: String,
}
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GitActionPreview {
    path: String,
    arguments: Vec<String>,
    status: String,
}

fn git(path: &Path, arguments: &[String]) -> Result<String, String> {
    let mut command = Command::new("git");
    command
        .current_dir(path)
        .args(arguments)
        .env("GIT_TERMINAL_PROMPT", "0");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let output = command.output().map_err(|e| e.to_string())?;
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    if output.status.success() {
        Ok(text)
    } else {
        Err(text)
    }
}
fn args(parts: &[&str]) -> Vec<String> {
    parts.iter().map(|s| (*s).into()).collect()
}
fn branch_name(path: &Path, value: &str) -> Result<String, String> {
    let name = value
        .trim()
        .strip_prefix("refs/heads/")
        .unwrap_or(value.trim());
    if name.is_empty() || name.starts_with('-') || name == "HEAD" {
        return Err("Enter a valid branch name".into());
    }
    git(
        path,
        &args(&["check-ref-format", &format!("refs/heads/{name}")]),
    )?;
    Ok(name.into())
}
impl SessionNavigationService {
    fn git_plan(
        &self,
        input: &GitActionInput,
    ) -> Result<(std::path::PathBuf, Vec<String>), String> {
        let mut path = self
            .catalog
            .resolve_main_working_tree(&input.repository_id)?;
        let name = input
            .branch_ref
            .as_deref()
            .map(|b| branch_name(&path, b))
            .transpose()?;
        let arguments = match input.action {
            GitAction::Create => {
                let next = branch_name(&path, &input.value)?;
                let start = git(
                    &path,
                    &args(&[
                        "rev-parse",
                        "--verify",
                        "--end-of-options",
                        &format!("{}^{{commit}}", input.start_point.trim()),
                    ]),
                )?;
                args(&["branch", "--", &next, start.trim()])
            }
            GitAction::Rename => args(&[
                "branch",
                "-m",
                "--",
                name.as_deref().ok_or("Choose a branch")?,
                &branch_name(&path, &input.value)?,
            ]),
            GitAction::Delete => args(&[
                "branch",
                "-d",
                "--",
                name.as_deref().ok_or("Choose a branch")?,
            ]),
            GitAction::Fetch => args(&["fetch", "--all"]),
            GitAction::Merge => {
                let branch = name.as_deref().ok_or("Choose a branch")?;
                let root = path.to_string_lossy().into_owned();
                let worktrees = self.execution_targets.endpoints.worktrees(
                    &Default::default(),
                    &root,
                    Some(&format!("refs/heads/{branch}")),
                )?;
                if worktrees.len() != 1 {
                    return Err(
                        "Fast-forward merge requires one local worktree checked out on this branch"
                            .into(),
                    );
                }
                path = worktrees[0].path.clone().into();
                let commit = git(
                    &path,
                    &args(&[
                        "rev-parse",
                        "--verify",
                        "--end-of-options",
                        &format!("{}^{{commit}}", input.value.trim()),
                    ]),
                )?;
                // Native Git also enforces safe updates; require a clean index and worktree here.
                if !git(&path, &args(&["status", "--porcelain"]))?.is_empty() {
                    return Err("Commit or stash the worktree changes before merging".into());
                }
                args(&["merge", "--ff-only", "--", commit.trim()])
            }
        };
        Ok((path, arguments))
    }
    pub(crate) fn preview_git_action(
        &self,
        input: &GitActionInput,
    ) -> Result<GitActionPreview, String> {
        let (path, arguments) = self.git_plan(input)?;
        Ok(GitActionPreview {
            path: path.to_string_lossy().into_owned(),
            arguments,
            status: git(&path, &args(&["status", "--short", "--branch"]))?,
        })
    }
    pub(crate) fn execute_git_action(
        &self,
        input: &GitActionInput,
        expected: &GitActionPreview,
    ) -> Result<String, String> {
        let (path, arguments) = self.git_plan(input)?;
        if path.to_string_lossy() != expected.path || arguments != expected.arguments {
            return Err("The Git target changed since preview. Preview the command again.".into());
        }
        let output = git(&path, &arguments)?;
        self.branch_inventory.invalidate();
        Ok(if output.trim().is_empty() {
            "Git command completed.".into()
        } else {
            output
        })
    }
    pub(crate) fn inspect_navigation_worktree(
        &self,
        repository_id: &str,
        branch_ref: &str,
        device_id: &str,
        path: &str,
    ) -> Result<orchid_engine::protocol::WorktreeInspection, String> {
        let execution = if device_id == "local" {
            Default::default()
        } else {
            self.execution_targets
                .devices()?
                .into_iter()
                .find(|d| d.device_id == device_id)
                .and_then(|d| d.profiles.into_iter().next())
                .ok_or("Device execution route is unavailable")?
                .execution
        };
        let root = self
            .execution_targets
            .repository_root(repository_id, &execution)?;
        let worktrees =
            self.execution_targets
                .endpoints
                .worktrees(&execution, &root, Some(branch_ref))?;
        let worktree = worktrees
            .iter()
            .find(|w| w.path == path && w.branch_ref.as_deref() == Some(branch_ref))
            .ok_or("Worktree is no longer attached to this branch")?;
        self.execution_targets
            .endpoints
            .inspect_worktree(&execution, &worktree.path, None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_options_and_invalid_branch_names() {
        let root = std::env::temp_dir();
        for name in ["", "-D", "HEAD", "bad name", "main..other", "a:b"] {
            assert!(branch_name(&root, name).is_err(), "{name}");
        }
        assert_eq!(
            branch_name(&root, "refs/heads/feature/test").unwrap(),
            "feature/test"
        );
    }
}
