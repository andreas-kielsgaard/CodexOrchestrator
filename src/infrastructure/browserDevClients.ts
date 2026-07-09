import type {
  CreateTaskDashboardTaskInput,
  DiscoverTaskReposInput,
  DiscoveredTaskRepo,
  RegisterTaskRepoInput,
  RegisterTaskWorktreeInput,
  TaskDashboardClient,
  TaskDashboardSnapshot,
  UpdateTaskDashboardTaskInput,
} from '../application/taskDashboardClient';
import { emptyTaskDashboardSnapshot } from '../application/taskDashboardClient';
import type {
  RuntimeCommandClient,
  StartAgentSessionCommandInput,
  StartAgentSessionCommandResult,
  StartCodexTaskRunCommandInput,
  StartCodexTaskRunCommandResult,
} from '../application/runtimeCommandClient';
import type { TaskRunDetailClient } from '../application/taskRunDetailClient';
import { TaskRunDetailTaskNotFoundError } from '../application/taskRunDetailClient';
import type { EntityId, IsoDateTime } from '../domain/model';

export interface BrowserDevClientBundle {
  taskDashboardClient: TaskDashboardClient;
  taskRunDetailClient: TaskRunDetailClient;
  runtimeCommandClient: RuntimeCommandClient;
}

export function createBrowserDevClientBundle(): BrowserDevClientBundle {
  const taskDashboardClient = createBrowserDevTaskDashboardClient();

  return {
    taskDashboardClient,
    taskRunDetailClient: createBrowserDevTaskRunDetailClient(),
    runtimeCommandClient: createBrowserDevRuntimeCommandClient(),
  };
}

function createBrowserDevTaskDashboardClient(): TaskDashboardClient {
  let snapshot = emptyTaskDashboardSnapshot();

  return {
    async loadDashboard(): Promise<TaskDashboardSnapshot> {
      return cloneDashboard(snapshot);
    },
    async registerWorktree(input: RegisterTaskWorktreeInput): Promise<TaskDashboardSnapshot> {
      const projectId = `browser-project-${slug(input.projectName)}` as EntityId;
      const repoId = `browser-repo-${slug(input.repoName ?? input.projectName)}` as EntityId;
      const worktreeId = `browser-worktree-${slug(input.worktreePath)}` as EntityId;

      snapshot = {
        ...snapshot,
        projects: upsertById(snapshot.projects, {
          id: projectId,
          name: input.projectName,
        }),
        repos: upsertById(snapshot.repos, {
          id: repoId,
          projectId,
          project: input.projectName,
          name: input.repoName ?? input.projectName,
          rootPath: input.repoRootPath,
        }),
        worktreeAnchors: upsertById(snapshot.worktreeAnchors, {
          id: worktreeId,
          projectId,
          project: input.projectName,
          repoId,
          repo: input.repoName ?? input.projectName,
          path: input.worktreePath,
          ...(input.branchName ? { branchId: `browser-branch-${slug(input.branchName)}` } : {}),
          ...(input.branchName ? { branch: input.branchName } : {}),
        }),
      };

      return cloneDashboard(snapshot);
    },
    async registerRepo(input: RegisterTaskRepoInput): Promise<TaskDashboardSnapshot> {
      const projectName = input.projectName ?? 'Browser Dev Project';
      const repoName =
        input.repoName ?? input.repoRootPath.split(/[\\/]/).filter(Boolean).pop() ?? 'repo';
      const projectId = `browser-project-${slug(projectName)}` as EntityId;
      const repoId = `browser-repo-${slug(input.repoRootPath)}` as EntityId;

      snapshot = {
        ...snapshot,
        projects: upsertById(snapshot.projects, { id: projectId, name: projectName }),
        repos: upsertById(snapshot.repos, {
          id: repoId,
          projectId,
          project: projectName,
          name: repoName,
          rootPath: input.repoRootPath,
        }),
      };

      return cloneDashboard(snapshot);
    },
    async discoverRepos(input: DiscoverTaskReposInput): Promise<DiscoveredTaskRepo[]> {
      const name = input.rootPath.split(/[\\/]/).filter(Boolean).pop() ?? 'repo';

      return [{ name, path: input.rootPath }];
    },
    async createTask(input: CreateTaskDashboardTaskInput): Promise<TaskDashboardSnapshot> {
      const now = nowIso();
      const projectName =
        snapshot.projects.find((project) => project.id === input.projectId)?.name ??
        'Browser Dev Project';
      const taskId = `browser-task-${crypto.randomUUID()}` as EntityId;

      snapshot = {
        ...snapshot,
        projects: upsertById(snapshot.projects, {
          id: input.projectId,
          name: projectName,
        }),
        groups: snapshot.groups.map((group, index) =>
          index === 0
            ? {
                ...group,
                tasks: [
                  ...group.tasks,
                  {
                    id: taskId,
                    title: input.title,
                    summary: input.summary,
                    project: projectName,
                    executionState: input.executionState ?? 'draft',
                    attentionState: input.attentionState ?? 'needs_action_now',
                    priority: input.priority ?? 'normal',
                    ...(input.repoId
                      ? { repo: snapshot.repos.find((repo) => repo.id === input.repoId)?.name }
                      : {}),
                    ...(input.worktreeId
                      ? {
                          worktreePath: snapshot.worktreeAnchors.find(
                            (worktree) => worktree.id === input.worktreeId,
                          )?.path,
                        }
                      : {}),
                    updatedAt: now,
                  },
                ],
              }
            : group,
        ),
        totalOpenTasks: snapshot.totalOpenTasks + 1,
      };

      return cloneDashboard(snapshot);
    },
    async updateTask(
      taskId: EntityId,
      input: UpdateTaskDashboardTaskInput,
    ): Promise<TaskDashboardSnapshot> {
      const now = nowIso();

      snapshot = {
        ...snapshot,
        groups: snapshot.groups.map((group) => ({
          ...group,
          tasks: group.tasks.map((task) =>
            task.id === taskId ? { ...task, ...input, updatedAt: now } : task,
          ),
        })),
      };

      return cloneDashboard(snapshot);
    },
    async archiveTask(taskId: EntityId): Promise<TaskDashboardSnapshot> {
      snapshot = {
        ...snapshot,
        groups: snapshot.groups.map((group) => ({
          ...group,
          tasks: group.tasks.filter((task) => task.id !== taskId),
        })),
      };
      snapshot = {
        ...snapshot,
        totalOpenTasks: snapshot.groups.reduce((total, group) => total + group.tasks.length, 0),
      };

      return cloneDashboard(snapshot);
    },
  };
}

function createBrowserDevTaskRunDetailClient(): TaskRunDetailClient {
  return {
    async loadTaskRunDetail(taskId: EntityId) {
      throw new TaskRunDetailTaskNotFoundError(taskId);
    },
  };
}

function createBrowserDevRuntimeCommandClient(): RuntimeCommandClient {
  return {
    async startCodexTaskRun(
      input: StartCodexTaskRunCommandInput,
    ): Promise<StartCodexTaskRunCommandResult> {
      const now = nowIso();

      return {
        status: 'failed',
        taskId: input.taskId,
        taskRunId: `browser-runtime-unavailable-${crypto.randomUUID()}` as EntityId,
        error:
          'Browser dev mode cannot start Codex task runs. Use the Tauri desktop runtime for live execution.',
        task: {
          id: input.taskId,
          executionState: 'blocked',
          attentionState: 'needs_action_now',
          conversationIds: [],
          ...(input.worktreeId ? { worktreeId: input.worktreeId } : {}),
          updatedAt: now,
        },
        taskRun: {
          id: `browser-runtime-unavailable-${crypto.randomUUID()}` as EntityId,
          executionState: 'failed',
          ...(input.worktreeId ? { worktreeId: input.worktreeId } : {}),
          completedAt: now,
          updatedAt: now,
        },
      };
    },
    async startAgentSession(
      input: StartAgentSessionCommandInput,
    ): Promise<StartAgentSessionCommandResult> {
      const now = nowIso();

      return {
        sessionId: `browser-agent-session-${crypto.randomUUID()}` as EntityId,
        status: 'failed',
        command: 'codex',
        args: [
          'exec',
          '--json',
          ...(input.additionalArgs ?? []),
          ...(input.sessionId ? ['resume', input.sessionId] : []),
          input.prompt,
        ],
        stdout: '',
        stderr: '',
        startedAt: now,
        completedAt: now,
        error:
          'Browser dev mode cannot start Agent Session CLI instances. Use the Tauri desktop runtime for live execution.',
      };
    },
  };
}

function cloneDashboard(snapshot: TaskDashboardSnapshot): TaskDashboardSnapshot {
  return structuredClone(snapshot);
}

function upsertById<T extends { id: EntityId }>(items: T[], item: T): T[] {
  return [...items.filter((candidate) => candidate.id !== item.id), item];
}

function nowIso(): IsoDateTime {
  return new Date().toISOString() as IsoDateTime;
}

function slug(value: string): string {
  return (
    value
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, '-')
      .replace(/^-|-$/g, '') || 'item'
  );
}
