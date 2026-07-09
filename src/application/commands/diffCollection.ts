import type { ArtifactStore } from '../../domain/artifactStore';
import type { EventStore } from '../../domain/eventStore';
import type {
  Artifact,
  DomainRecords,
  EntityId,
  Event,
  Task,
  TaskRun,
  Worktree,
} from '../../domain/model';
import type { OpenTaskDashboardStore } from '../../domain/openTaskDashboardStore';
import { normalizeDomainPath } from '../../domain/repoSyncPlanning';

export interface DiffCollectionService {
  readonly dashboardStore: OpenTaskDashboardStore;
  readonly artifactStore: ArtifactStore;
  readonly eventStore: EventStore;
  readonly diffProvider: GitDiffProvider;
}

export interface GitDiffProvider {
  collectDiff(input: GitDiffProviderInput): Promise<GitDiffProviderResult>;
}

export interface GitDiffProviderInput {
  worktreePath: string;
}

export interface GitDiffProviderResult {
  diff: string;
}

export interface CollectTaskDiffInput {
  taskId: EntityId;
  taskRunId?: EntityId;
  worktreePath?: string;
  title?: string;
}

export interface CollectTaskDiffResult {
  task: Task;
  taskRun?: TaskRun;
  worktree?: Worktree;
  worktreePath: string;
  artifact: Artifact;
  event: Event;
  diff: string;
  isEmptyDiff: boolean;
}

export class DiffCollectionTaskNotFoundError extends Error {
  constructor(taskId: EntityId) {
    super(`Diff collection task not found: ${taskId}`);
    this.name = 'DiffCollectionTaskNotFoundError';
  }
}

export class DiffCollectionTaskRunNotFoundForTaskError extends Error {
  constructor(taskId: EntityId, taskRunId: EntityId) {
    super(`Diff collection task run not found for task: ${taskRunId} for ${taskId}`);
    this.name = 'DiffCollectionTaskRunNotFoundForTaskError';
  }
}

export class DiffCollectionWorktreeNotResolvedError extends Error {
  constructor(taskId: EntityId) {
    super(`Diff collection could not resolve a worktree path for task: ${taskId}`);
    this.name = 'DiffCollectionWorktreeNotResolvedError';
  }
}

export class DiffCollectionWorktreeNotFoundError extends Error {
  constructor(taskId: EntityId, worktreeId: EntityId) {
    super(`Diff collection worktree not found for task: ${worktreeId} for ${taskId}`);
    this.name = 'DiffCollectionWorktreeNotFoundError';
  }
}

interface WorktreeResolution {
  worktreePath: string;
  worktree?: Worktree;
}

export async function collectTaskDiff(
  service: DiffCollectionService,
  input: CollectTaskDiffInput,
): Promise<CollectTaskDiffResult> {
  const records = await service.dashboardStore.loadOpenTaskDashboardRecords();
  const task = requireTask(records, input.taskId);
  const taskRun = resolveTaskRun(records, task, input.taskRunId);
  const resolvedWorktree = resolveWorktree(records, task, taskRun, input.worktreePath);
  const providerResult = await service.diffProvider.collectDiff({
    worktreePath: resolvedWorktree.worktreePath,
  });
  const isEmptyDiff = providerResult.diff.length === 0;

  const artifact = await service.artifactStore.createArtifact({
    kind: 'diff',
    title: input.title ?? 'Worktree diff',
    taskId: task.id,
    ...(taskRun === undefined ? {} : { taskRunId: taskRun.id }),
    content: providerResult.diff,
  });
  const event = await service.eventStore.appendEvent({
    kind: 'artifact_created',
    projectId: task.projectId,
    taskId: task.id,
    ...(taskRun === undefined ? {} : { taskRunId: taskRun.id }),
    artifactId: artifact.id,
    payload: {
      artifactKind: artifact.kind,
      artifactId: artifact.id,
      diffLength: providerResult.diff.length,
      isEmptyDiff,
      worktreePath: normalizeDomainPath(resolvedWorktree.worktreePath),
      ...(resolvedWorktree.worktree === undefined
        ? {}
        : { worktreeId: resolvedWorktree.worktree.id }),
    },
  });

  return {
    task,
    ...(taskRun === undefined ? {} : { taskRun }),
    ...(resolvedWorktree.worktree === undefined ? {} : { worktree: resolvedWorktree.worktree }),
    worktreePath: resolvedWorktree.worktreePath,
    artifact,
    event,
    diff: providerResult.diff,
    isEmptyDiff,
  };
}

function requireTask(records: DomainRecords, taskId: EntityId): Task {
  const task = records.tasks.find((candidate) => candidate.id === taskId);

  if (task === undefined) {
    throw new DiffCollectionTaskNotFoundError(taskId);
  }

  return {
    ...task,
    conversationIds: [...task.conversationIds],
  };
}

function resolveTaskRun(
  records: DomainRecords,
  task: Task,
  taskRunId: EntityId | undefined,
): TaskRun | undefined {
  if (taskRunId === undefined) {
    return undefined;
  }

  const taskRun = records.taskRuns.find(
    (candidate) => candidate.id === taskRunId && candidate.taskId === task.id,
  );

  if (taskRun === undefined) {
    throw new DiffCollectionTaskRunNotFoundForTaskError(task.id, taskRunId);
  }

  return { ...taskRun };
}

function resolveWorktree(
  records: DomainRecords,
  task: Task,
  taskRun: TaskRun | undefined,
  explicitWorktreePath: string | undefined,
): WorktreeResolution {
  if (explicitWorktreePath !== undefined && explicitWorktreePath.trim() !== '') {
    return {
      worktreePath: explicitWorktreePath,
      ...resolveOptionalWorktreeByPath(records, explicitWorktreePath),
    };
  }

  const worktreeId = taskRun?.worktreeId ?? task.worktreeId;

  if (worktreeId === undefined) {
    throw new DiffCollectionWorktreeNotResolvedError(task.id);
  }

  const worktree = records.worktrees.find((candidate) => candidate.id === worktreeId);

  if (worktree === undefined) {
    throw new DiffCollectionWorktreeNotFoundError(task.id, worktreeId);
  }

  if (worktree.path.trim() === '') {
    throw new DiffCollectionWorktreeNotResolvedError(task.id);
  }

  return {
    worktreePath: worktree.path,
    worktree: { ...worktree },
  };
}

function resolveOptionalWorktreeByPath(
  records: DomainRecords,
  worktreePath: string,
): Pick<WorktreeResolution, 'worktree'> {
  const normalizedPath = normalizeDomainPath(worktreePath);
  const worktree = records.worktrees.find(
    (candidate) => normalizeDomainPath(candidate.path) === normalizedPath,
  );

  return worktree === undefined ? {} : { worktree: { ...worktree } };
}