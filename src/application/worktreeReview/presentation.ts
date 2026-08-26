import type {
  ArtifactState,
  AssociationBaseline,
  BuildWorkspacePlan,
  CleanupState,
  ReviewBuildSource,
  ReviewOperationAttempt,
  WorktreeChanges,
} from './contracts';

export function shortObjectId(objectId: string): string {
  return objectId.slice(0, 8);
}

export function baselineLabel(baseline: AssociationBaseline): string {
  switch (baseline.kind) {
    case 'created_at':
      return `Created at ${baseline.commit.abbreviatedObjectId}`;
    case 'observed_at_association':
      return `First observed at ${baseline.commit.abbreviatedObjectId}`;
    case 'user_selected':
      return `Selected baseline ${baseline.commit.abbreviatedObjectId}`;
  }
}

export function changesLabel(changes: WorktreeChanges): string {
  const parts = [
    plural(changes.commitsAheadOfBaseline, 'commit') + ' since baseline',
    plural(changes.stagedFiles, 'staged file'),
    plural(changes.unstagedFiles, 'unstaged file'),
    plural(changes.untrackedFiles, 'untracked file'),
  ];
  if (changes.commitsBehindBaseline > 0) {
    parts.splice(1, 0, `${plural(changes.commitsBehindBaseline, 'commit')} behind baseline`);
  }
  return parts.join(' · ');
}

export function hasWorkSinceBaseline(changes: WorktreeChanges): boolean {
  return (
    changes.commitsAheadOfBaseline > 0 ||
    changes.commitsBehindBaseline > 0 ||
    changes.stagedFiles > 0 ||
    changes.unstagedFiles > 0 ||
    changes.untrackedFiles > 0
  );
}

export function sourceLabel(source: ReviewBuildSource): string {
  switch (source.kind) {
    case 'existing_worktree':
      return `Direct worktree at ${shortObjectId(source.expectedHead)}`;
    case 'worktree_snapshot':
      return `Worktree snapshot from ${shortObjectId(source.baseObjectId)}`;
    case 'branch_commit':
      return `Branch commit ${shortObjectId(source.objectId)}`;
  }
}

export function workspacePlanDisclosure(plan: BuildWorkspacePlan): string {
  switch (plan.kind) {
    case 'borrow_selected_worktree':
      return 'This build will run in the selected Worktree checkout. Worktree Review will not remove it.';
    case 'create_managed_branch_worktree':
      return 'Creating this build will first create and retain a Worktree checkout for this branch.';
    case 'create_owned_build_worktree':
      return 'Creating this build will create and retain an isolated Worktree checkout for the selected source.';
  }
}

export function attemptLabel(attempt: ReviewOperationAttempt | undefined): string {
  if (!attempt) return 'No build attempt';
  if (attempt.executionState === 'interrupted') return 'Interrupted';
  if (attempt.executionState !== 'completed')
    return `${capitalize(attempt.executionState)} · ${attempt.stage}`;
  switch (attempt.verdict) {
    case 'passed':
      return 'Build passed';
    case 'failed':
      return 'Build failed';
    case 'unknown':
      return 'Outcome unknown';
    case 'not_evaluated':
      return 'Completed without a build verdict';
  }
}

export function artifactLabel(artifact: ArtifactState): string {
  switch (artifact.state) {
    case 'not_produced':
      return 'No retained build output';
    case 'promotion_failed':
      return 'Artifact promotion failed';
    case 'available':
      return `${plural(artifact.fileCount, 'retained file')} · ${artifact.storageLabel}`;
    case 'removed':
      return 'Artifacts removed; receipt retained';
  }
}

export function cleanupLabel(cleanup: CleanupState): string {
  switch (cleanup.state) {
    case 'retained':
      return `Retained · ${cleanup.policy}`;
    case 'not_eligible':
      return `Protected from cleanup · ${cleanup.reason}`;
    case 'eligible':
      return `Eligible for cleanup · ${cleanup.reason}`;
    case 'running':
      return `Cleanup ${cleanup.completedEffects} of ${cleanup.totalEffects}`;
    case 'failed':
      return `Cleanup incomplete · ${cleanup.summary}`;
    case 'complete':
      return `Cleanup complete · ${cleanup.summary}`;
  }
}

function plural(count: number, noun: string): string {
  return `${count} ${noun}${count === 1 ? '' : 's'}`;
}

function capitalize(value: string): string {
  return value.charAt(0).toUpperCase() + value.slice(1);
}
