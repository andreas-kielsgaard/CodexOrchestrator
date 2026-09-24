import type { Ref } from 'react';
import {
  targetKey,
  sourceTarget,
  type ReviewTarget,
  type ReviewBranch,
} from '../../application/branches';
import './branchBrowser.css';
export function BranchBrowser({
  repositoryId,
  branches,
  selectedTarget,
  onBranchChange,
  onOpenGraph,
  graphTriggerRef,
  disabled = false,
  unreadTargetKeys,
}: {
  readonly repositoryId: string;
  readonly branches: readonly ReviewBranch[];
  readonly selectedTarget: ReviewTarget | null;
  readonly onBranchChange: (target: ReviewTarget) => void;
  readonly onOpenGraph: () => void;
  readonly graphTriggerRef?: Ref<HTMLButtonElement>;
  readonly disabled?: boolean;
  readonly unreadTargetKeys?: ReadonlySet<string>;
}) {
  const source = selectedTarget ? sourceTarget(selectedTarget) : null;
  const selectedRow = source
    ? (branches.find((branch) => targetKey(branch.target) === targetKey(source)) ??
      (source.kind === 'worktree'
        ? branches.find((branch) => branch.worktreeIds.includes(source.worktreeId))
        : undefined))
    : undefined;
  return (
    <section className="branch-browser" data-repository-id={repositoryId}>
      <button
        type="button"
        ref={graphTriggerRef}
        className="worktree-review__secondary worktree-review__select-branch"
        disabled={disabled || branches.length === 0}
        onClick={onOpenGraph}
      >
        Select branch…
      </button>
      <div className="worktree-review__branch-heading">
        <h2>Branches</h2>
        <span>{branches.length}</span>
      </div>
      {branches.length === 0 ? (
        <p className="worktree-review__empty">No branches are available in this repository.</p>
      ) : (
        <ul className="worktree-review__branch-list" aria-label="Branches">
          {branches.map((branch) => (
            <li key={targetKey(branch.target)}>
              <button
                type="button"
                className="worktree-review__branch"
                aria-current={branch === selectedRow ? 'true' : undefined}
                disabled={disabled}
                onClick={() => onBranchChange(branch.target)}
              >
                <strong>{branch.displayName}</strong>
                {unreadTargetKeys?.has(targetKey(branch.target)) && (
                  <span className="worktree-review__notification-dot" aria-label="Build finished" />
                )}
                <span>{branch.tip.abbreviatedObjectId}</span>
                <small>
                  {branch.availableWorktreeCount}{' '}
                  {branch.availableWorktreeCount === 1 ? 'worktree' : 'worktrees'}
                </small>
                <small
                  title={
                    branch.activity
                      ? `Approximate activity; checked ${new Date(branch.activity.observedAt).toLocaleString()}`
                      : 'Latest commit; edit activity loads lazily'
                  }
                >
                  {branch.activity ? 'Edited approx. ' : 'Committed '}
                  {new Date(
                    branch.activity?.changedAt ?? branch.tip.committedAt,
                  ).toLocaleDateString()}
                </small>
              </button>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}
