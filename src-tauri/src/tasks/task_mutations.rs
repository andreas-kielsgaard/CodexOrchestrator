use super::*;

pub(crate) fn create_task(
    conn: &Connection,
    input: CreateOpenTaskCommandInput,
) -> Result<(), String> {
    validate_non_empty("projectId", &input.project_id)?;
    validate_non_empty("title", &input.title)?;
    validate_non_empty("summary", &input.summary)?;

    let execution_state = validate_value(
        "executionState",
        input.execution_state.as_deref().unwrap_or("draft"),
        &EXECUTION_STATES,
    )?;
    let attention_state = validate_value(
        "attentionState",
        input
            .attention_state
            .as_deref()
            .unwrap_or("needs_action_now"),
        &ATTENTION_STATES,
    )?;
    let priority = validate_value(
        "priority",
        input.priority.as_deref().unwrap_or("normal"),
        &PRIORITIES,
    )?;
    let timestamp = now_iso();
    let task_id = Uuid::new_v4().to_string();
    let (repo_id, branch_id, worktree_id) = resolve_create_task_anchor(
        conn,
        &input.project_id,
        input.repo_id,
        input.branch_id,
        input.worktree_id,
    )?;

    conn.execute(
        "
INSERT INTO tasks (
  id, project_id, repo_id, branch_id, worktree_id, title, summary, execution_state,
  attention_state, priority, due_at, snoozed_until, created_at, updated_at
) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, NULL, NULL, ?11, ?11)
",
        params![
            task_id,
            input.project_id,
            repo_id,
            branch_id,
            worktree_id,
            input.title,
            input.summary,
            execution_state,
            attention_state,
            priority,
            timestamp
        ],
    )
    .map_err(sql_error("create open task"))?;

    Ok(())
}

pub(crate) fn update_task(
    conn: &Connection,
    task_id: &str,
    input: UpdateOpenTaskCommandInput,
) -> Result<(), String> {
    validate_non_empty("taskId", task_id)?;
    let existing = select_task(conn, task_id)?.ok_or_else(|| open_task_not_found(task_id))?;

    let title = validate_optional_non_empty("title", input.title)?.unwrap_or(existing.title);
    let summary =
        validate_optional_non_empty("summary", input.summary)?.unwrap_or(existing.summary);
    let execution_state = match input.execution_state {
        Some(value) => validate_value("executionState", &value, &EXECUTION_STATES)?.to_string(),
        None => existing.execution_state,
    };
    let attention_state = match input.attention_state {
        Some(value) => validate_value("attentionState", &value, &ATTENTION_STATES)?.to_string(),
        None => existing.attention_state,
    };
    let priority = match input.priority {
        Some(value) => validate_value("priority", &value, &PRIORITIES)?.to_string(),
        None => existing.priority,
    };

    conn.execute(
        "
UPDATE tasks SET
  title = ?1,
  summary = ?2,
  execution_state = ?3,
  attention_state = ?4,
  priority = ?5,
  updated_at = ?6
WHERE id = ?7
",
        params![
            title,
            summary,
            execution_state,
            attention_state,
            priority,
            now_iso(),
            task_id
        ],
    )
    .map_err(sql_error("update open task"))?;

    Ok(())
}

pub(crate) fn archive_task(conn: &Connection, task_id: &str) -> Result<(), String> {
    validate_non_empty("taskId", task_id)?;
    let changed = conn
        .execute(
            "UPDATE tasks SET execution_state = 'archived', updated_at = ?1 WHERE id = ?2",
            params![now_iso(), task_id],
        )
        .map_err(sql_error("archive open task"))?;

    if changed == 0 {
        return Err(open_task_not_found(task_id));
    }

    Ok(())
}
