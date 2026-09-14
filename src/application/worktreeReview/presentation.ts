export { targetKey, sourceTarget, orderReviewTargets } from '../branches';
import type {
  BuildOutputState,
  AssociationBaseline,
  BuildWorkspacePlan,
  CleanupState,
  ReviewBuildSource,
  ReviewOperationAttempt,
  WorktreeChanges,
  ReviewBranch,
  ReviewTarget,
  GitCommit,
  CommitSourceContext,
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
    case 'physical_worktree':
      return `${source.snapshot ? 'Snapshot' : 'Live checkout'} at ${shortObjectId(source.headObjectId)} � captured ${shortObjectId(source.capturedObjectId)}`;
    case 'exact_commit':
      return `Commit ${shortObjectId(source.objectId)}`;
    case 'existing_worktree':
      return source.triggerVirtualCommitId
        ? `Live checkout triggered at ${shortObjectId(source.triggerHeadObjectId)} · virtual commit ${shortObjectId(source.triggerVirtualCommitId)}`
        : `Live checkout triggered at ${shortObjectId(source.triggerHeadObjectId)}`;
    case 'worktree_snapshot':
      return source.virtualCommitId
        ? `Retained checkout from virtual commit ${shortObjectId(source.virtualCommitId)} · source HEAD ${shortObjectId(source.headObjectId)}`
        : `Retained checkout from ${shortObjectId(source.capturedObjectId)}`;
    case 'branch_commit':
      return `Branch commit ${shortObjectId(source.objectId)}`;
  }
}

export function workspacePlanDisclosure(plan: BuildWorkspacePlan): string {
  switch (plan.kind) {
    case 'borrow_physical_worktree':
    case 'borrow_selected_worktree':
      return 'This build will compile the live Worktree checkout using its existing dependencies. Worktree Review will neither install dependencies in nor remove that checkout.';
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
  switch (attempt.outcome) {
    case 'succeeded':
      return 'Compilation completed';
    case 'failed':
      return 'Compilation failed';
    case 'unknown':
      return 'Outcome unknown';
    case 'not_completed':
      return 'Compilation did not complete';
  }
}

export function buildOutputLabel(output: BuildOutputState): string {
  switch (output.state) {
    case 'not_produced':
      return 'No retained build output';
    case 'unavailable':
      return `Retained output unavailable · ${output.summary}`;
    case 'available':
      return `Available · ${output.storageLabel}`;
    case 'removed':
      return 'Build output removed; receipt retained';
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

export function commitSourceContext(branch: ReviewBranch): CommitSourceContext {
  const target = branch.target;
  if (target.kind === 'commit') return target.context;
  return target.kind === 'branch'
    ? { kind: 'branch', branchRef: target.branchRef, tipObjectId: branch.tip.objectId }
    : { kind: 'worktree', worktreeId: target.worktreeId, tipObjectId: branch.tip.objectId };
}

export function commitTarget(branch: ReviewBranch, objectId: string): ReviewTarget {
  return {
    kind: 'commit',
    repositoryId: branch.repositoryId,
    objectId,
    context: commitSourceContext(branch),
  };
}

export function uniqueCommits(commits: readonly GitCommit[]): GitCommit[] {
  return [...new Map(commits.map((commit) => [commit.objectId, commit])).values()];
}
