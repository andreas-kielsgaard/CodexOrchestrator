import { invoke } from '@tauri-apps/api/core';
import type {
  RegisteredRepository,
  RepositoryCatalogClient,
  RepositoryCatalogOverview,
  RepositoryId,
} from '../../application/repositoryCatalog';
import type { ResolvedRepoBranchWorktreeTarget } from '../../application/worktreeTargets';

export type RepositoryCatalogInvoke = <Result>(
  command: string,
  args?: Record<string, unknown>,
) => Promise<Result>;

export function createTauriRepositoryCatalogClient(
  invokeCommand: RepositoryCatalogInvoke = invoke,
): RepositoryCatalogClient {
  return {
    overview: () => invokeCommand<RepositoryCatalogOverview>('repository_catalog_overview'),
    registerDirectory: (repositoryRoot: string) =>
      invokeCommand<RegisteredRepository>('register_repository_directory', {
        input: { repositoryRoot },
      }),
    registerCodexRepository: (repositoryId: RepositoryId) =>
      invokeCommand<RegisteredRepository>('register_codex_repository', {
        input: { repositoryId },
      }),
    listWorktreeTargets: () =>
      invokeCommand<ResolvedRepoBranchWorktreeTarget[]>(
        'list_registered_repository_worktree_targets',
      ),
  };
}

export const tauriRepositoryCatalog = createTauriRepositoryCatalogClient();
