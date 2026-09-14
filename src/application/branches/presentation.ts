import type { ReviewTarget, ReviewBranch } from './contracts';
export function targetKey(target: ReviewTarget): string {
  const source =
    target.kind === 'branch'
      ? target.branchRef
      : target.kind === 'worktree'
        ? target.worktreeId
        : `${target.objectId}:${JSON.stringify(target.context)}`;
  return `${target.repositoryId}:${target.kind}:${source}`;
}

export function orderReviewTargets(targets: readonly ReviewBranch[]): ReviewBranch[] {
  const timestamp = (target: ReviewBranch) =>
    Date.parse(target.activity?.changedAt ?? target.tip.committedAt) || 0;
  return [...targets].sort(
    (left, right) =>
      Number(right.availableWorktreeCount > 0) - Number(left.availableWorktreeCount > 0) ||
      timestamp(right) - timestamp(left) ||
      left.displayName.localeCompare(right.displayName) ||
      targetKey(left.target).localeCompare(targetKey(right.target)),
  );
}

export function sourceTarget(target: ReviewTarget): ReviewTarget {
  if (target.kind !== 'commit') return target;
  return target.context.kind === 'branch'
    ? { kind: 'branch', repositoryId: target.repositoryId, branchRef: target.context.branchRef }
    : {
        kind: 'worktree',
        repositoryId: target.repositoryId,
        worktreeId: target.context.worktreeId,
      };
}
