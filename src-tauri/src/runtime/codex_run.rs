use super::*;

#[cfg(test)]
pub(crate) fn start_codex_task_run_with_runner(
    conn: &Connection,
    input: StartCodexTaskRunCommandInput,
    runner: &impl CodexCommandRunner,
) -> Result<StartCodexTaskRunCommandResult, String> {
    start_codex_task_run_with_runners(
        conn,
        input,
        runner,
        &SystemGitDiffRunner,
        &SystemValidationCommandRunner,
    )
}

pub(crate) fn start_codex_task_run_with_runners(
    conn: &Connection,
    input: StartCodexTaskRunCommandInput,
    codex_runner: &impl CodexCommandRunner,
    git_diff_runner: &impl GitDiffRunner,
    validation_runner: &impl ValidationCommandRunner,
) -> Result<StartCodexTaskRunCommandResult, String> {
    validate_start_codex_task_run_input(&input)?;
    let started = start_codex_task_run_lifecycle(conn, &input)?;
    let command_input = CodexCommandRunInput {
        command: "codex".to_string(),
        args: build_codex_exec_args(&input),
        cwd: input.cwd.clone(),
        env: input.env.clone(),
    };

    match codex_runner.run(command_input) {
        Ok(process_result) => finish_codex_task_run_from_process_result(
            conn,
            &input,
            &started,
            process_result,
            git_diff_runner,
            validation_runner,
        ),
        Err(error) => {
            let mut result = fail_started_codex_task_run(
                conn,
                &started,
                None,
                None,
                Some(error.clone()),
                error,
            )?;
            attach_skipped_post_run_capture_if_requested(&input, &mut result);
            Ok(result)
        }
    }
}

pub(crate) fn validate_start_codex_task_run_input(
    input: &StartCodexTaskRunCommandInput,
) -> Result<(), String> {
    validate_non_empty("taskId", &input.task_id)?;
    validate_non_empty("prompt", &input.prompt)?;

    if let Some(cwd) = &input.cwd {
        validate_non_empty("cwd", cwd)?;
    }

    if let Some(worktree_id) = &input.worktree_id {
        validate_non_empty("worktreeId", worktree_id)?;
    }

    if let Some(additional_args) = &input.additional_args {
        for (index, arg) in additional_args.iter().enumerate() {
            validate_non_empty(&format!("additionalArgs[{index}]"), arg)?;
        }
    }

    if let Some(env) = &input.env {
        for key in env.keys() {
            validate_non_empty("env key", key)?;
        }
    }

    if let Some(post_run_capture) = &input.post_run_capture {
        if let Some(validation_command) = &post_run_capture.validation_command {
            validate_non_empty(
                "postRunCapture.validationCommand.command",
                &validation_command.command,
            )?;

            if let Some(args) = &validation_command.args {
                for (index, arg) in args.iter().enumerate() {
                    validate_non_empty(
                        &format!("postRunCapture.validationCommand.args[{index}]"),
                        arg,
                    )?;
                }
            }

            if let Some(cwd) = &validation_command.cwd {
                validate_non_empty("postRunCapture.validationCommand.cwd", cwd)?;
            }

            if let Some(env) = &validation_command.env {
                for key in env.keys() {
                    validate_non_empty("postRunCapture.validationCommand.env key", key)?;
                }
            }
        }
    }

    Ok(())
}

pub(crate) fn build_codex_exec_args(input: &StartCodexTaskRunCommandInput) -> Vec<String> {
    let mut args = vec!["exec".to_string(), "--json".to_string()];

    if let Some(additional_args) = &input.additional_args {
        args.extend(additional_args.iter().cloned());
    }

    args.push(input.prompt.clone());
    args
}

pub(crate) fn build_start_codex_task_run_result(
    conn: &Connection,
    status: &str,
    started: &StartedCodexTaskRun,
    raw_event_stream_artifact_id: Option<String>,
    final_response_artifact_id: Option<String>,
    exit_code: Option<i64>,
    status_reason: Option<String>,
    error: Option<String>,
) -> Result<StartCodexTaskRunCommandResult, String> {
    let task = select_detail_task(conn, &started.task_id)?
        .ok_or_else(|| task_detail_not_found(&started.task_id))?;
    let task_run = select_detail_task_runs(conn, &started.task_id)?
        .into_iter()
        .find(|task_run| task_run.id == started.task_run_id)
        .ok_or_else(|| format!("Task run not found: {}", started.task_run_id))?;

    Ok(StartCodexTaskRunCommandResult {
        status: status.to_string(),
        task_id: started.task_id.clone(),
        task_run_id: started.task_run_id.clone(),
        conversation_id: Some(started.conversation_id.clone()),
        raw_event_stream_artifact_id,
        final_response_artifact_id,
        exit_code,
        status_reason,
        error,
        post_run_capture: None,
        task: start_codex_task_run_task_state(task),
        task_run: start_codex_task_run_task_run_state(task_run),
    })
}

pub(crate) fn start_codex_task_run_task_state(task: DetailTask) -> StartCodexTaskRunTaskState {
    StartCodexTaskRunTaskState {
        id: task.id,
        execution_state: task.execution_state,
        attention_state: task.attention_state,
        conversation_ids: task.conversation_ids,
        repo_id: task.repo_id,
        branch_id: task.branch_id,
        worktree_id: task.worktree_id,
        updated_at: task.updated_at,
    }
}

pub(crate) fn start_codex_task_run_task_run_state(
    task_run: DetailTaskRun,
) -> StartCodexTaskRunTaskRunState {
    StartCodexTaskRunTaskRunState {
        id: task_run.id,
        execution_state: task_run.execution_state,
        conversation_id: task_run.conversation_id,
        worktree_id: task_run.worktree_id,
        started_at: task_run.started_at,
        completed_at: task_run.completed_at,
        exit_code: task_run.exit_code,
        updated_at: task_run.updated_at,
    }
}

impl CodexRuntimeStatus {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            CodexRuntimeStatus::Completed => "completed",
            CodexRuntimeStatus::Failed => "failed",
            CodexRuntimeStatus::Error => "error",
        }
    }
}
