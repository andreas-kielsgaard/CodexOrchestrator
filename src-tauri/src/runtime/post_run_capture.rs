use super::*;

pub(crate) fn attach_skipped_post_run_capture_if_requested(
    input: &StartCodexTaskRunCommandInput,
    result: &mut StartCodexTaskRunCommandResult,
) {
    if input.post_run_capture.is_some() {
        result.post_run_capture = Some(StartCodexTaskRunPostRunCaptureResult {
            skipped_reason: Some("run_failed".to_string()),
            ..StartCodexTaskRunPostRunCaptureResult::default()
        });
    }
}

pub(crate) fn attach_post_run_capture_if_requested(
    conn: &Connection,
    input: &StartCodexTaskRunCommandInput,
    started: &StartedCodexTaskRun,
    git_diff_runner: &impl GitDiffRunner,
    validation_runner: &impl ValidationCommandRunner,
    result: &mut StartCodexTaskRunCommandResult,
) {
    let Some(options) = &input.post_run_capture else {
        return;
    };

    result.post_run_capture = Some(run_post_run_capture(
        conn,
        input,
        started,
        options,
        git_diff_runner,
        validation_runner,
    ));
}

pub(crate) fn run_post_run_capture(
    conn: &Connection,
    input: &StartCodexTaskRunCommandInput,
    started: &StartedCodexTaskRun,
    options: &StartCodexTaskRunPostRunCaptureInput,
    git_diff_runner: &impl GitDiffRunner,
    validation_runner: &impl ValidationCommandRunner,
) -> StartCodexTaskRunPostRunCaptureResult {
    let diff = if options.collect_diff.unwrap_or(false) {
        Some(
            match collect_post_run_diff(conn, input, started, git_diff_runner) {
                Ok(result) => result,
                Err(error) => StartCodexTaskRunDiffCaptureResult::Failed { error },
            },
        )
    } else {
        None
    };

    let validation = options
        .validation_command
        .as_ref()
        .map(|validation_command| {
            match run_post_run_validation_command(
                conn,
                input,
                started,
                validation_command,
                validation_runner,
            ) {
                Ok(result) => result,
                Err(error) => StartCodexTaskRunValidationCaptureResult {
                    status: "failed".to_string(),
                    validation_run_id: None,
                    output_artifact_id: None,
                    started_event_id: None,
                    artifact_created_event_id: None,
                    completed_event_id: None,
                    exit_code: None,
                    signal: None,
                    error: Some(error),
                },
            }
        });

    StartCodexTaskRunPostRunCaptureResult {
        diff,
        validation,
        skipped_reason: None,
    }
}

pub(crate) fn collect_post_run_diff(
    conn: &Connection,
    input: &StartCodexTaskRunCommandInput,
    started: &StartedCodexTaskRun,
    git_diff_runner: &impl GitDiffRunner,
) -> Result<StartCodexTaskRunDiffCaptureResult, String> {
    let resolved = resolve_post_run_worktree_path(
        conn,
        started,
        input.cwd.as_deref(),
        input.worktree_id.as_deref(),
    )?;
    let diff_result = git_diff_runner.collect_tracked_diff(GitDiffRunInput {
        worktree_path: resolved.path.clone(),
    })?;
    let is_empty_diff = diff_result.diff.is_empty();
    let artifact_id = create_artifact(
        conn,
        Some(&started.task_id),
        Some(&started.task_run_id),
        None,
        "diff",
        "Post-run diff",
        Some(&diff_result.diff),
    )?;
    let occurred_at = now_iso();
    let mut payload = Map::new();
    insert_string(&mut payload, "artifactKind", "diff");
    insert_string(&mut payload, "artifactId", &artifact_id);
    insert_i64(&mut payload, "diffLength", diff_result.diff.len() as i64);
    insert_bool(&mut payload, "isEmptyDiff", is_empty_diff);
    insert_string(&mut payload, "worktreePath", &resolved.path);
    if let Some(worktree_id) = &resolved.worktree_id {
        insert_string(&mut payload, "worktreeId", worktree_id);
    }
    let event_id = create_event(
        conn,
        "artifact_created",
        &occurred_at,
        Some(&started.project_id),
        Some(&started.task_id),
        Some(&started.task_run_id),
        None,
        Some(&artifact_id),
        None,
        payload,
    )?;

    Ok(StartCodexTaskRunDiffCaptureResult::Captured {
        artifact_id,
        event_id,
        diff_length: diff_result.diff.len() as i64,
        is_empty_diff,
        worktree_path: resolved.path,
    })
}
