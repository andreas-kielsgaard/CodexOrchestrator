import type { ComponentType } from 'react';
import type { RepositoryCatalogClient } from '../../application/repositoryCatalog';
import type { RepoBranchWorktreeTargetSelectorProps } from '../../application/worktreeTargets';
import { RepositoryWorktreeTargetSelector } from './RepositoryWorktreeTargetSelector';

export function createRepositoryWorktreeTargetSelector(
  catalog: Pick<RepositoryCatalogClient, 'listWorktreeTargets'>,
): ComponentType<RepoBranchWorktreeTargetSelectorProps> {
  return function BoundRepositoryWorktreeTargetSelector(props) {
    return <RepositoryWorktreeTargetSelector {...props} catalog={catalog} />;
  };
}
