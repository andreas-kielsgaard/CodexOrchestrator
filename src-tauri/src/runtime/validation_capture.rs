use super::*;

pub(crate) fn run_post_run_validation_command(
    conn: &Connection,
    input: &StartCodexTaskRunCommandInput,
    started: &StartedCodexTaskRun,
    validation_command: &StartCodexTaskRunValidationCommandInput,
    validation_runner: &impl ValidationCommandRunner,
) -> Result<StartCodexTaskRunValidationCaptureResult, String> {
    let resolved = resolve_post_run_validation_cwd(conn, input, started, validation_command)?;
    let args = validation_command.args.clone().unwrap_or_default();
    let display_command = render_validation_command(&validation_command.command, &args);
    let started_at = now_iso();
    let validation_run_id = Uuid::new_v4().to_string();

    conn.execute(
        "
INSERT INTO validation_runs (
  id, task_id, task_run_id, command, status, started_at, completed_at, exit_code,
  output_artifact_id, created_at, updated_at
) VALUES (?1, ?2, ?3, ?4, 'running', ?5, NULL, NULL, NULL, ?5, ?5)
",
        params![
            validation_run_id,
            started.task_id,
            started.task_run_id,
            display_command,
            started_at
        ],
    )
    .map_err(sql_error("create validation run"))?;

    let started_event_id = append_validation_started_event(
        conn,
        started,
        &validation_run_id,
        validation_command,
        &args,
        &resolved,
        &started_at,
    )?;

    let (runtime_result, runtime_error) = match validation_runner.run(ValidationCommandRunInput {
        command: validation_command.command.clone(),
        args: args.clone(),
        cwd: resolved.path.clone(),
        env: validation_command.env.clone(),
    }) {
        Ok(result) => (Some(result), None),
        Err(error) => (None, Some(error)),
    };

    finish_post_run_validation_command(
        conn,
        started,
        validation_command,
        &args,
        &display_command,
        &validation_run_id,
        &started_event_id,
        &resolved,
        &started_at,
        runtime_result,
        runtime_error,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn finish_post_run_validation_command(
    conn: &Connection,
    started: &StartedCodexTaskRun,
    validation_command: &StartCodexTaskRunValidationCommandInput,
    args: &[String],
    display_command: &str,
    validation_run_id: &str,
    started_event_id: &str,
    resolved: &ResolvedPostRunWorktreePath,
    started_at: &str,
    runtime_result: Option<ValidationCommandRunResult>,
    runtime_error: Option<String>,
) -> Result<StartCodexTaskRunValidationCaptureResult, String> {
    let status = classify_validation_command_status(runtime_result.as_ref());
    let completed_at = now_iso();
    let output_content = create_validation_log_content(
        started,
        validation_run_id,
        status,
        validation_command,
        args,
        resolved,
        started_at,
        &completed_at,
        runtime_result.as_ref(),
        runtime_error.as_deref(),
    )?;
    let output_artifact_id = create_artifact(
        conn,
        Some(&started.task_id),
        Some(&started.task_run_id),
        None,
        "validation_log",
        &format!("Validation log: {display_command}"),
        Some(&output_content),
    )?;
    let artifact_created_event_id = append_validation_artifact_created_event(
        conn,
        started,
        validation_run_id,
        &output_artifact_id,
        status,
        runtime_result.as_ref(),
        runtime_error.as_deref(),
    )?;

    conn.execute(
        "
UPDATE validation_runs
SET status = ?1, completed_at = ?2, exit_code = ?3, output_artifact_id = ?4, updated_at = ?2
WHERE id = ?5
",
        params![
            status,
            completed_at,
            runtime_result.as_ref().and_then(|result| result.exit_code),
            output_artifact_id,
            validation_run_id
        ],
    )
    .map_err(sql_error("complete validation run"))?;

    let completed_event_id = append_validation_completed_event(
        conn,
        started,
        validation_run_id,
        &output_artifact_id,
        status,
        &completed_at,
        runtime_result.as_ref(),
        runtime_error.as_deref(),
    )?;

    Ok(StartCodexTaskRunValidationCaptureResult {
        status: status.to_string(),
        validation_run_id: Some(validation_run_id.to_string()),
        output_artifact_id: Some(output_artifact_id),
        started_event_id: Some(started_event_id.to_string()),
        artifact_created_event_id: Some(artifact_created_event_id),
        completed_event_id: Some(completed_event_id),
        exit_code: runtime_result.as_ref().and_then(|result| result.exit_code),
        signal: runtime_result
            .as_ref()
            .and_then(|result| result.signal.clone()),
        error: runtime_error,
    })
}

pub(crate) fn resolve_post_run_validation_cwd(
    conn: &Connection,
    input: &StartCodexTaskRunCommandInput,
    started: &StartedCodexTaskRun,
    validation_command: &StartCodexTaskRunValidationCommandInput,
) -> Result<ResolvedPostRunWorktreePath, String> {
    if let Some(cwd) = &validation_command.cwd {
        return Ok(ResolvedPostRunWorktreePath {
            path: cwd.clone(),
            worktree_id: input.worktree_id.clone(),
        });
    }

    resolve_post_run_worktree_path(
        conn,
        started,
        input.cwd.as_deref(),
        input.worktree_id.as_deref(),
    )
}

pub(crate) fn resolve_post_run_worktree_path(
    conn: &Connection,
    started: &StartedCodexTaskRun,
    explicit_cwd: Option<&str>,
    input_worktree_id: Option<&str>,
) -> Result<ResolvedPostRunWorktreePath, String> {
    if let Some(cwd) = explicit_cwd {
        return Ok(ResolvedPostRunWorktreePath {
            path: cwd.to_string(),
            worktree_id: input_worktree_id.map(ToString::to_string),
        });
    }

    let worktree_id = match input_worktree_id {
        Some(worktree_id) => Some(worktree_id.to_string()),
        None => select_task_run_worktree_id(conn, &started.task_run_id)?
            .or(select_task_worktree_id(conn, &started.task_id)?),
    }
    .ok_or_else(|| {
        format!(
            "Post-run capture could not resolve a cwd or linked worktree path for task: {}",
            started.task_id
        )
    })?;

    let path = select_worktree_path(conn, &worktree_id)?.ok_or_else(|| {
        format!(
            "Post-run capture worktree not found for task {}: {}",
            started.task_id, worktree_id
        )
    })?;

    Ok(ResolvedPostRunWorktreePath {
        path,
        worktree_id: Some(worktree_id),
    })
}

pub(crate) fn select_task_run_worktree_id(
    conn: &Connection,
    task_run_id: &str,
) -> Result<Option<String>, String> {
    conn.query_row(
        "SELECT worktree_id FROM task_runs WHERE id = ?1",
        params![task_run_id],
        |row| row.get(0),
    )
    .optional()
    .map_err(sql_error("read task run worktree id"))
    .map(Option::flatten)
}

pub(crate) fn select_task_worktree_id(
    conn: &Connection,
    task_id: &str,
) -> Result<Option<String>, String> {
    conn.query_row(
        "SELECT worktree_id FROM tasks WHERE id = ?1",
        params![task_id],
        |row| row.get(0),
    )
    .optional()
    .map_err(sql_error("read task worktree id"))
    .map(Option::flatten)
}

pub(crate) fn select_worktree_path(
    conn: &Connection,
    worktree_id: &str,
) -> Result<Option<String>, String> {
    conn.query_row(
        "SELECT path FROM worktrees WHERE id = ?1",
        params![worktree_id],
        |row| row.get(0),
    )
    .optional()
    .map_err(sql_error("read worktree path"))
}

pub(crate) fn append_validation_started_event(
    conn: &Connection,
    started: &StartedCodexTaskRun,
    validation_run_id: &str,
    validation_command: &StartCodexTaskRunValidationCommandInput,
    args: &[String],
    resolved: &ResolvedPostRunWorktreePath,
    started_at: &str,
) -> Result<String, String> {
    let mut payload = Map::new();
    insert_string(&mut payload, "taskId", &started.task_id);
    insert_string(&mut payload, "validationRunId", validation_run_id);
    insert_string(&mut payload, "command", &validation_command.command);
    insert_string_array(&mut payload, "args", args);
    insert_string(&mut payload, "cwd", &resolved.path);
    if let Some(worktree_id) = &resolved.worktree_id {
        insert_string(&mut payload, "worktreeId", worktree_id);
    }
    insert_string(&mut payload, "startedAt", started_at);

    create_event(
        conn,
        "validation_started",
        started_at,
        Some(&started.project_id),
        Some(&started.task_id),
        Some(&started.task_run_id),
        None,
        None,
        Some(validation_run_id),
        payload,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn create_validation_log_content(
    started: &StartedCodexTaskRun,
    validation_run_id: &str,
    status: &str,
    validation_command: &StartCodexTaskRunValidationCommandInput,
    args: &[String],
    resolved: &ResolvedPostRunWorktreePath,
    started_at: &str,
    completed_at: &str,
    runtime_result: Option<&ValidationCommandRunResult>,
    runtime_error: Option<&str>,
) -> Result<String, String> {
    let mut payload = Map::new();
    insert_string(&mut payload, "taskId", &started.task_id);
    insert_string(&mut payload, "validationRunId", validation_run_id);
    insert_string(&mut payload, "status", status);
    insert_string(&mut payload, "command", &validation_command.command);
    insert_string_array(&mut payload, "args", args);
    insert_string(&mut payload, "cwd", &resolved.path);
    if let Some(worktree_id) = &resolved.worktree_id {
        insert_string(&mut payload, "worktreeId", worktree_id);
    }
    insert_string(&mut payload, "startedAt", started_at);
    insert_string(&mut payload, "completedAt", completed_at);
    payload.insert(
        "process".to_string(),
        Value::Object(validation_process_payload(runtime_result, runtime_error)),
    );

    serde_json::to_string_pretty(&Value::Object(payload))
        .map_err(|error| format!("Unable to encode validation log artifact: {error}"))
}

pub(crate) fn validation_process_payload(
    runtime_result: Option<&ValidationCommandRunResult>,
    runtime_error: Option<&str>,
) -> Map<String, Value> {
    let mut process = Map::new();

    match runtime_result {
        Some(result) => {
            insert_string(&mut process, "stdout", &result.stdout);
            insert_string(&mut process, "stderr", &result.stderr);
            insert_nullable_i64(&mut process, "exitCode", result.exit_code);
            insert_nullable_string(&mut process, "signal", result.signal.as_deref());
        }
        None => {
            insert_string(&mut process, "stdout", "");
            insert_string(&mut process, "stderr", "");
            insert_nullable_i64(&mut process, "exitCode", None);
            insert_nullable_string(&mut process, "signal", None);
            insert_string(
                &mut process,
                "error",
                runtime_error.unwrap_or("Validation command did not return a process result"),
            );
        }
    }

    process
}

pub(crate) fn append_validation_artifact_created_event(
    conn: &Connection,
    started: &StartedCodexTaskRun,
    validation_run_id: &str,
    output_artifact_id: &str,
    status: &str,
    runtime_result: Option<&ValidationCommandRunResult>,
    runtime_error: Option<&str>,
) -> Result<String, String> {
    let occurred_at = now_iso();
    let mut payload = Map::new();
    insert_string(&mut payload, "artifactKind", "validation_log");
    insert_string(&mut payload, "artifactId", output_artifact_id);
    insert_string(&mut payload, "validationRunId", validation_run_id);
    insert_string(&mut payload, "validationStatus", status);
    if let Some(result) = runtime_result {
        insert_i64(&mut payload, "stdoutLength", result.stdout.len() as i64);
        insert_i64(&mut payload, "stderrLength", result.stderr.len() as i64);
        if let Some(exit_code) = result.exit_code {
            insert_i64(&mut payload, "exitCode", exit_code);
        }
        if let Some(signal) = &result.signal {
            insert_string(&mut payload, "signal", signal);
        }
    }
    if let Some(error) = runtime_error {
        insert_string(&mut payload, "error", error);
    }

    create_event(
        conn,
        "artifact_created",
        &occurred_at,
        Some(&started.project_id),
        Some(&started.task_id),
        Some(&started.task_run_id),
        None,
        Some(output_artifact_id),
        Some(validation_run_id),
        payload,
    )
}

pub(crate) fn append_validation_completed_event(
    conn: &Connection,
    started: &StartedCodexTaskRun,
    validation_run_id: &str,
    output_artifact_id: &str,
    status: &str,
    completed_at: &str,
    runtime_result: Option<&ValidationCommandRunResult>,
    runtime_error: Option<&str>,
) -> Result<String, String> {
    let mut payload = Map::new();
    insert_string(&mut payload, "outcome", status);
    insert_string(&mut payload, "taskId", &started.task_id);
    insert_string(&mut payload, "validationRunId", validation_run_id);
    insert_string(&mut payload, "artifactId", output_artifact_id);
    insert_string(&mut payload, "completedAt", completed_at);
    if let Some(result) = runtime_result {
        if let Some(exit_code) = result.exit_code {
            insert_i64(&mut payload, "exitCode", exit_code);
        }
        if let Some(signal) = &result.signal {
            insert_string(&mut payload, "signal", signal);
        }
    }
    if let Some(error) = runtime_error {
        insert_string(&mut payload, "error", error);
    }

    create_event(
        conn,
        "validation_completed",
        completed_at,
        Some(&started.project_id),
        Some(&started.task_id),
        Some(&started.task_run_id),
        None,
        Some(output_artifact_id),
        Some(validation_run_id),
        payload,
    )
}

pub(crate) fn classify_validation_command_status(
    result: Option<&ValidationCommandRunResult>,
) -> &'static str {
    match result {
        Some(result) if result.exit_code == Some(0) && result.signal.is_none() => "passed",
        _ => "failed",
    }
}

pub(crate) fn render_validation_command(command: &str, args: &[String]) -> String {
    std::iter::once(command.to_string())
        .chain(args.iter().map(|arg| render_validation_command_arg(arg)))
        .collect::<Vec<_>>()
        .join(" ")
}

pub(crate) fn render_validation_command_arg(arg: &str) -> String {
    if arg.chars().all(is_plain_validation_command_arg_char) {
        return arg.to_string();
    }

    serde_json::to_string(arg).unwrap_or_else(|_| arg.to_string())
}

pub(crate) fn is_plain_validation_command_arg_char(character: char) -> bool {
    character.is_ascii_alphanumeric() || "_./:=@+-".contains(character)
}
