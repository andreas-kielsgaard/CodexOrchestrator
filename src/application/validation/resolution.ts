import type { DomainRecords, EntityId, Task, TaskRun, Worktree } from '../../domain/model';
import type { RunTaskValidationCommandInput, ResolvedValidationCwd } from './types';

export class ValidationCommandTaskNotFoundError extends Error {
  constructor(taskId: EntityId) {
    super(`Task not found before validation command run: ${taskId}`);
    this.name = 'ValidationCommandTaskNotFoundError';
  }
}

export class ValidationCommandTaskRunNotFoundForTaskError extends Error {
  constructor(taskId: EntityId, taskRunId: EntityId) {
    super(`Validation command task run not found for task: ${taskRunId} for ${taskId}`);
    this.name = 'ValidationCommandTaskRunNotFoundForTaskError';
  }
}

export class ValidationCommandWorktreeRequiredError extends Error {
  constructor(taskId: EntityId) {
    super(`Validation command run requires a cwd or linked worktree path for task: ${taskId}`);
    this.name = 'ValidationCommandWorktreeRequiredError';
  }
}

export class ValidationCommandWorktreeNotFoundError extends Error {
  constructor(worktreeId: EntityId, taskId: EntityId) {
    super(`Validation command worktree not found for task ${taskId}: ${worktreeId}`);
    this.name = 'ValidationCommandWorktreeNotFoundError';
  }
}

export function requireTask(records: DomainRecords, taskId: EntityId): Task {
  const task = records.tasks.find((candidate) => candidate.id === taskId);

  if (task === undefined) {
    throw new ValidationCommandTaskNotFoundError(taskId);
  }

  return task;
}

export function resolveTaskRun(
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
    throw new ValidationCommandTaskRunNotFoundForTaskError(task.id, taskRunId);
  }

  return taskRun;
}

export function resolveValidationCwd(
  records: DomainRecords,
  task: Task,
  input: RunTaskValidationCommandInput,
): ResolvedValidationCwd {
  const worktreeId = input.worktreeId ?? task.worktreeId;

  if (input.cwd !== undefined) {
    if (input.worktreeId !== undefined) {
      return {
        cwd: input.cwd,
        worktree: requireWorktree(records, input.worktreeId, task.id),
      };
    }

    const linkedWorktree =
      task.worktreeId === undefined ? undefined : findWorktree(records, task.worktreeId);

    return {
      cwd: input.cwd,
      ...(linkedWorktree === undefined ? {} : { worktree: linkedWorktree }),
    };
  }

  if (worktreeId === undefined) {
    throw new ValidationCommandWorktreeRequiredError(task.id);
  }

  const worktree = requireWorktree(records, worktreeId, task.id);

  return {
    cwd: worktree.path,
    worktree,
  };
}

function findWorktree(records: DomainRecords, worktreeId: EntityId): Worktree | undefined {
  return records.worktrees.find((candidate) => candidate.id === worktreeId);
}

function requireWorktree(records: DomainRecords, worktreeId: EntityId, taskId: EntityId): Worktree {
  const worktree = findWorktree(records, worktreeId);

  if (worktree === undefined) {
    throw new ValidationCommandWorktreeNotFoundError(worktreeId, taskId);
  }

  return worktree;
}
