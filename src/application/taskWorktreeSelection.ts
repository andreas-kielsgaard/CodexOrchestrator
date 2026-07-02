import type { Branch, EntityId, IsoDateTime, Repo, Task, Worktree } from '../domain/model';
import type { OpenTaskDashboardStore } from '../domain/openTaskDashboardStore';
import type { OpenTaskWriteStore } from '../domain/openTaskWriteStore';
import { normalizeDomainPath } from '../domain/repoSyncPlanning';
import {
  registerAndScanRepo,
  type RegisterAndScanRepoResult,
  type RepoRegistryScanService,
  type RepoRegistryScanSummary,
  type RepoRegistrySyncSummary,
} from './repoRegistryScan';

export interface TaskWorktreeSelectionService {
  dashboardStore: OpenTaskDashboardStore;
  taskWriteStore: OpenTaskWriteStore;
  repoRegistry: RepoRegistryScanService;
  worktreeCreator?: GitWorktreeCreator;
}

export interface GitWorktreeCreator {
  createWorktree(input: GitCreateWorktreeInput): Promise<GitCreateWorktreeResult>;
}

export interface GitCreateWorktreeInput {
  repoRootPath: string;
  worktreePath: string;
  branchName: string;
  baseBranch?: string;
}

export interface GitCreateWorktreeResult {
  repoRootPath: string;
  worktreePath: string;
  branchName: string;
  baseBranch?: string;
}

export interface SelectOrCreateTaskWorktreeInput {
  taskId: EntityId;
  repoRootPath: string;
  defaultBranch?: string;
  scannedAt?: IsoDateTime;
  worktree: TaskWorktreeRequest;
}

export type TaskWorktreeRequest =
  | {
      mode: 'select';
      worktreePath?: string;
      branchName?: string;
    }
  | {
      mode: 'create';
      worktreePath: string;
      branchName: string;
      baseBranch?: string;
    };

export interface SelectOrCreateTaskWorktreeResult {
  task: Task;
  repo: Repo;
  branch?: Branch;
  worktree: Worktree;
  scan: RepoRegistryScanSummary;
  sync: RepoRegistrySyncSummary;
  creation?: GitCreateWorktreeResult;
}

export class TaskWorktreeTaskNotFoundError extends Error {
  constructor(taskId: EntityId) {
    super(`Task not found before worktree selection: ${taskId}`);
    this.name = 'TaskWorktreeTaskNotFoundError';
  }
}

export class TaskWorktreeCreationUnavailableError extends Error {
  constructor() {
    super('Task worktree creation requires an injected GitWorktreeCreator');
    this.name = 'TaskWorktreeCreationUnavailableError';
  }
}

export class TaskWorktreeSelectionTargetRequiredError extends Error {
  constructor() {
    super('Task worktree selection requires a worktreePath, branchName, or create request');
    this.name = 'TaskWorktreeSelectionTargetRequiredError';
  }
}

export class TaskWorktreeNotFoundAfterScanError extends Error {
  constructor(target: TaskWorktreeMatchTarget, repoRootPath: string) {
    super(
      `Scanned repo did not contain requested worktree for ${describeTarget(target)} under ${normalizeDomainPath(
        repoRootPath,
      )}`,
    );
    this.name = 'TaskWorktreeNotFoundAfterScanError';
  }
}

interface TaskWorktreeMatchTarget {
  worktreePath?: string;
  branchName?: string;
}

interface SelectedTaskWorktree {
  worktree: Worktree;
  branch?: Branch;
}

export async function selectOrCreateTaskWorktree(
  service: TaskWorktreeSelectionService,
  input: SelectOrCreateTaskWorktreeInput,
): Promise<SelectOrCreateTaskWorktreeResult> {
  const task = await requireTask(service.dashboardStore, input.taskId);
  const creation = await createWorktreeIfRequested(service, input);
  const target = targetFromRequest(input.worktree, creation);
  const repoRootPath = creation?.repoRootPath ?? input.repoRootPath;
  const registryResult = await registerAndScanRepo(service.repoRegistry, {
    projectId: task.projectId,
    rootPath: repoRootPath,
    ...(input.defaultBranch === undefined ? {} : { defaultBranch: input.defaultBranch }),
    ...(input.scannedAt === undefined ? {} : { scannedAt: input.scannedAt }),
  });
  const selected = selectScannedWorktree(registryResult, target);

  if (selected === undefined) {
    throw new TaskWorktreeNotFoundAfterScanError(target, repoRootPath);
  }

  const updatedTask = await service.taskWriteStore.updateTask(input.taskId, {
    repoId: registryResult.repo.id,
    branchId: selected.branch?.id ?? null,
    worktreeId: selected.worktree.id,
  });

  return {
    task: updatedTask,
    repo: registryResult.repo,
    ...(selected.branch === undefined ? {} : { branch: selected.branch }),
    worktree: selected.worktree,
    scan: registryResult.scan,
    sync: registryResult.sync,
    ...(creation === undefined ? {} : { creation }),
  };
}

async function requireTask(store: OpenTaskDashboardStore, taskId: EntityId): Promise<Task> {
  const records = await store.loadOpenTaskDashboardRecords();
  const task = records.tasks.find((candidate) => candidate.id === taskId);

  if (task === undefined) {
    throw new TaskWorktreeTaskNotFoundError(taskId);
  }

  return task;
}

async function createWorktreeIfRequested(
  service: TaskWorktreeSelectionService,
  input: SelectOrCreateTaskWorktreeInput,
): Promise<GitCreateWorktreeResult | undefined> {
  if (input.worktree.mode !== 'create') {
    return undefined;
  }

  if (service.worktreeCreator === undefined) {
    throw new TaskWorktreeCreationUnavailableError();
  }

  return service.worktreeCreator.createWorktree({
    repoRootPath: input.repoRootPath,
    worktreePath: input.worktree.worktreePath,
    branchName: input.worktree.branchName,
    ...(input.worktree.baseBranch === undefined ? {} : { baseBranch: input.worktree.baseBranch }),
  });
}

function targetFromRequest(
  request: TaskWorktreeRequest,
  creation: GitCreateWorktreeResult | undefined,
): TaskWorktreeMatchTarget {
  const target =
    request.mode === 'create'
      ? {
          worktreePath: creation?.worktreePath ?? request.worktreePath,
          branchName: creation?.branchName ?? request.branchName,
        }
      : {
          ...(request.worktreePath === undefined ? {} : { worktreePath: request.worktreePath }),
          ...(request.branchName === undefined ? {} : { branchName: request.branchName }),
        };

  if (target.worktreePath === undefined && target.branchName === undefined) {
    throw new TaskWorktreeSelectionTargetRequiredError();
  }

  return target;
}

function selectScannedWorktree(
  registryResult: RegisterAndScanRepoResult,
  target: TaskWorktreeMatchTarget,
): SelectedTaskWorktree | undefined {
  const normalizedTargetPath =
    target.worktreePath === undefined ? undefined : normalizeDomainPath(target.worktreePath);
  const branch =
    target.branchName === undefined
      ? undefined
      : registryResult.branches.find((candidate) => candidate.name === target.branchName);

  if (target.branchName !== undefined && branch === undefined) {
    return undefined;
  }

  const worktree = registryResult.worktrees.find((candidate) => {
    if (candidate.repoId !== registryResult.repo.id) {
      return false;
    }

    if (
      normalizedTargetPath !== undefined &&
      normalizeDomainPath(candidate.path) !== normalizedTargetPath
    ) {
      return false;
    }

    if (target.branchName !== undefined && candidate.branchId !== branch?.id) {
      return false;
    }

    return true;
  });

  if (worktree === undefined) {
    return undefined;
  }

  const selectedBranch =
    worktree.branchId === undefined
      ? undefined
      : registryResult.branches.find((candidate) => candidate.id === worktree.branchId);

  return {
    worktree,
    ...(selectedBranch === undefined ? {} : { branch: selectedBranch }),
  };
}

function describeTarget(target: TaskWorktreeMatchTarget): string {
  const parts = [
    target.worktreePath === undefined
      ? undefined
      : `path ${normalizeDomainPath(target.worktreePath)}`,
    target.branchName === undefined ? undefined : `branch ${target.branchName}`,
  ].filter((part): part is string => part !== undefined);

  return parts.join(' and ');
}
