use super::domain::{
    OperationExecutionState, OperationVerdict, RetentionKey, ReviewBuild, ReviewBuildId,
    ReviewOperationAttempt,
};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum RetentionDisposition {
    KeepNewestSuccessful,
    EligibleTerminal,
    LeaveRunning,
    Unverified,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RetentionDecision {
    pub(crate) build_id: ReviewBuildId,
    pub(crate) disposition: RetentionDisposition,
}

/// Evaluates policy without performing effects. Cleanup remains a separate, durable workflow.
pub(crate) fn evaluate(
    builds: &[ReviewBuild],
    latest_attempts: &HashMap<ReviewBuildId, ReviewOperationAttempt>,
    keep_successful: usize,
) -> Vec<RetentionDecision> {
    let mut successful_by_source: HashMap<RetentionKey, Vec<&ReviewBuild>> = HashMap::new();
    for build in builds {
        let Some(attempt) = latest_attempts.get(&build.id) else {
            continue;
        };
        if attempt.execution == OperationExecutionState::Completed
            && attempt.verdict == OperationVerdict::Passed
        {
            successful_by_source
                .entry(build.retention_key.clone())
                .or_default()
                .push(build);
        }
    }
    let mut protected = HashSet::new();
    for builds in successful_by_source.values_mut() {
        builds.sort_by(|left, right| {
            right
                .updated_at
                .cmp(&left.updated_at)
                .then_with(|| right.id.cmp(&left.id))
        });
        protected.extend(
            builds
                .iter()
                .take(keep_successful.max(1))
                .map(|build| build.id.clone()),
        );
    }

    builds
        .iter()
        .map(|build| {
            let disposition = match latest_attempts.get(&build.id) {
                Some(attempt)
                    if matches!(
                        attempt.execution,
                        OperationExecutionState::Pending | OperationExecutionState::Running
                    ) =>
                {
                    RetentionDisposition::LeaveRunning
                }
                Some(attempt)
                    if attempt.execution == OperationExecutionState::Completed
                        && attempt.verdict == OperationVerdict::Passed
                        && protected.contains(&build.id) =>
                {
                    RetentionDisposition::KeepNewestSuccessful
                }
                Some(attempt)
                    if attempt.execution == OperationExecutionState::Completed
                        && attempt.verdict != OperationVerdict::Unknown =>
                {
                    RetentionDisposition::EligibleTerminal
                }
                _ => RetentionDisposition::Unverified,
            };
            RetentionDecision {
                build_id: build.id.clone(),
                disposition,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::worktree_review::domain::{
        BranchRef, GitObjectId, OperationAttemptId, ReviewBuildName, ReviewOperationKind,
        ReviewSourceSelection, SourceBinding, SourceFingerprint, WorkspaceId,
    };
    use chrono::{Duration, Utc};

    fn build(id: &str, source: &str, age_minutes: i64) -> ReviewBuild {
        let now = Utc::now() - Duration::minutes(age_minutes);
        let workspace_id = WorkspaceId::new(format!("workspace-{id}")).unwrap();
        ReviewBuild {
            id: ReviewBuildId::new(id).unwrap(),
            name: ReviewBuildName::new(format!("Build {id}")).unwrap(),
            source: SourceBinding {
                repository_id: crate::worktree_review::domain::RepositoryId::new("repository")
                    .unwrap(),
                branch_ref: BranchRef::new("refs/heads/main").unwrap(),
                selection: ReviewSourceSelection::BranchCommit {
                    selected_object: GitObjectId::new("a".repeat(40)).unwrap(),
                },
                workspace_id: workspace_id.clone(),
                materialized_object: GitObjectId::new("a".repeat(40)).unwrap(),
                materialized_state_fingerprint: SourceFingerprint::new("fingerprint").unwrap(),
            },
            workspace_id,
            retention_key: RetentionKey::new(source).unwrap(),
            current_artifact_set_id: None,
            lifecycle: crate::worktree_review::domain::BuildLifecycle::Active,
            created_at: now,
            updated_at: now,
        }
    }

    fn attempt(
        build: &ReviewBuild,
        execution: OperationExecutionState,
        verdict: OperationVerdict,
    ) -> ReviewOperationAttempt {
        let now = Utc::now();
        ReviewOperationAttempt {
            id: OperationAttemptId::new(format!("attempt-{}", build.id)).unwrap(),
            build_id: build.id.clone(),
            kind: ReviewOperationKind::Build,
            execution,
            verdict,
            active_stage: None,
            failure: None,
            requested_at: now,
            started_at: (execution != OperationExecutionState::Pending).then_some(now),
            completed_at: matches!(
                execution,
                OperationExecutionState::Completed | OperationExecutionState::Interrupted
            )
            .then_some(now),
        }
    }

    #[test]
    fn protects_only_the_newest_successful_build_per_logical_source() {
        let old = build("old", "source", 10);
        let new = build("new", "source", 1);
        let attempts = HashMap::from([
            (
                old.id.clone(),
                attempt(
                    &old,
                    OperationExecutionState::Completed,
                    OperationVerdict::Passed,
                ),
            ),
            (
                new.id.clone(),
                attempt(
                    &new,
                    OperationExecutionState::Completed,
                    OperationVerdict::Passed,
                ),
            ),
        ]);

        let decisions = evaluate(&[old.clone(), new.clone()], &attempts, 1);

        assert_eq!(
            decisions,
            vec![
                RetentionDecision {
                    build_id: old.id,
                    disposition: RetentionDisposition::EligibleTerminal,
                },
                RetentionDecision {
                    build_id: new.id,
                    disposition: RetentionDisposition::KeepNewestSuccessful,
                },
            ]
        );
    }

    #[test]
    fn leaves_running_builds_alone_and_does_not_infer_unknown_outcomes() {
        let running = build("running", "source", 20);
        let interrupted = build("interrupted", "source", 30);
        let attempts = HashMap::from([
            (
                running.id.clone(),
                attempt(
                    &running,
                    OperationExecutionState::Running,
                    OperationVerdict::Unknown,
                ),
            ),
            (
                interrupted.id.clone(),
                attempt(
                    &interrupted,
                    OperationExecutionState::Interrupted,
                    OperationVerdict::Unknown,
                ),
            ),
        ]);

        let decisions = evaluate(&[running, interrupted], &attempts, 1);

        assert_eq!(decisions[0].disposition, RetentionDisposition::LeaveRunning);
        assert_eq!(decisions[1].disposition, RetentionDisposition::Unverified);
    }
}
