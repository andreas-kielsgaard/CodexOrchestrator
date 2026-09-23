import type { Ref } from 'react';
import { BranchBrowser } from '../branches/BranchBrowser';
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
  onViewDetached,
  detachedSelected,
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
  readonly onViewDetached: () => void;
  readonly detachedSelected: boolean;
}) {
  const attachedBranches = branches.filter((branch) => branch.target.kind === 'branch');
  const detachedCount = branches.filter(
    (branch) => branch.target.kind === 'worktree' && branch.branchRef === null,
  ).length;
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

      <BranchBrowser
        repositoryId={selectedRepositoryId}
        branches={attachedBranches}
        selectedTarget={selectedTarget}
        onBranchChange={onBranchChange}
        onOpenGraph={onSelectBranch}
        graphTriggerRef={graphTriggerRef}
        disabled={disabled}
      />
      <section className="worktree-review__detached-nav" aria-label="Detached worktrees">
        <div className="worktree-review__branch-heading">
          <h2>Detached worktrees</h2>
          <span>{detachedCount}</span>
        </div>
        <button
          type="button"
          className="worktree-review__secondary"
          disabled={disabled || detachedCount === 0}
          aria-current={detachedSelected ? 'page' : undefined}
          onClick={onViewDetached}
        >
          View details
        </button>
      </section>
    </aside>
  );
}
