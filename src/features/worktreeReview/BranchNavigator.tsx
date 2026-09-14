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
        branches={branches}
        selectedTarget={selectedTarget}
        onBranchChange={onBranchChange}
        onOpenGraph={onSelectBranch}
        graphTriggerRef={graphTriggerRef}
        disabled={disabled}
      />
    </aside>
  );
}
