use super::*;

pub(crate) fn upsert_branches(
    conn: &Connection,
    repo_id: &str,
    branch_names: &[String],
    timestamp: &str,
) -> Result<BTreeMap<String, String>, String> {
    let mut branch_ids = BTreeMap::new();

    for branch_name in branch_names {
        branch_ids.insert(
            branch_name.clone(),
            upsert_branch(conn, repo_id, branch_name, timestamp)?,
        );
    }

    Ok(branch_ids)
}

pub(crate) fn upsert_project(
    conn: &Connection,
    name: &str,
    timestamp: &str,
) -> Result<String, String> {
    if let Some(project_id) = conn
        .query_row(
            "SELECT id FROM projects WHERE name = ?1 ORDER BY created_at, id LIMIT 1",
            params![name],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(sql_error("read project by name"))?
    {
        conn.execute(
            "UPDATE projects SET updated_at = ?1 WHERE id = ?2",
            params![timestamp, project_id],
        )
        .map_err(sql_error("update project"))?;
        return Ok(project_id);
    }

    let project_id = Uuid::new_v4().to_string();
    conn.execute(
        "
INSERT INTO projects (id, name, description, created_at, updated_at)
VALUES (?1, ?2, NULL, ?3, ?3)
",
        params![project_id, name, timestamp],
    )
    .map_err(sql_error("create project"))?;
    Ok(project_id)
}

pub(crate) fn upsert_repo(
    conn: &Connection,
    project_id: &str,
    name: &str,
    root_path: &str,
    default_branch: Option<&str>,
    timestamp: &str,
) -> Result<String, String> {
    if let Some(repo_id) = conn
        .query_row(
            "SELECT id FROM repos WHERE project_id = ?1 AND root_path = ?2",
            params![project_id, root_path],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(sql_error("read repo by path"))?
    {
        conn.execute(
            "
UPDATE repos SET name = ?1, default_branch = ?2, updated_at = ?3 WHERE id = ?4
",
            params![name, default_branch, timestamp, repo_id],
        )
        .map_err(sql_error("update repo"))?;
        return Ok(repo_id);
    }

    let repo_id = Uuid::new_v4().to_string();
    conn.execute(
        "
INSERT INTO repos (id, project_id, name, root_path, default_branch, remote_url, created_at, updated_at)
VALUES (?1, ?2, ?3, ?4, ?5, NULL, ?6, ?6)
",
        params![repo_id, project_id, name, root_path, default_branch, timestamp],
    )
    .map_err(sql_error("create repo"))?;
    Ok(repo_id)
}

pub(crate) fn upsert_branch(
    conn: &Connection,
    repo_id: &str,
    name: &str,
    timestamp: &str,
) -> Result<String, String> {
    if let Some(branch_id) = conn
        .query_row(
            "SELECT id FROM branches WHERE repo_id = ?1 AND name = ?2",
            params![repo_id, name],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(sql_error("read branch by name"))?
    {
        conn.execute(
            "UPDATE branches SET updated_at = ?1 WHERE id = ?2",
            params![timestamp, branch_id],
        )
        .map_err(sql_error("update branch"))?;
        return Ok(branch_id);
    }

    let branch_id = Uuid::new_v4().to_string();
    conn.execute(
        "
INSERT INTO branches (id, repo_id, name, base_branch, head_sha, intent, created_at, updated_at)
VALUES (?1, ?2, ?3, NULL, NULL, NULL, ?4, ?4)
",
        params![branch_id, repo_id, name, timestamp],
    )
    .map_err(sql_error("create branch"))?;
    Ok(branch_id)
}

pub(crate) fn upsert_worktree(
    conn: &Connection,
    repo_id: &str,
    branch_id: Option<&str>,
    path: &str,
    is_main: bool,
    timestamp: &str,
) -> Result<String, String> {
    if let Some(worktree_id) = conn
        .query_row(
            "SELECT id FROM worktrees WHERE repo_id = ?1 AND path = ?2",
            params![repo_id, path],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(sql_error("read worktree by path"))?
    {
        conn.execute(
            "
UPDATE worktrees SET branch_id = ?1, is_main = ?2, last_scanned_at = ?3, updated_at = ?3
WHERE id = ?4
",
            params![branch_id, bool_to_sqlite(is_main), timestamp, worktree_id],
        )
        .map_err(sql_error("update worktree"))?;
        return Ok(worktree_id);
    }

    let worktree_id = Uuid::new_v4().to_string();
    conn.execute(
        "
INSERT INTO worktrees (
  id, repo_id, branch_id, path, is_main, is_dirty, lock_reason, last_scanned_at, created_at, updated_at
) VALUES (?1, ?2, ?3, ?4, ?5, 0, NULL, ?6, ?6, ?6)
",
        params![
            worktree_id,
            repo_id,
            branch_id,
            path,
            bool_to_sqlite(is_main),
            timestamp
        ],
    )
    .map_err(sql_error("create worktree"))?;
    Ok(worktree_id)
}

pub(crate) fn resolve_create_task_anchor(
    conn: &Connection,
    project_id: &str,
    repo_id: Option<String>,
    branch_id: Option<String>,
    worktree_id: Option<String>,
) -> Result<(Option<String>, Option<String>, Option<String>), String> {
    if let Some(worktree_id) = worktree_id {
        validate_non_empty("worktreeId", &worktree_id)?;
        let (anchor_project_id, anchor_repo_id, anchor_branch_id) =
            select_worktree_task_anchor(conn, &worktree_id)?
                .ok_or_else(|| format!("Worktree not found: {worktree_id}"))?;
        ensure_same_anchor("projectId", project_id, &anchor_project_id)?;

        if let Some(repo_id) = repo_id.as_deref() {
            ensure_same_anchor("repoId", repo_id, &anchor_repo_id)?;
        }

        if let Some(branch_id) = branch_id.as_deref() {
            match anchor_branch_id.as_deref() {
                Some(anchor_branch_id) => {
                    ensure_same_anchor("branchId", branch_id, anchor_branch_id)?
                }
                None => return Err(format!("Branch does not belong to worktree: {branch_id}")),
            }
        }

        return Ok((Some(anchor_repo_id), anchor_branch_id, Some(worktree_id)));
    }

    if let Some(branch_id) = branch_id {
        validate_non_empty("branchId", &branch_id)?;
        let (anchor_project_id, anchor_repo_id) = select_branch_task_anchor(conn, &branch_id)?
            .ok_or_else(|| format!("Branch not found: {branch_id}"))?;
        ensure_same_anchor("projectId", project_id, &anchor_project_id)?;

        if let Some(repo_id) = repo_id.as_deref() {
            ensure_same_anchor("repoId", repo_id, &anchor_repo_id)?;
        }

        return Ok((Some(anchor_repo_id), Some(branch_id), None));
    }

    if let Some(repo_id) = repo_id {
        validate_non_empty("repoId", &repo_id)?;
        let anchor_project_id = select_repo_project_id(conn, &repo_id)?
            .ok_or_else(|| format!("Repo not found: {repo_id}"))?;
        ensure_same_anchor("projectId", project_id, &anchor_project_id)?;
        return Ok((Some(repo_id), None, None));
    }

    Ok((None, None, None))
}
