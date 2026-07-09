use super::*;

pub(crate) fn register_task_worktree_anchor(
    conn: &Connection,
    input: RegisterTaskWorktreeCommandInput,
) -> Result<(), String> {
    let project_name = input.project_name.trim().to_string();
    let worktree_path = input.worktree_path.trim().to_string();
    let repo_root_path = match input.repo_root_path.trim() {
        "" => worktree_path.clone(),
        value => value.to_string(),
    };
    validate_non_empty("projectName", &project_name)?;
    validate_non_empty("repoRootPath", &repo_root_path)?;
    validate_non_empty("worktreePath", &worktree_path)?;

    let repo_name = input
        .repo_name
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .unwrap_or_else(|| path_label(&repo_root_path).unwrap_or_else(|| project_name.clone()));
    let branch_name = input
        .branch_name
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string);
    let timestamp = now_iso();

    let project_id = upsert_project(conn, &project_name, &timestamp)?;
    let repo_id = upsert_repo(
        conn,
        &project_id,
        &repo_name,
        &repo_root_path,
        branch_name.as_deref(),
        &timestamp,
    )?;
    let branch_id = match branch_name {
        Some(branch_name) => Some(upsert_branch(conn, &repo_id, &branch_name, &timestamp)?),
        None => None,
    };
    upsert_worktree(
        conn,
        &repo_id,
        branch_id.as_deref(),
        &worktree_path,
        input.is_main.unwrap_or(false),
        &timestamp,
    )?;

    Ok(())
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct GitWorktreeFacts {
    pub(crate) path: String,
    pub(crate) branch_name: Option<String>,
    pub(crate) is_main: bool,
}

pub(crate) fn register_task_repo_anchor(
    conn: &Connection,
    input: RegisterTaskRepoCommandInput,
) -> Result<(), String> {
    let repo_root_path = input.repo_root_path.trim().to_string();
    validate_non_empty("repoRootPath", &repo_root_path)?;

    let git_root_path = git_stdout(&repo_root_path, &["rev-parse", "--show-toplevel"])?
        .trim()
        .to_string();
    validate_non_empty("gitRootPath", &git_root_path)?;

    let repo_label = path_label(&git_root_path).unwrap_or_else(|| "Repository".to_string());
    let project_name = input
        .project_name
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .unwrap_or_else(|| repo_label.clone());
    let repo_name = input
        .repo_name
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .unwrap_or_else(|| repo_label.clone());
    let default_branch = git_default_branch(&git_root_path)?;
    let worktrees = git_worktree_facts(&git_root_path)?;
    let branch_names = git_branch_names(&git_root_path, &worktrees)?;
    let timestamp = now_iso();

    let project_id = upsert_project(conn, &project_name, &timestamp)?;
    let repo_id = upsert_repo(
        conn,
        &project_id,
        &repo_name,
        &git_root_path,
        default_branch.as_deref(),
        &timestamp,
    )?;
    let branch_ids = upsert_branches(conn, &repo_id, &branch_names, &timestamp)?;

    for worktree in worktrees {
        let branch_id = worktree
            .branch_name
            .as_deref()
            .and_then(|branch_name| branch_ids.get(branch_name))
            .map(String::as_str);
        upsert_worktree(
            conn,
            &repo_id,
            branch_id,
            &worktree.path,
            worktree.is_main,
            &timestamp,
        )?;
    }

    Ok(())
}
