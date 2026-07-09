use super::*;

pub(crate) fn select_task(conn: &Connection, task_id: &str) -> Result<Option<TaskRow>, String> {
    conn.query_row(
        "
SELECT id, project_id, repo_id, branch_id, worktree_id, title, summary, execution_state,
  attention_state, priority, updated_at
FROM tasks
WHERE id = ?1
",
        params![task_id],
        map_task_row,
    )
    .optional()
    .map_err(sql_error("load open task"))
}

pub(crate) fn select_tasks(conn: &Connection) -> Result<Vec<TaskRow>, String> {
    let mut stmt = conn
        .prepare(
            "
SELECT id, project_id, repo_id, branch_id, worktree_id, title, summary, execution_state,
  attention_state, priority, updated_at
FROM tasks
ORDER BY updated_at DESC, id
",
        )
        .map_err(sql_error("prepare task dashboard query"))?;

    let rows = stmt
        .query_map([], map_task_row)
        .map_err(sql_error("query task dashboard rows"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(sql_error("read task dashboard rows"))?;

    Ok(rows)
}

pub(crate) fn select_projects(conn: &Connection) -> Result<Vec<ProjectRow>, String> {
    let mut stmt = conn
        .prepare("SELECT id, name FROM projects ORDER BY id")
        .map_err(sql_error("prepare projects query"))?;

    let rows = stmt
        .query_map([], |row| {
            Ok(ProjectRow {
                id: row.get(0)?,
                name: row.get(1)?,
            })
        })
        .map_err(sql_error("query project rows"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(sql_error("read project rows"))?;

    Ok(rows)
}

pub(crate) fn select_repos(conn: &Connection) -> Result<Vec<RepoRow>, String> {
    let mut stmt = conn
        .prepare("SELECT id, project_id, name, root_path FROM repos ORDER BY id")
        .map_err(sql_error("prepare repos query"))?;

    let rows = stmt
        .query_map([], |row| {
            Ok(RepoRow {
                id: row.get(0)?,
                project_id: row.get(1)?,
                name: row.get(2)?,
                root_path: row.get(3)?,
            })
        })
        .map_err(sql_error("query repo rows"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(sql_error("read repo rows"))?;

    Ok(rows)
}

pub(crate) fn select_branches(conn: &Connection) -> Result<Vec<BranchRow>, String> {
    let mut stmt = conn
        .prepare("SELECT id, name FROM branches ORDER BY id")
        .map_err(sql_error("prepare branches query"))?;

    let rows = stmt
        .query_map([], |row| {
            Ok(BranchRow {
                id: row.get(0)?,
                name: row.get(1)?,
            })
        })
        .map_err(sql_error("query branch rows"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(sql_error("read branch rows"))?;

    Ok(rows)
}

pub(crate) fn select_worktrees(conn: &Connection) -> Result<Vec<WorktreeRow>, String> {
    let mut stmt = conn
        .prepare("SELECT id, path FROM worktrees ORDER BY id")
        .map_err(sql_error("prepare worktrees query"))?;

    let rows = stmt
        .query_map([], |row| {
            Ok(WorktreeRow {
                id: row.get(0)?,
                path: row.get(1)?,
            })
        })
        .map_err(sql_error("query worktree rows"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(sql_error("read worktree rows"))?;

    Ok(rows)
}

pub(crate) fn select_dashboard_worktree_anchors(
    conn: &Connection,
) -> Result<Vec<TaskDashboardWorktreeAnchor>, String> {
    let mut stmt = conn
        .prepare(
            "
SELECT
  worktrees.id,
  projects.id,
  projects.name,
  repos.id,
  repos.name,
  branches.id,
  branches.name,
  worktrees.path
FROM worktrees
JOIN repos ON repos.id = worktrees.repo_id
JOIN projects ON projects.id = repos.project_id
LEFT JOIN branches ON branches.id = worktrees.branch_id
ORDER BY projects.name, repos.name, worktrees.path, worktrees.id
",
        )
        .map_err(sql_error("prepare worktree anchor query"))?;

    let rows = stmt
        .query_map([], |row| {
            Ok(TaskDashboardWorktreeAnchor {
                id: row.get(0)?,
                project_id: row.get(1)?,
                project: row.get(2)?,
                repo_id: row.get(3)?,
                repo: row.get(4)?,
                branch_id: row.get(5)?,
                branch: row.get(6)?,
                path: row.get(7)?,
            })
        })
        .map_err(sql_error("query worktree anchor rows"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(sql_error("read worktree anchor rows"))?;

    Ok(rows)
}

pub(crate) fn select_worktree_task_anchor(
    conn: &Connection,
    worktree_id: &str,
) -> Result<Option<(String, String, Option<String>)>, String> {
    conn.query_row(
        "
SELECT repos.project_id, worktrees.repo_id, worktrees.branch_id
FROM worktrees
JOIN repos ON repos.id = worktrees.repo_id
WHERE worktrees.id = ?1
",
        params![worktree_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )
    .optional()
    .map_err(sql_error("read worktree task anchor"))
}

pub(crate) fn select_branch_task_anchor(
    conn: &Connection,
    branch_id: &str,
) -> Result<Option<(String, String)>, String> {
    conn.query_row(
        "
SELECT repos.project_id, branches.repo_id
FROM branches
JOIN repos ON repos.id = branches.repo_id
WHERE branches.id = ?1
",
        params![branch_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )
    .optional()
    .map_err(sql_error("read branch task anchor"))
}

pub(crate) fn select_repo_project_id(
    conn: &Connection,
    repo_id: &str,
) -> Result<Option<String>, String> {
    conn.query_row(
        "SELECT project_id FROM repos WHERE id = ?1",
        params![repo_id],
        |row| row.get(0),
    )
    .optional()
    .map_err(sql_error("read repo project id"))
}

pub(crate) fn select_detail_task(
    conn: &Connection,
    task_id: &str,
) -> Result<Option<DetailTask>, String> {
    let task = conn
        .query_row(
            "
SELECT id, project_id, repo_id, branch_id, worktree_id, title, summary, execution_state,
  attention_state, priority, due_at, snoozed_until, created_at, updated_at
FROM tasks
WHERE id = ?1
",
            params![task_id],
            |row| {
                Ok(DetailTask {
                    id: row.get(0)?,
                    project_id: row.get(1)?,
                    repo_id: row.get(2)?,
                    branch_id: row.get(3)?,
                    worktree_id: row.get(4)?,
                    conversation_ids: Vec::new(),
                    title: row.get(5)?,
                    summary: row.get(6)?,
                    execution_state: row.get(7)?,
                    attention_state: row.get(8)?,
                    priority: row.get(9)?,
                    due_at: row.get(10)?,
                    snoozed_until: row.get(11)?,
                    created_at: row.get(12)?,
                    updated_at: row.get(13)?,
                })
            },
        )
        .optional()
        .map_err(sql_error("load task detail task"))?;

    match task {
        Some(mut task) => {
            task.conversation_ids = select_task_conversation_ids(conn, task_id)?;
            Ok(Some(task))
        }
        None => Ok(None),
    }
}

pub(crate) fn select_task_conversation_ids(
    conn: &Connection,
    task_id: &str,
) -> Result<Vec<String>, String> {
    let mut stmt = conn
        .prepare(
            "
SELECT conversation_id
FROM task_conversation_links
WHERE task_id = ?1
ORDER BY position, conversation_id
",
        )
        .map_err(sql_error("prepare task conversation links query"))?;

    let rows = stmt
        .query_map(params![task_id], |row| row.get(0))
        .map_err(sql_error("query task conversation link rows"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(sql_error("read task conversation link rows"))?;

    Ok(rows)
}

pub(crate) fn select_detail_project(
    conn: &Connection,
    project_id: &str,
) -> Result<Option<DetailProject>, String> {
    conn.query_row(
        "
SELECT id, name, description, created_at, updated_at
FROM projects
WHERE id = ?1
",
        params![project_id],
        |row| {
            Ok(DetailProject {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
            })
        },
    )
    .optional()
    .map_err(sql_error("load task detail project"))
}

pub(crate) fn select_detail_repo(
    conn: &Connection,
    repo_id: &str,
) -> Result<Option<DetailRepo>, String> {
    conn.query_row(
        "
SELECT id, project_id, name, root_path, default_branch, remote_url, created_at, updated_at
FROM repos
WHERE id = ?1
",
        params![repo_id],
        |row| {
            Ok(DetailRepo {
                id: row.get(0)?,
                project_id: row.get(1)?,
                name: row.get(2)?,
                root_path: row.get(3)?,
                default_branch: row.get(4)?,
                remote_url: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        },
    )
    .optional()
    .map_err(sql_error("load task detail repo"))
}

pub(crate) fn select_detail_branch(
    conn: &Connection,
    branch_id: &str,
) -> Result<Option<DetailBranch>, String> {
    conn.query_row(
        "
SELECT id, repo_id, name, base_branch, head_sha, intent, created_at, updated_at
FROM branches
WHERE id = ?1
",
        params![branch_id],
        |row| {
            Ok(DetailBranch {
                id: row.get(0)?,
                repo_id: row.get(1)?,
                name: row.get(2)?,
                base_branch: row.get(3)?,
                head_sha: row.get(4)?,
                intent: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        },
    )
    .optional()
    .map_err(sql_error("load task detail branch"))
}

pub(crate) fn select_detail_worktree(
    conn: &Connection,
    worktree_id: &str,
) -> Result<Option<DetailWorktree>, String> {
    conn.query_row(
        "
SELECT id, repo_id, branch_id, path, is_main, is_dirty, lock_reason, last_scanned_at,
  created_at, updated_at
FROM worktrees
WHERE id = ?1
",
        params![worktree_id],
        |row| {
            let is_main: i64 = row.get(4)?;
            let is_dirty: i64 = row.get(5)?;

            Ok(DetailWorktree {
                id: row.get(0)?,
                repo_id: row.get(1)?,
                branch_id: row.get(2)?,
                path: row.get(3)?,
                is_main: is_main == 1,
                is_dirty: is_dirty == 1,
                lock_reason: row.get(6)?,
                last_scanned_at: row.get(7)?,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
            })
        },
    )
    .optional()
    .map_err(sql_error("load task detail worktree"))
}

pub(crate) fn select_detail_task_runs(
    conn: &Connection,
    task_id: &str,
) -> Result<Vec<DetailTaskRun>, String> {
    let mut stmt = conn
        .prepare(
            "
SELECT id, task_id, conversation_id, worktree_id, execution_state, started_at, completed_at,
  exit_code, created_at, updated_at
FROM task_runs
WHERE task_id = ?1
ORDER BY created_at, id
",
        )
        .map_err(sql_error("prepare task detail runs query"))?;

    let rows = stmt
        .query_map(params![task_id], |row| {
            Ok(DetailTaskRun {
                id: row.get(0)?,
                task_id: row.get(1)?,
                conversation_id: row.get(2)?,
                worktree_id: row.get(3)?,
                execution_state: row.get(4)?,
                started_at: row.get(5)?,
                completed_at: row.get(6)?,
                exit_code: row.get(7)?,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
            })
        })
        .map_err(sql_error("query task detail run rows"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(sql_error("read task detail run rows"))?;

    Ok(rows)
}

pub(crate) fn select_detail_artifacts(
    conn: &Connection,
    task_id: &str,
) -> Result<Vec<DetailArtifact>, String> {
    let mut stmt = conn
        .prepare(
            "
SELECT id, task_id, task_run_id, conversation_id, kind, title, uri, content, created_at
FROM artifacts
WHERE task_id = ?1
ORDER BY created_at, id
",
        )
        .map_err(sql_error("prepare task detail artifacts query"))?;

    let rows = stmt
        .query_map(params![task_id], |row| {
            Ok(DetailArtifact {
                id: row.get(0)?,
                task_id: row.get(1)?,
                task_run_id: row.get(2)?,
                conversation_id: row.get(3)?,
                kind: row.get(4)?,
                title: row.get(5)?,
                uri: row.get(6)?,
                content: row.get(7)?,
                created_at: row.get(8)?,
            })
        })
        .map_err(sql_error("query task detail artifact rows"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(sql_error("read task detail artifact rows"))?;

    Ok(rows)
}

pub(crate) fn select_detail_validation_runs(
    conn: &Connection,
    task_id: &str,
) -> Result<Vec<DetailValidationRun>, String> {
    let mut stmt = conn
        .prepare(
            "
SELECT id, task_id, task_run_id, command, status, started_at, completed_at, exit_code,
  output_artifact_id, created_at, updated_at
FROM validation_runs
WHERE task_id = ?1
ORDER BY created_at, id
",
        )
        .map_err(sql_error("prepare task detail validation runs query"))?;

    let rows = stmt
        .query_map(params![task_id], |row| {
            Ok(DetailValidationRun {
                id: row.get(0)?,
                task_id: row.get(1)?,
                task_run_id: row.get(2)?,
                command: row.get(3)?,
                status: row.get(4)?,
                started_at: row.get(5)?,
                completed_at: row.get(6)?,
                exit_code: row.get(7)?,
                output_artifact_id: row.get(8)?,
                created_at: row.get(9)?,
                updated_at: row.get(10)?,
            })
        })
        .map_err(sql_error("query task detail validation run rows"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(sql_error("read task detail validation run rows"))?;

    Ok(rows)
}

pub(crate) fn select_detail_events(
    conn: &Connection,
    task_id: &str,
) -> Result<Vec<DetailEvent>, String> {
    let mut stmt = conn
        .prepare(
            "
SELECT id, kind, occurred_at, project_id, task_id, task_run_id, conversation_id, artifact_id,
  validation_run_id, payload_json
FROM events
WHERE task_id = ?1
ORDER BY occurred_at, id
",
        )
        .map_err(sql_error("prepare task detail events query"))?;

    let rows = stmt
        .query_map(params![task_id], |row| {
            let id: String = row.get(0)?;
            let payload_json: String = row.get(9)?;
            let payload = parse_event_payload(&id, &payload_json).map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(
                    9,
                    rusqlite::types::Type::Text,
                    Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, error)),
                )
            })?;

            Ok(DetailEvent {
                id,
                kind: row.get(1)?,
                occurred_at: row.get(2)?,
                project_id: row.get(3)?,
                task_id: row.get(4)?,
                task_run_id: row.get(5)?,
                conversation_id: row.get(6)?,
                artifact_id: row.get(7)?,
                validation_run_id: row.get(8)?,
                payload,
            })
        })
        .map_err(sql_error("query task detail event rows"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(sql_error("read task detail event rows"))?;

    Ok(rows)
}

pub(crate) fn map_task_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<TaskRow> {
    Ok(TaskRow {
        id: row.get(0)?,
        project_id: row.get(1)?,
        repo_id: row.get(2)?,
        branch_id: row.get(3)?,
        worktree_id: row.get(4)?,
        title: row.get(5)?,
        summary: row.get(6)?,
        execution_state: row.get(7)?,
        attention_state: row.get(8)?,
        priority: row.get(9)?,
        updated_at: row.get(10)?,
    })
}

pub(crate) fn next_task_conversation_position(
    conn: &Connection,
    task_id: &str,
) -> Result<i64, String> {
    conn.query_row(
        "
SELECT COALESCE(MAX(position) + 1, 0)
FROM task_conversation_links
WHERE task_id = ?1
",
        params![task_id],
        |row| row.get(0),
    )
    .map_err(sql_error("read next task conversation position"))
}
