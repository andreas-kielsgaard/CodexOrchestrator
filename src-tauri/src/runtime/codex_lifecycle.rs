use super::*;

pub(crate) fn start_codex_task_run_lifecycle(
    conn: &Connection,
    input: &StartCodexTaskRunCommandInput,
) -> Result<StartedCodexTaskRun, String> {
    let existing_task = select_detail_task(conn, &input.task_id)?
        .ok_or_else(|| task_detail_not_found(&input.task_id))?;
    let timestamp = now_iso();
    let task_run_id = Uuid::new_v4().to_string();
    let conversation_id = Uuid::new_v4().to_string();
    let conversation_title = input.conversation_title.as_deref().unwrap_or("Codex run");

    conn.execute(
        "
INSERT INTO task_runs (
  id, task_id, conversation_id, worktree_id, execution_state, started_at, completed_at,
  exit_code, created_at, updated_at
) VALUES (?1, ?2, NULL, ?3, 'running', ?4, NULL, NULL, ?4, ?4)
",
        params![
            task_run_id,
            existing_task.id,
            input.worktree_id.as_deref(),
            timestamp
        ],
    )
    .map_err(sql_error("create task run"))?;

    conn.execute(
        "
INSERT INTO conversations (
  id, task_id, task_run_id, provider, external_thread_id, title, summary, created_at, updated_at
) VALUES (?1, ?2, ?3, 'codex', NULL, ?4, ?5, ?6, ?6)
",
        params![
            conversation_id,
            existing_task.id,
            task_run_id,
            conversation_title,
            input.conversation_summary.as_deref(),
            timestamp
        ],
    )
    .map_err(sql_error("create Codex conversation"))?;

    conn.execute(
        "UPDATE task_runs SET conversation_id = ?1, updated_at = ?2 WHERE id = ?3",
        params![conversation_id, timestamp, task_run_id],
    )
    .map_err(sql_error("link task run conversation"))?;

    let position = next_task_conversation_position(conn, &existing_task.id)?;
    conn.execute(
        "
INSERT INTO task_conversation_links (task_id, conversation_id, position, created_at)
VALUES (?1, ?2, ?3, ?4)
",
        params![existing_task.id, conversation_id, position, timestamp],
    )
    .map_err(sql_error("link task conversation"))?;

    conn.execute(
        "
UPDATE tasks
SET execution_state = 'running', attention_state = 'waiting_on_agent', updated_at = ?1
WHERE id = ?2
",
        params![timestamp, existing_task.id],
    )
    .map_err(sql_error("mark task run running"))?;

    let mut payload = Map::new();
    insert_string(&mut payload, "taskId", &existing_task.id);
    insert_string(&mut payload, "taskRunId", &task_run_id);
    if let Some(worktree_id) = &input.worktree_id {
        insert_string(&mut payload, "worktreeId", worktree_id);
    }
    insert_string(&mut payload, "startedAt", &timestamp);
    insert_string(&mut payload, "conversationId", &conversation_id);
    create_event(
        conn,
        "run_started",
        &timestamp,
        Some(&existing_task.project_id),
        Some(&existing_task.id),
        Some(&task_run_id),
        Some(&conversation_id),
        None,
        None,
        payload,
    )?;

    Ok(StartedCodexTaskRun {
        task_id: existing_task.id,
        project_id: existing_task.project_id,
        task_run_id,
        conversation_id,
    })
}

pub(crate) fn finish_codex_task_run_from_process_result(
    conn: &Connection,
    input: &StartCodexTaskRunCommandInput,
    started: &StartedCodexTaskRun,
    process_result: CodexCommandRunResult,
    git_diff_runner: &impl GitDiffRunner,
    validation_runner: &impl ValidationCommandRunner,
) -> Result<StartCodexTaskRunCommandResult, String> {
    let stdout_jsonl_length = process_result.stdout.len();
    let exit_code = process_result.exit_code;
    let signal = process_result.signal.clone();
    let raw_event_stream_artifact_id =
        create_raw_event_stream_artifact(conn, started, &process_result.stdout)?;
    let runtime_result = match codex_runtime_result_from_process_result(process_result.clone()) {
        Ok(runtime_result) => runtime_result,
        Err(error) => {
            append_raw_event_stream_created_event(
                conn,
                started,
                &raw_event_stream_artifact_id,
                "error",
                stdout_jsonl_length,
                exit_code,
                signal.as_deref(),
                Some(&error),
            )?;

            let mut result = fail_started_codex_task_run(
                conn,
                started,
                Some(raw_event_stream_artifact_id),
                exit_code,
                Some("Codex JSONL parse failed".to_string()),
                error,
            )?;
            attach_skipped_post_run_capture_if_requested(input, &mut result);
            return Ok(result);
        }
    };

    append_raw_event_stream_created_event(
        conn,
        started,
        &raw_event_stream_artifact_id,
        runtime_result.status.as_str(),
        runtime_result.stdout_jsonl.len(),
        runtime_result.exit_code,
        runtime_result.signal.as_deref(),
        None,
    )?;
    update_conversation_from_runtime_result(conn, started, &runtime_result)?;

    if runtime_result.status == CodexRuntimeStatus::Completed {
        let mut result = complete_started_codex_task_run(
            conn,
            started,
            raw_event_stream_artifact_id,
            runtime_result,
        )?;
        attach_post_run_capture_if_requested(
            conn,
            input,
            started,
            git_diff_runner,
            validation_runner,
            &mut result,
        );
        Ok(result)
    } else {
        let error = codex_failure_reason(&runtime_result);
        let mut result = fail_started_codex_task_run(
            conn,
            started,
            Some(raw_event_stream_artifact_id),
            runtime_result.exit_code,
            Some(runtime_result.status_reason),
            error,
        )?;
        attach_skipped_post_run_capture_if_requested(input, &mut result);
        Ok(result)
    }
}

pub(crate) fn codex_runtime_result_from_process_result(
    process_result: CodexCommandRunResult,
) -> Result<CodexRuntimeResult, String> {
    let summary = parse_codex_jsonl_summary(&process_result.stdout)?;
    let classification = classify_codex_exec_result(&process_result, &summary);

    Ok(CodexRuntimeResult {
        exit_code: process_result.exit_code,
        signal: process_result.signal,
        status: classification.0,
        status_reason: classification.1,
        stdout_jsonl: process_result.stdout,
        stderr: process_result.stderr,
        summary,
    })
}

pub(crate) fn complete_started_codex_task_run(
    conn: &Connection,
    started: &StartedCodexTaskRun,
    raw_event_stream_artifact_id: String,
    runtime_result: CodexRuntimeResult,
) -> Result<StartCodexTaskRunCommandResult, String> {
    let completed_at = now_iso();

    conn.execute(
        "
UPDATE task_runs
SET execution_state = 'completed', completed_at = ?1, exit_code = ?2, updated_at = ?1
WHERE id = ?3
",
        params![completed_at, runtime_result.exit_code, started.task_run_id],
    )
    .map_err(sql_error("complete task run"))?;

    let final_response_artifact_id = match &runtime_result.summary.final_agent_message_text {
        Some(final_response) => Some(create_artifact(
            conn,
            Some(&started.task_id),
            Some(&started.task_run_id),
            Some(&started.conversation_id),
            "final_response",
            "Final Codex response",
            Some(final_response),
        )?),
        None => None,
    };

    conn.execute(
        "
UPDATE tasks
SET execution_state = 'completed', attention_state = 'needs_review', updated_at = ?1
WHERE id = ?2
",
        params![completed_at, started.task_id],
    )
    .map_err(sql_error("mark task run completed"))?;

    let mut payload = Map::new();
    insert_string(&mut payload, "outcome", "completed");
    insert_string(&mut payload, "taskId", &started.task_id);
    insert_string(&mut payload, "taskRunId", &started.task_run_id);
    insert_string(&mut payload, "completedAt", &completed_at);
    if let Some(exit_code) = runtime_result.exit_code {
        insert_i64(&mut payload, "exitCode", exit_code);
    }
    if let Some(artifact_id) = &final_response_artifact_id {
        insert_string(&mut payload, "artifactId", artifact_id);
    }
    create_event(
        conn,
        "run_completed",
        &completed_at,
        Some(&started.project_id),
        Some(&started.task_id),
        Some(&started.task_run_id),
        Some(&started.conversation_id),
        final_response_artifact_id.as_deref(),
        None,
        payload,
    )?;

    build_start_codex_task_run_result(
        conn,
        "completed",
        started,
        Some(raw_event_stream_artifact_id),
        final_response_artifact_id,
        runtime_result.exit_code,
        Some(runtime_result.status_reason),
        None,
    )
}

pub(crate) fn fail_started_codex_task_run(
    conn: &Connection,
    started: &StartedCodexTaskRun,
    raw_event_stream_artifact_id: Option<String>,
    exit_code: Option<i64>,
    status_reason: Option<String>,
    error: String,
) -> Result<StartCodexTaskRunCommandResult, String> {
    let completed_at = now_iso();

    conn.execute(
        "
UPDATE task_runs
SET execution_state = 'failed', completed_at = ?1, exit_code = ?2, updated_at = ?1
WHERE id = ?3
",
        params![completed_at, exit_code, started.task_run_id],
    )
    .map_err(sql_error("fail task run"))?;

    conn.execute(
        "
UPDATE tasks
SET execution_state = 'failed', attention_state = 'needs_action_now', updated_at = ?1
WHERE id = ?2
",
        params![completed_at, started.task_id],
    )
    .map_err(sql_error("mark task run failed"))?;

    let mut payload = Map::new();
    insert_string(&mut payload, "outcome", "failed");
    insert_string(&mut payload, "taskId", &started.task_id);
    insert_string(&mut payload, "taskRunId", &started.task_run_id);
    insert_string(&mut payload, "completedAt", &completed_at);
    if let Some(exit_code) = exit_code {
        insert_i64(&mut payload, "exitCode", exit_code);
    }
    insert_string(&mut payload, "error", &error);
    create_event(
        conn,
        "run_completed",
        &completed_at,
        Some(&started.project_id),
        Some(&started.task_id),
        Some(&started.task_run_id),
        Some(&started.conversation_id),
        None,
        None,
        payload,
    )?;

    build_start_codex_task_run_result(
        conn,
        "failed",
        started,
        raw_event_stream_artifact_id,
        None,
        exit_code,
        status_reason,
        Some(error),
    )
}

pub(crate) fn create_raw_event_stream_artifact(
    conn: &Connection,
    started: &StartedCodexTaskRun,
    stdout_jsonl: &str,
) -> Result<String, String> {
    create_artifact(
        conn,
        Some(&started.task_id),
        Some(&started.task_run_id),
        Some(&started.conversation_id),
        "raw_event_stream",
        "Raw Codex JSONL",
        Some(stdout_jsonl),
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn append_raw_event_stream_created_event(
    conn: &Connection,
    started: &StartedCodexTaskRun,
    artifact_id: &str,
    codex_status: &str,
    stdout_jsonl_length: usize,
    exit_code: Option<i64>,
    signal: Option<&str>,
    parse_error: Option<&str>,
) -> Result<String, String> {
    let occurred_at = now_iso();
    let mut payload = Map::new();
    insert_string(&mut payload, "artifactKind", "raw_event_stream");
    insert_string(&mut payload, "artifactId", artifact_id);
    insert_string(&mut payload, "codexStatus", codex_status);
    insert_i64(
        &mut payload,
        "stdoutJsonlLength",
        stdout_jsonl_length as i64,
    );
    if let Some(exit_code) = exit_code {
        insert_i64(&mut payload, "exitCode", exit_code);
    }
    if let Some(signal) = signal {
        insert_string(&mut payload, "signal", signal);
    }
    if let Some(parse_error) = parse_error {
        insert_string(&mut payload, "parseError", parse_error);
    }

    create_event(
        conn,
        "artifact_created",
        &occurred_at,
        Some(&started.project_id),
        Some(&started.task_id),
        Some(&started.task_run_id),
        Some(&started.conversation_id),
        Some(artifact_id),
        None,
        payload,
    )
}
