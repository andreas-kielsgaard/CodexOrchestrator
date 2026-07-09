use super::*;

pub(crate) fn group_artifacts(mut artifacts: Vec<DetailArtifact>) -> TaskRunDetailArtifactGroups {
    artifacts.sort_by(compare_artifacts_chronologically);
    let mut groups = TaskRunDetailArtifactGroups::default();

    for artifact in artifacts {
        match artifact.kind.as_str() {
            "final_response" => groups.final_responses.push(artifact),
            "raw_event_stream" => groups.raw_event_streams.push(artifact),
            "diff" => groups.diffs.push(artifact),
            "validation_log" => groups.validation_logs.push(artifact),
            "note" => groups.notes.push(artifact),
            "screenshot" => groups.screenshots.push(artifact),
            "handoff" => groups.handoffs.push(artifact),
            "summary" => groups.summaries.push(artifact),
            _ => groups.other.push(artifact),
        }
    }

    groups
}

pub(crate) fn compare_runs_for_review(
    left: &DetailTaskRun,
    right: &DetailTaskRun,
) -> std::cmp::Ordering {
    review_time(right)
        .cmp(review_time(left))
        .then_with(|| right.id.cmp(&left.id))
}

pub(crate) fn review_time(run: &DetailTaskRun) -> &str {
    run.completed_at
        .as_deref()
        .or(run.started_at.as_deref())
        .unwrap_or(&run.created_at)
}

pub(crate) fn compare_artifacts_chronologically(
    left: &DetailArtifact,
    right: &DetailArtifact,
) -> std::cmp::Ordering {
    left.created_at
        .cmp(&right.created_at)
        .then_with(|| left.id.cmp(&right.id))
}

pub(crate) fn compare_events_chronologically(
    left: &DetailEvent,
    right: &DetailEvent,
) -> std::cmp::Ordering {
    left.occurred_at
        .cmp(&right.occurred_at)
        .then_with(|| left.id.cmp(&right.id))
}

pub(crate) fn compare_validation_runs_for_review(
    left: &DetailValidationRun,
    right: &DetailValidationRun,
) -> std::cmp::Ordering {
    validation_review_time(right)
        .cmp(validation_review_time(left))
        .then_with(|| right.id.cmp(&left.id))
}

pub(crate) fn validation_review_time(run: &DetailValidationRun) -> &str {
    run.completed_at
        .as_deref()
        .or(run.started_at.as_deref())
        .unwrap_or(&run.created_at)
}
