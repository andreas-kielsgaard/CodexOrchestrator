import type { ResolvedRepoBranchWorktreeTarget } from '../worktreeTargets';
import type { RegisteredRepository, RepositoryCatalogOverview, RepositoryId } from './contracts';

/** Shared registered-repository use cases; no review or workflow behavior belongs here. */
export interface RepositoryCatalogClient {
  overview(): Promise<RepositoryCatalogOverview>;
  registerDirectory(repositoryRoot: string): Promise<RegisteredRepository>;
  registerCodexRepository(repositoryId: RepositoryId): Promise<RegisteredRepository>;
  listWorktreeTargets(): Promise<readonly ResolvedRepoBranchWorktreeTarget[]>;
}
