import type {
  BranchRef,
  RepositoryId,
  ReviewBranch,
  ReviewRepository,
} from '../../application/worktreeReview';

export function BranchNavigator({
  repositories,
  selectedRepositoryId,
  branches,
  selectedBranchRef,
  disabled,
  onRepositoryChange,
  onRegisterRepository,
  onBranchChange,
}: {
  readonly repositories: readonly ReviewRepository[];
  readonly selectedRepositoryId: RepositoryId | '';
  readonly branches: readonly ReviewBranch[];
  readonly selectedBranchRef: BranchRef | '';
  readonly disabled: boolean;
  readonly onRepositoryChange: (repositoryId: RepositoryId) => void;
  readonly onRegisterRepository: () => void;
  readonly onBranchChange: (branchRef: BranchRef) => void;
}) {
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

      <div className="worktree-review__branch-heading">
        <h2>Branches</h2>
        <span>{branches.length}</span>
      </div>
      {branches.length === 0 ? (
        <p className="worktree-review__empty">No branches are available in this repository.</p>
      ) : (
        <ul className="worktree-review__branch-list" aria-label="Branches">
          {branches.map((branch) => (
            <li key={branch.branchRef}>
              <button
                type="button"
                className="worktree-review__branch"
                aria-current={branch.branchRef === selectedBranchRef ? 'true' : undefined}
                disabled={disabled}
                onClick={() => onBranchChange(branch.branchRef)}
              >
                <strong>{branch.displayName}</strong>
                <span>{branch.tip.abbreviatedObjectId}</span>
                <small>
                  {branch.associatedWorktreeCount}{' '}
                  {branch.associatedWorktreeCount === 1 ? 'worktree' : 'worktrees'}
                </small>
              </button>
            </li>
          ))}
        </ul>
      )}
    </aside>
  );
}
