import type { EntityId, IsoDateTime, ValidationRun, ValidationStatus } from './model';

export interface ValidationRunStoreIdProvider {
  nextId(): EntityId;
}

export interface ValidationRunStoreTimeProvider {
  now(): IsoDateTime;
}

export interface CreateValidationRunInput {
  command: string;
  status: ValidationStatus;
  taskId?: EntityId;
  taskRunId?: EntityId;
  startedAt?: IsoDateTime;
  completedAt?: IsoDateTime;
  exitCode?: number;
  outputArtifactId?: EntityId;
}

export interface UpdateValidationRunInput {
  taskId?: EntityId | null;
  taskRunId?: EntityId | null;
  status?: ValidationStatus;
  startedAt?: IsoDateTime | null;
  completedAt?: IsoDateTime | null;
  exitCode?: number | null;
  outputArtifactId?: EntityId | null;
}

export interface ValidationRunQuery {
  taskId?: EntityId;
  taskRunId?: EntityId;
  status?: ValidationStatus;
  outputArtifactId?: EntityId;
  limit?: number;
}

export interface ValidationRunStore {
  createValidationRun(input: CreateValidationRunInput): Promise<ValidationRun>;
  updateValidationRun(
    validationRunId: EntityId,
    input: UpdateValidationRunInput,
  ): Promise<ValidationRun>;
  queryValidationRuns(query?: ValidationRunQuery): Promise<ValidationRun[]>;
}

export class ValidationRunNotFoundError extends Error {
  constructor(validationRunId: EntityId) {
    super(`Validation run not found: ${validationRunId}`);
    this.name = 'ValidationRunNotFoundError';
  }
}

export class InMemoryValidationRunStore implements ValidationRunStore {
  private validationRuns: ValidationRun[];

  constructor(
    private readonly ids: ValidationRunStoreIdProvider,
    private readonly clock: ValidationRunStoreTimeProvider,
    validationRuns: readonly ValidationRun[] = [],
  ) {
    this.validationRuns = validationRuns.map(cloneValidationRun);
  }

  async createValidationRun(input: CreateValidationRunInput): Promise<ValidationRun> {
    const now = this.clock.now();
    const validationRun = createValidationRunRecord(input, this.ids.nextId(), now);

    this.validationRuns = [...this.validationRuns, validationRun];

    return cloneValidationRun(validationRun);
  }

  async updateValidationRun(
    validationRunId: EntityId,
    input: UpdateValidationRunInput,
  ): Promise<ValidationRun> {
    const validationRunIndex = this.validationRuns.findIndex(
      (validationRun) => validationRun.id === validationRunId,
    );

    if (validationRunIndex === -1) {
      throw new ValidationRunNotFoundError(validationRunId);
    }

    const updatedValidationRun = applyValidationRunUpdate(
      this.validationRuns[validationRunIndex],
      input,
      this.clock.now(),
    );

    this.validationRuns = this.validationRuns.map((validationRun, index) =>
      index === validationRunIndex ? updatedValidationRun : validationRun,
    );

    return cloneValidationRun(updatedValidationRun);
  }

  async queryValidationRuns(query: ValidationRunQuery = {}): Promise<ValidationRun[]> {
    return queryStoredValidationRuns(this.validationRuns, query).map(cloneValidationRun);
  }

  snapshot(): ValidationRun[] {
    return this.validationRuns.map(cloneValidationRun);
  }
}

export function createValidationRunRecord(
  input: CreateValidationRunInput,
  id: EntityId,
  now: IsoDateTime,
): ValidationRun {
  return {
    id,
    ...(input.taskId === undefined ? {} : { taskId: input.taskId }),
    ...(input.taskRunId === undefined ? {} : { taskRunId: input.taskRunId }),
    command: input.command,
    status: input.status,
    ...(input.startedAt === undefined ? {} : { startedAt: input.startedAt }),
    ...(input.completedAt === undefined ? {} : { completedAt: input.completedAt }),
    ...(input.exitCode === undefined ? {} : { exitCode: input.exitCode }),
    ...(input.outputArtifactId === undefined ? {} : { outputArtifactId: input.outputArtifactId }),
    createdAt: now,
    updatedAt: now,
  };
}

export function applyValidationRunUpdate(
  validationRun: ValidationRun,
  input: UpdateValidationRunInput,
  updatedAt: IsoDateTime,
): ValidationRun {
  const updatedValidationRun: ValidationRun = {
    ...validationRun,
    ...(input.status === undefined ? {} : { status: input.status }),
    updatedAt,
  };

  applyOptionalField(updatedValidationRun, 'taskId', input.taskId);
  applyOptionalField(updatedValidationRun, 'taskRunId', input.taskRunId);
  applyOptionalField(updatedValidationRun, 'startedAt', input.startedAt);
  applyOptionalField(updatedValidationRun, 'completedAt', input.completedAt);
  applyOptionalField(updatedValidationRun, 'exitCode', input.exitCode);
  applyOptionalField(updatedValidationRun, 'outputArtifactId', input.outputArtifactId);

  return updatedValidationRun;
}

export function queryStoredValidationRuns(
  validationRuns: readonly ValidationRun[],
  query: ValidationRunQuery = {},
): ValidationRun[] {
  assertValidLimit(query.limit);

  const queriedValidationRuns = validationRuns
    .filter((validationRun) => validationRunMatchesQuery(validationRun, query))
    .sort(compareValidationRunsChronologically);

  return query.limit === undefined
    ? queriedValidationRuns
    : queriedValidationRuns.slice(0, query.limit);
}

export function cloneValidationRun(validationRun: ValidationRun): ValidationRun {
  return { ...validationRun };
}

function validationRunMatchesQuery(
  validationRun: ValidationRun,
  query: ValidationRunQuery,
): boolean {
  return (
    matchesOptionalFilter(validationRun.taskId, query.taskId) &&
    matchesOptionalFilter(validationRun.taskRunId, query.taskRunId) &&
    matchesOptionalFilter(validationRun.status, query.status) &&
    matchesOptionalFilter(validationRun.outputArtifactId, query.outputArtifactId)
  );
}

function matchesOptionalFilter<T>(value: T | undefined, filter: T | undefined): boolean {
  return filter === undefined || value === filter;
}

function compareValidationRunsChronologically(left: ValidationRun, right: ValidationRun): number {
  const createdAtComparison = left.createdAt.localeCompare(right.createdAt);

  if (createdAtComparison !== 0) {
    return createdAtComparison;
  }

  return left.id.localeCompare(right.id);
}

function applyOptionalField<T extends keyof ValidationRun>(
  validationRun: ValidationRun,
  field: T,
  value: ValidationRun[T] | null | undefined,
): void {
  if (value === undefined) {
    return;
  }

  if (value === null) {
    delete validationRun[field];
    return;
  }

  validationRun[field] = value;
}

function assertValidLimit(limit: number | undefined): void {
  if (limit === undefined) {
    return;
  }

  if (!Number.isInteger(limit) || limit < 0) {
    throw new Error(`ValidationRun query limit must be a non-negative integer: ${limit}`);
  }
}
