import {
  dashboardGroupOrder,
  projectOpenTaskDashboard,
  type DashboardGroup,
} from '../../domain/dashboardProjection';
import type { AttentionState, EntityId, ExecutionState, Task } from '../../domain/model';
import type { OpenTaskDashboardStore } from '../../domain/openTaskDashboardStore';
import type { CreateOpenTaskInput, OpenTaskWriteStore } from '../../domain/openTaskWriteStore';

export interface TaskDashboardProject {
  id: EntityId;
  name: string;
}

export interface TaskDashboardRepo {
  id: EntityId;
  projectId: EntityId;
  project: string;
  name: string;
  rootPath: string;
}

export interface TaskDashboardWorktreeAnchor {
  id: EntityId;
  projectId: EntityId;
  project: string;
  repoId: EntityId;
  repo: string;
  branchId?: EntityId;
  branch?: string;
  path: string;
}

export interface TaskDashboardSnapshot {
  groups: DashboardGroup[];
  projects: TaskDashboardProject[];
  repos: TaskDashboardRepo[];
  worktreeAnchors: TaskDashboardWorktreeAnchor[];
  totalOpenTasks: number;
}

export interface CreateTaskDashboardTaskInput {
  projectId: EntityId;
  repoId?: EntityId;
  branchId?: EntityId;
  worktreeId?: EntityId;
  title: string;
  summary: string;
  executionState?: ExecutionState;
  attentionState?: AttentionState;
  priority?: Task['priority'];
}

export interface RegisterTaskWorktreeInput {
  projectName: string;
  repoName?: string;
  repoRootPath: string;
  branchName?: string;
  worktreePath: string;
  isMain?: boolean;
}

export interface RegisterTaskRepoInput {
  repoRootPath: string;
  projectName?: string;
  repoName?: string;
}

export interface DiscoverTaskReposInput {
  rootPath: string;
  maxDepth?: number;
}

export interface DiscoveredTaskRepo {
  name: string;
  path: string;
}

export interface UpdateTaskDashboardTaskInput {
  title?: string;
  summary?: string;
  executionState?: ExecutionState;
  attentionState?: AttentionState;
  priority?: Task['priority'];
}

export interface TaskDashboardClient {
  loadDashboard(): Promise<TaskDashboardSnapshot>;
  registerWorktree?(input: RegisterTaskWorktreeInput): Promise<TaskDashboardSnapshot>;
  registerRepo?(input: RegisterTaskRepoInput): Promise<TaskDashboardSnapshot>;
  discoverRepos?(input: DiscoverTaskReposInput): Promise<DiscoveredTaskRepo[]>;
  createTask(input: CreateTaskDashboardTaskInput): Promise<TaskDashboardSnapshot>;
  updateTask(taskId: EntityId, input: UpdateTaskDashboardTaskInput): Promise<TaskDashboardSnapshot>;
  archiveTask(taskId: EntityId): Promise<TaskDashboardSnapshot>;
}

export interface StoreBackedTaskDashboardClientStores {
  dashboard: OpenTaskDashboardStore;
  write: OpenTaskWriteStore;
}

export function createStoreBackedTaskDashboardClient(
  stores: StoreBackedTaskDashboardClientStores,
): TaskDashboardClient {
  return {
    async loadDashboard(): Promise<TaskDashboardSnapshot> {
      return loadTaskDashboardSnapshot(stores.dashboard);
    },

    async createTask(input: CreateTaskDashboardTaskInput): Promise<TaskDashboardSnapshot> {
      await stores.write.createTask(normalizeCreateTaskInput(input));
      return loadTaskDashboardSnapshot(stores.dashboard);
    },

    async updateTask(
      taskId: EntityId,
      input: UpdateTaskDashboardTaskInput,
    ): Promise<TaskDashboardSnapshot> {
      await stores.write.updateTask(taskId, input);
      return loadTaskDashboardSnapshot(stores.dashboard);
    },

    async archiveTask(taskId: EntityId): Promise<TaskDashboardSnapshot> {
      await stores.write.archiveTask(taskId);
      return loadTaskDashboardSnapshot(stores.dashboard);
    },
  };
}

export function emptyTaskDashboardSnapshot(): TaskDashboardSnapshot {
  return {
    groups: dashboardGroupOrder.map((group) => ({ ...group, tasks: [] })),
    projects: [],
    repos: [],
    worktreeAnchors: [],
    totalOpenTasks: 0,
  };
}

async function loadTaskDashboardSnapshot(
  store: OpenTaskDashboardStore,
): Promise<TaskDashboardSnapshot> {
  const records = await store.loadOpenTaskDashboardRecords();
  const groups = projectOpenTaskDashboard(records);

  return {
    groups,
    projects: records.projects
      .map((project) => ({ id: project.id, name: project.name }))
      .sort((left, right) => left.name.localeCompare(right.name)),
    repos: loadDashboardRepos(records),
    worktreeAnchors: loadWorktreeAnchors(records),
    totalOpenTasks: groups.reduce((total, group) => total + group.tasks.length, 0),
  };
}

function normalizeCreateTaskInput(input: CreateTaskDashboardTaskInput): CreateOpenTaskInput {
  return {
    projectId: input.projectId,
    repoId: input.repoId,
    branchId: input.branchId,
    worktreeId: input.worktreeId,
    title: input.title,
    summary: input.summary,
    executionState: input.executionState ?? 'draft',
    attentionState: input.attentionState ?? 'needs_action_now',
    priority: input.priority ?? 'normal',
  };
}

function loadDashboardRepos(records: {
  projects: Array<{ id: EntityId; name: string }>;
  repos: Array<{ id: EntityId; projectId: EntityId; name: string; rootPath: string }>;
}): TaskDashboardRepo[] {
  const projectsById = new Map(records.projects.map((project) => [project.id, project]));

  return records.repos
    .flatMap((repo): TaskDashboardRepo[] => {
      const project = projectsById.get(repo.projectId);

      if (!project) {
        return [];
      }

      return [
        {
          id: repo.id,
          projectId: project.id,
          project: project.name,
          name: repo.name,
          rootPath: repo.rootPath,
        },
      ];
    })
    .sort((left, right) =>
      `${left.project}\u0000${left.name}\u0000${left.rootPath}`.localeCompare(
        `${right.project}\u0000${right.name}\u0000${right.rootPath}`,
      ),
    );
}

function loadWorktreeAnchors(records: {
  projects: Array<{ id: EntityId; name: string }>;
  repos: Array<{ id: EntityId; projectId: EntityId; name: string }>;
  branches: Array<{ id: EntityId; repoId: EntityId; name: string }>;
  worktrees: Array<{ id: EntityId; repoId: EntityId; branchId?: EntityId; path: string }>;
}): TaskDashboardWorktreeAnchor[] {
  const projectsById = new Map(records.projects.map((project) => [project.id, project]));
  const reposById = new Map(records.repos.map((repo) => [repo.id, repo]));
  const branchesById = new Map(records.branches.map((branch) => [branch.id, branch]));

  return records.worktrees
    .flatMap((worktree): TaskDashboardWorktreeAnchor[] => {
      const repo = reposById.get(worktree.repoId);
      const project = repo ? projectsById.get(repo.projectId) : undefined;

      if (!repo || !project) {
        return [];
      }

      const branch = worktree.branchId ? branchesById.get(worktree.branchId) : undefined;

      return [
        {
          id: worktree.id,
          projectId: project.id,
          project: project.name,
          repoId: repo.id,
          repo: repo.name,
          ...(branch ? { branchId: branch.id, branch: branch.name } : {}),
          path: worktree.path,
        },
      ];
    })
    .sort((left, right) =>
      `${left.project}\u0000${left.repo}\u0000${left.path}`.localeCompare(
        `${right.project}\u0000${right.repo}\u0000${right.path}`,
      ),
    );
}