import type { RegisterTaskRepoInput } from '../../../capabilities/repoOnboarding';
import type { DiscoveredTaskRepo } from '../../../capabilities/repoOnboarding';
import { compactPath } from '../../../app/viewModels/formatting';

export interface RepoSetupDraft {
  projectName: string;
  repoRootPath: string;
}

export interface RepoSetupFormViewModel extends RepoSetupDraft {
  scanRootPath: string;
}

export interface DiscoveredRepoOption {
  name: string;
  path: string;
  compactPath: string;
}

export function createDiscoveredRepoOptions(
  repos: DiscoveredTaskRepo[],
): DiscoveredRepoOption[] {
  return repos.map((repo) => ({
    name: repo.name,
    path: repo.path,
    compactPath: compactPath(repo.path),
  }));
}

export function normalizeRepoSetupInput(
  form: RepoSetupDraft,
): RegisterTaskRepoInput | undefined {
  const repoRootPath = form.repoRootPath.trim();

  if (!repoRootPath) {
    return undefined;
  }

  return {
    repoRootPath,
    ...(form.projectName.trim() ? { projectName: form.projectName.trim() } : {}),
  };
}
