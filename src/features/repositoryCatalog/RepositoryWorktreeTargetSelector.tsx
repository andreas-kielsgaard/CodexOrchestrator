import { useEffect, useState } from 'react';
import type { RepositoryCatalogClient } from '../../application/repositoryCatalog';
import type {
  RepoBranchWorktreeTargetSelectorProps,
  ResolvedRepoBranchWorktreeTarget,
} from '../../application/worktreeTargets';
import './repositoryWorktreeTargetSelector.css';

type LoadState =
  | { readonly kind: 'loading' }
  | { readonly kind: 'ready'; readonly targets: readonly ResolvedRepoBranchWorktreeTarget[] }
  | { readonly kind: 'failed'; readonly message: string };

export interface RepositoryWorktreeTargetSelectorProps extends RepoBranchWorktreeTargetSelectorProps {
  readonly catalog: Pick<RepositoryCatalogClient, 'listWorktreeTargets'>;
}

export function RepositoryWorktreeTargetSelector({
  id,
  value,
  disabled = false,
  onChange,
  catalog,
}: RepositoryWorktreeTargetSelectorProps) {
  const [load, setLoad] = useState<LoadState>({ kind: 'loading' });

  useEffect(() => {
    let current = true;
    setLoad({ kind: 'loading' });
    void catalog.listWorktreeTargets().then(
      (targets) => current && setLoad({ kind: 'ready', targets }),
      (error: unknown) =>
        current &&
        setLoad({
          kind: 'failed',
          message: error instanceof Error ? error.message : String(error),
        }),
    );
    return () => {
      current = false;
    };
  }, [catalog]);

  const targets = load.kind === 'ready' ? load.targets : [];
  const unavailable = disabled || load.kind !== 'ready' || targets.length === 0;

  return (
    <div className="repository-worktree-target-selector">
      <select
        id={id}
        aria-label={id ? undefined : 'Repository and branch'}
        value={value?.worktree.id ?? ''}
        disabled={unavailable}
        aria-busy={load.kind === 'loading'}
        onChange={(event) => {
          const target = targets.find(
            (candidate) => candidate.worktree.id === event.currentTarget.value,
          );
          if (target) onChange(target);
        }}
      >
        <option value="" disabled>
          {load.kind === 'loading'
            ? 'Loading worktrees…'
            : targets.length === 0
              ? 'No registered branch worktrees'
              : 'Select repository and branch'}
        </option>
        {targets.map((target) => (
          <option key={target.worktree.id} value={target.worktree.id}>
            {target.repository.name} · {target.branch.name}
          </option>
        ))}
      </select>
      {load.kind === 'failed' ? (
        <p className="repository-worktree-target-selector__error" role="alert">
          {load.message}
        </p>
      ) : null}
    </div>
  );
}
