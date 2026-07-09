import type {
  DiscoverTaskReposInput,
  DiscoveredTaskRepo,
  RegisterTaskRepoInput,
  TaskDashboardSnapshot,
} from '../application/commands/taskDashboardClient';

export type {
  DiscoverTaskReposInput,
  DiscoveredTaskRepo,
  RegisterTaskRepoInput,
} from '../application/commands/taskDashboardClient';

export interface RegisterTaskRepoCapability {
  registerRepo(input: RegisterTaskRepoInput): Promise<TaskDashboardSnapshot>;
}

export interface DiscoverTaskReposCapability {
  discoverRepos(input: DiscoverTaskReposInput): Promise<DiscoveredTaskRepo[]>;
}

export interface RepoOnboardingCapability {
  registerRepo?: RegisterTaskRepoCapability['registerRepo'];
  discoverRepos?: DiscoverTaskReposCapability['discoverRepos'];
}
