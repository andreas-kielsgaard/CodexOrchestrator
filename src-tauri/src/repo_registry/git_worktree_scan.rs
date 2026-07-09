use super::*;

pub(crate) fn git_stdout(cwd: &str, args: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(cwd)
        .args(args)
        .output()
        .map_err(|error| format!("Failed to launch git: {error}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            format!("Git command failed: git -C {cwd} {}", args.join(" "))
        } else {
            stderr
        });
    }

    String::from_utf8(output.stdout).map_err(|error| format!("Git output was not UTF-8: {error}"))
}

pub(crate) fn git_default_branch(repo_root_path: &str) -> Result<Option<String>, String> {
    if let Ok(default_branch) = git_stdout(
        repo_root_path,
        &[
            "symbolic-ref",
            "--quiet",
            "--short",
            "refs/remotes/origin/HEAD",
        ],
    ) {
        let normalized = default_branch
            .trim()
            .strip_prefix("origin/")
            .unwrap_or(default_branch.trim())
            .to_string();

        if !normalized.is_empty() {
            return Ok(Some(normalized));
        }
    }

    let current_branch = git_stdout(repo_root_path, &["branch", "--show-current"])?;
    let current_branch = current_branch.trim();

    if current_branch.is_empty() {
        Ok(None)
    } else {
        Ok(Some(current_branch.to_string()))
    }
}

pub(crate) fn git_branch_names(
    repo_root_path: &str,
    worktrees: &[GitWorktreeFacts],
) -> Result<Vec<String>, String> {
    let mut names = HashSet::new();
    let branch_output = git_stdout(repo_root_path, &["branch", "--format=%(refname:short)"])?;

    for branch_name in branch_output.lines().map(str::trim) {
        if !branch_name.is_empty() {
            names.insert(branch_name.to_string());
        }
    }

    for worktree in worktrees {
        if let Some(branch_name) = worktree.branch_name.as_deref() {
            names.insert(branch_name.to_string());
        }
    }

    let mut names = names.into_iter().collect::<Vec<_>>();
    names.sort();
    Ok(names)
}

pub(crate) fn git_worktree_facts(repo_root_path: &str) -> Result<Vec<GitWorktreeFacts>, String> {
    let output = git_stdout(repo_root_path, &["worktree", "list", "--porcelain"])?;
    let worktrees = parse_git_worktree_list(&output, repo_root_path);

    if worktrees.is_empty() {
        Ok(vec![GitWorktreeFacts {
            path: repo_root_path.to_string(),
            branch_name: git_default_branch(repo_root_path)?,
            is_main: true,
        }])
    } else {
        Ok(worktrees)
    }
}

pub(crate) fn parse_git_worktree_list(output: &str, repo_root_path: &str) -> Vec<GitWorktreeFacts> {
    let mut worktrees = Vec::new();
    let mut current_path: Option<String> = None;
    let mut current_branch: Option<String> = None;
    let mut is_bare = false;

    for line in output.lines() {
        if let Some(path) = line.strip_prefix("worktree ") {
            push_git_worktree_fact(
                &mut worktrees,
                current_path.take(),
                current_branch.take(),
                is_bare,
                repo_root_path,
            );
            current_path = Some(path.to_string());
            current_branch = None;
            is_bare = false;
            continue;
        }

        if let Some(branch_ref) = line.strip_prefix("branch ") {
            current_branch = normalize_git_branch_ref(branch_ref);
        } else if line == "bare" {
            is_bare = true;
        }
    }

    push_git_worktree_fact(
        &mut worktrees,
        current_path,
        current_branch,
        is_bare,
        repo_root_path,
    );

    worktrees
}

pub(crate) fn push_git_worktree_fact(
    worktrees: &mut Vec<GitWorktreeFacts>,
    path: Option<String>,
    branch_name: Option<String>,
    is_bare: bool,
    repo_root_path: &str,
) {
    if is_bare {
        return;
    }

    if let Some(path) = path.filter(|value| !value.trim().is_empty()) {
        worktrees.push(GitWorktreeFacts {
            is_main: same_filesystem_path(&path, repo_root_path),
            path,
            branch_name,
        });
    }
}
