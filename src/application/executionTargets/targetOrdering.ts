import type { ReviewBranch, ReviewTarget } from '../branches';
import type { ProfileWorktreeTargetsDto } from './contracts';

function timestamp(value: string | null | undefined) {
  return value ? Date.parse(value) || 0 : 0;
}

/**
 * Agent Session ordering is device-scoped: an existing checkout is useful only when the
 * selected execution device can actually run it. Worktree Review retains its own overview order.
 */
export function orderTargetBranches(
  branches: readonly ReviewBranch[],
  referenceTarget: ReviewTarget | null,
  profiles: readonly ProfileWorktreeTargetsDto[],
): readonly ReviewBranch[] {
  const defaultBranch = referenceTarget?.kind === 'branch' ? referenceTarget.branchRef : null;
  const facts = new Map<string, { instantiated: boolean; dirty: boolean; committedAt: number }>();
  for (const profile of profiles) {
    for (const instance of profile.instances) {
      const current = facts.get(instance.branchRef) ?? {
        instantiated: false,
        dirty: false,
        committedAt: 0,
      };
      current.instantiated = true;
      current.dirty ||= Boolean(instance.dirty);
      current.committedAt = Math.max(current.committedAt, timestamp(instance.headCommittedAt));
      facts.set(instance.branchRef, current);
    }
  }
  return [...branches].sort((left, right) => {
    const leftDefault = left.branchRef === defaultBranch;
    const rightDefault = right.branchRef === defaultBranch;
    const leftFacts = facts.get(left.branchRef ?? '') ?? { instantiated: false, dirty: false, committedAt: 0 };
    const rightFacts = facts.get(right.branchRef ?? '') ?? { instantiated: false, dirty: false, committedAt: 0 };
    return (
      Number(rightDefault) - Number(leftDefault) ||
      Number(rightFacts.instantiated) - Number(leftFacts.instantiated) ||
      Number(rightFacts.dirty) - Number(leftFacts.dirty) ||
      Math.max(rightFacts.committedAt, timestamp(right.tip.committedAt)) -
        Math.max(leftFacts.committedAt, timestamp(left.tip.committedAt)) ||
      left.displayName.localeCompare(right.displayName)
    );
  });
}

export function targetWorktreeLabel(branchRef: string, path: string) {
  const branch = branchRef.replace(/^refs\/heads\//, '');
  const instance = path.split(/[\\/]/).filter(Boolean).at(-1);
  return instance && instance !== branch ? `${branch} · ${instance}` : branch;
}
