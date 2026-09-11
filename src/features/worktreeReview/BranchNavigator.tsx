import type { Ref } from 'react';
import { targetKey, sourceTarget } from '../../application/worktreeReview';
import type {
  ReviewTarget,
  RepositoryId,
  ReviewBranch,
  ReviewRepository,
} from '../../application/worktreeReview';

export function BranchNavigator({
  repositories,
  selectedRepositoryId,
  branches,
  selectedTarget,
  onSelectBranch,
  graphTriggerRef,
  disabled,
  onRepositoryChange,
  onRegisterRepository,
  onBranchChange,
}: {
  readonly repositories: readonly ReviewRepository[];
  readonly selectedRepositoryId: RepositoryId | '';
  readonly branches: readonly ReviewBranch[];
  readonly selectedTarget: ReviewTarget | null;
  readonly onSelectBranch: () => void;
  readonly graphTriggerRef?: Ref<HTMLButtonElement>;
  readonly disabled: boolean;
  readonly onRepositoryChange: (repositoryId: RepositoryId) => void;
  readonly onRegisterRepository: () => void;
  readonly onBranchChange: (target: ReviewTarget) => void;
}) {
  const source = selectedTarget ? sourceTarget(selectedTarget) : null;
  const selectedRow = source
    ? (branches.find((branch) => targetKey(branch.target) === targetKey(source)) ??
      (source.kind === 'worktree'
        ? branches.find((branch) => branch.worktreeIds.includes(source.worktreeId))
        : undefined))
    : undefined;
  return (
    <aside className="worktree-review__navigator" aria-label="Repository and branch selection">
      <label className="worktree-review__field">
        <span>Repository</span>
        <select
          aria-label="Repository"
          value={selectedRepositoryId}
          disabled={disabled || repositories.length === 0}
          onChange={(event) => onRepositoryChange(event.target.value as RepositoryId)}
        >
          {repositories.length === 0 && <option value="">No repositories configured</option>}
          {repositories.map((repository) => (
            <option key={repository.repositoryId} value={repository.repositoryId}>
              {repository.name} — {repository.locationLabel}
            </option>
          ))}
        </select>
      </label>
      <button
        type="button"
        className="worktree-review__register-button"
        disabled={disabled}
        onClick={onRegisterRepository}
      >
        Add repository…
      </button>

      <button
        type="button"
        ref={graphTriggerRef}
        className="worktree-review__secondary worktree-review__select-branch"
        disabled={disabled || branches.length === 0}
        onClick={onSelectBranch}
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
    </aside>
  );
}
