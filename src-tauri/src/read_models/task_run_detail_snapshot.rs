use super::*;

pub(crate) fn load_task_run_detail_snapshot(
    conn: &Connection,
    task_id: &str,
) -> Result<TaskRunDetailSnapshot, String> {
    validate_non_empty("taskId", task_id)?;
    let task = select_detail_task(conn, task_id)?.ok_or_else(|| task_detail_not_found(task_id))?;
    let task_runs = select_detail_task_runs(conn, task_id)?;
    let artifacts = select_detail_artifacts(conn, task_id)?;
    let events = select_detail_events(conn, task_id)?;
    let validation_runs = select_detail_validation_runs(conn, task_id)?;

    let run_ids = task_runs
        .iter()
        .map(|run| run.id.clone())
        .collect::<HashSet<_>>();
    let validation_output_artifact_ids = validation_runs
        .iter()
        .filter_map(|run| run.output_artifact_id.clone())
        .collect::<HashSet<_>>();

    let mut runs_for_review = task_runs.clone();
    runs_for_review.sort_by(compare_runs_for_review);

    let runs = runs_for_review
        .into_iter()
        .map(|run| {
            let mut run_artifacts = artifacts
                .iter()
                .filter(|artifact| artifact.task_run_id.as_deref() == Some(run.id.as_str()))
                .cloned()
                .collect::<Vec<_>>();
            let run_validation_runs = validation_runs
                .iter()
                .filter(|validation_run| {
                    validation_run_belongs_to_run(validation_run, &run.id, &artifacts)
                })
                .cloned()
                .collect::<Vec<_>>();
            let run_validation_artifact_ids = run_validation_runs
                .iter()
                .filter_map(|validation_run| validation_run.output_artifact_id.clone())
                .collect::<HashSet<_>>();
            let mut run_artifact_ids = run_artifacts
                .iter()
                .map(|artifact| artifact.id.clone())
                .collect::<HashSet<_>>();

            let additional_validation_artifacts = artifacts
                .iter()
                .filter(|artifact| {
                    artifact.task_run_id.is_none()
                        && run_validation_artifact_ids.contains(&artifact.id)
                        && !run_artifact_ids.contains(&artifact.id)
                })
                .cloned()
                .collect::<Vec<_>>();

            for artifact in additional_validation_artifacts {
                run_artifact_ids.insert(artifact.id.clone());
                run_artifacts.push(artifact);
            }

            let mut run_events = events
                .iter()
                .filter(|event| event.task_run_id.as_deref() == Some(run.id.as_str()))
                .cloned()
                .collect::<Vec<_>>();
            run_events.sort_by(compare_events_chronologically);

            TaskRunDetailRun {
                run,
                artifacts: group_artifacts(run_artifacts),
                validation_runs: run_validation_runs
                    .iter()
                    .map(|validation_run| detail_validation_run(validation_run, &artifacts))
                    .collect(),
                events: run_events,
            }
        })
        .collect::<Vec<_>>();

    let unlinked_artifacts = group_artifacts(
        artifacts
            .iter()
            .filter(|artifact| {
                artifact.task_run_id.is_none()
                    && !validation_output_artifact_ids.contains(&artifact.id)
            })
            .cloned()
            .collect(),
    );
    let mut unlinked_validation_runs = validation_runs
        .iter()
        .filter(|validation_run| {
            !validation_run_belongs_to_any_run(validation_run, &run_ids, &artifacts)
        })
        .cloned()
        .collect::<Vec<_>>();
    unlinked_validation_runs.sort_by(compare_validation_runs_for_review);

    let mut event_timeline = events;
    event_timeline.sort_by(compare_events_chronologically);

    Ok(TaskRunDetailSnapshot {
        task: TaskRunDetailTaskAnchor {
            project: select_detail_project(conn, &task.project_id)?,
            repo: match &task.repo_id {
                Some(repo_id) => select_detail_repo(conn, repo_id)?,
                None => None,
            },
            branch: match &task.branch_id {
                Some(branch_id) => select_detail_branch(conn, branch_id)?,
                None => None,
            },
            worktree: match &task.worktree_id {
                Some(worktree_id) => select_detail_worktree(conn, worktree_id)?,
                None => None,
            },
            record: task,
        },
        runs,
        unlinked_artifacts,
        unlinked_validation_runs: unlinked_validation_runs
            .iter()
            .map(|validation_run| detail_validation_run(validation_run, &artifacts))
            .collect(),
        event_timeline,
    })
}

pub(crate) fn validation_run_belongs_to_run(
    validation_run: &DetailValidationRun,
    task_run_id: &str,
    artifacts: &[DetailArtifact],
) -> bool {
    if validation_run.task_run_id.as_deref() == Some(task_run_id) {
        return true;
    }

    let Some(output_artifact_id) = &validation_run.output_artifact_id else {
        return false;
    };

    artifacts.iter().any(|artifact| {
        artifact.id == *output_artifact_id && artifact.task_run_id.as_deref() == Some(task_run_id)
    })
}

pub(crate) fn validation_run_belongs_to_any_run(
    validation_run: &DetailValidationRun,
    run_ids: &HashSet<String>,
    artifacts: &[DetailArtifact],
) -> bool {
    if validation_run
        .task_run_id
        .as_ref()
        .is_some_and(|task_run_id| run_ids.contains(task_run_id))
    {
        return true;
    }

    let Some(output_artifact_id) = &validation_run.output_artifact_id else {
        return false;
    };

    artifacts.iter().any(|artifact| {
        artifact.id == *output_artifact_id
            && artifact
                .task_run_id
                .as_ref()
                .is_some_and(|task_run_id| run_ids.contains(task_run_id))
    })
}

pub(crate) fn detail_validation_run(
    validation_run: &DetailValidationRun,
    artifacts: &[DetailArtifact],
) -> TaskRunDetailValidationRun {
    TaskRunDetailValidationRun {
        run: validation_run.clone(),
        output_artifact: validation_run
            .output_artifact_id
            .as_ref()
            .and_then(|artifact_id| {
                artifacts
                    .iter()
                    .find(|artifact| artifact.id == *artifact_id)
                    .cloned()
            }),
    }
}
