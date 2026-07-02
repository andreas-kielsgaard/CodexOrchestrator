import type { EntityId, ExecutionState, IsoDateTime, TaskRun } from './model';

export interface TaskRunStoreIdProvider {
  nextId(): EntityId;
}

export interface TaskRunStoreTimeProvider {
  now(): IsoDateTime;
}

export interface CreateTaskRunInput {
  taskId: EntityId;
  executionState: ExecutionState;
  conversationId?: EntityId;
  worktreeId?: EntityId;
  startedAt?: IsoDateTime;
  completedAt?: IsoDateTime;
  exitCode?: number;
}

export interface UpdateTaskRunInput {
  conversationId?: EntityId | null;
  worktreeId?: EntityId | null;
  executionState?: ExecutionState;
  startedAt?: IsoDateTime | null;
  completedAt?: IsoDateTime | null;
  exitCode?: number | null;
}

export interface TaskRunQuery {
  taskId?: EntityId;
  conversationId?: EntityId;
  worktreeId?: EntityId;
  executionState?: ExecutionState;
  limit?: number;
}

export interface TaskRunStore {
  createTaskRun(input: CreateTaskRunInput): Promise<TaskRun>;
  updateTaskRun(taskRunId: EntityId, input: UpdateTaskRunInput): Promise<TaskRun>;
  queryTaskRuns(query?: TaskRunQuery): Promise<TaskRun[]>;
}

export class TaskRunNotFoundError extends Error {
  constructor(taskRunId: EntityId) {
    super(`Task run not found: ${taskRunId}`);
    this.name = 'TaskRunNotFoundError';
  }
}

export class InMemoryTaskRunStore implements TaskRunStore {
  private taskRuns: TaskRun[];

  constructor(
    private readonly ids: TaskRunStoreIdProvider,
    private readonly clock: TaskRunStoreTimeProvider,
    taskRuns: readonly TaskRun[] = [],
  ) {
    this.taskRuns = taskRuns.map(cloneTaskRun);
  }

  async createTaskRun(input: CreateTaskRunInput): Promise<TaskRun> {
    const now = this.clock.now();
    const taskRun = createTaskRunRecord(input, this.ids.nextId(), now);

    this.taskRuns = [...this.taskRuns, taskRun];

    return cloneTaskRun(taskRun);
  }

  async updateTaskRun(taskRunId: EntityId, input: UpdateTaskRunInput): Promise<TaskRun> {
    const taskRunIndex = this.taskRuns.findIndex((taskRun) => taskRun.id === taskRunId);

    if (taskRunIndex === -1) {
      throw new TaskRunNotFoundError(taskRunId);
    }

    const updatedTaskRun = applyTaskRunUpdate(this.taskRuns[taskRunIndex], input, this.clock.now());

    this.taskRuns = this.taskRuns.map((taskRun, index) =>
      index === taskRunIndex ? updatedTaskRun : taskRun,
    );

    return cloneTaskRun(updatedTaskRun);
  }

  async queryTaskRuns(query: TaskRunQuery = {}): Promise<TaskRun[]> {
    return queryStoredTaskRuns(this.taskRuns, query).map(cloneTaskRun);
  }

  snapshot(): TaskRun[] {
    return this.taskRuns.map(cloneTaskRun);
  }
}

export function createTaskRunRecord(
  input: CreateTaskRunInput,
  id: EntityId,
  now: IsoDateTime,
): TaskRun {
  return {
    id,
    taskId: input.taskId,
    ...(input.conversationId === undefined ? {} : { conversationId: input.conversationId }),
    ...(input.worktreeId === undefined ? {} : { worktreeId: input.worktreeId }),
    executionState: input.executionState,
    ...(input.startedAt === undefined ? {} : { startedAt: input.startedAt }),
    ...(input.completedAt === undefined ? {} : { completedAt: input.completedAt }),
    ...(input.exitCode === undefined ? {} : { exitCode: input.exitCode }),
    createdAt: now,
    updatedAt: now,
  };
}

export function applyTaskRunUpdate(
  taskRun: TaskRun,
  input: UpdateTaskRunInput,
  updatedAt: IsoDateTime,
): TaskRun {
  const updatedTaskRun: TaskRun = {
    ...taskRun,
    ...(input.executionState === undefined ? {} : { executionState: input.executionState }),
    updatedAt,
  };

  applyOptionalField(updatedTaskRun, 'conversationId', input.conversationId);
  applyOptionalField(updatedTaskRun, 'worktreeId', input.worktreeId);
  applyOptionalField(updatedTaskRun, 'startedAt', input.startedAt);
  applyOptionalField(updatedTaskRun, 'completedAt', input.completedAt);
  applyOptionalField(updatedTaskRun, 'exitCode', input.exitCode);

  return updatedTaskRun;
}

export function queryStoredTaskRuns(
  taskRuns: readonly TaskRun[],
  query: TaskRunQuery = {},
): TaskRun[] {
  assertValidLimit(query.limit);

  const queriedTaskRuns = taskRuns
    .filter((taskRun) => taskRunMatchesQuery(taskRun, query))
    .sort(compareTaskRunsChronologically);

  return query.limit === undefined ? queriedTaskRuns : queriedTaskRuns.slice(0, query.limit);
}

export function cloneTaskRun(taskRun: TaskRun): TaskRun {
  return { ...taskRun };
}

function taskRunMatchesQuery(taskRun: TaskRun, query: TaskRunQuery): boolean {
  return (
    matchesOptionalFilter(taskRun.taskId, query.taskId) &&
    matchesOptionalFilter(taskRun.conversationId, query.conversationId) &&
    matchesOptionalFilter(taskRun.worktreeId, query.worktreeId) &&
    matchesOptionalFilter(taskRun.executionState, query.executionState)
  );
}

function matchesOptionalFilter<T>(value: T | undefined, filter: T | undefined): boolean {
  return filter === undefined || value === filter;
}

function compareTaskRunsChronologically(left: TaskRun, right: TaskRun): number {
  const createdAtComparison = left.createdAt.localeCompare(right.createdAt);

  if (createdAtComparison !== 0) {
    return createdAtComparison;
  }

  return left.id.localeCompare(right.id);
}

function applyOptionalField<T extends keyof TaskRun>(
  taskRun: TaskRun,
  field: T,
  value: TaskRun[T] | null | undefined,
): void {
  if (value === undefined) {
    return;
  }

  if (value === null) {
    delete taskRun[field];
    return;
  }

  taskRun[field] = value;
}

function assertValidLimit(limit: number | undefined): void {
  if (limit === undefined) {
    return;
  }

  if (!Number.isInteger(limit) || limit < 0) {
    throw new Error(`TaskRun query limit must be a non-negative integer: ${limit}`);
  }
}
