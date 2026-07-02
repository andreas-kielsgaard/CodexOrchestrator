import type { ValidationRun } from './model';
import { InMemoryValidationRunStore, ValidationRunNotFoundError } from './validationRunStore';

const now = '2026-07-02T10:00:00.000Z';
const updatedAt = '2026-07-02T10:05:00.000Z';
const createdAt = '2026-07-01T09:00:00.000Z';

describe('InMemoryValidationRunStore', () => {
  it('creates validation runs with deterministic ids, timestamps, and optional links/output', async () => {
    const store = createStore();

    await expect(
      store.createValidationRun({
        taskId: 'task-1',
        taskRunId: 'run-1',
        command: 'npm run test',
        status: 'running',
        startedAt: '2026-07-02T09:59:00.000Z',
        exitCode: 0,
        outputArtifactId: 'artifact-1',
      }),
    ).resolves.toEqual({
      id: 'validation-created',
      taskId: 'task-1',
      taskRunId: 'run-1',
      command: 'npm run test',
      status: 'running',
      startedAt: '2026-07-02T09:59:00.000Z',
      exitCode: 0,
      outputArtifactId: 'artifact-1',
      createdAt: now,
      updatedAt: now,
    });
  });

  it('does not invent optional defaults when creating a validation run', async () => {
    const store = createStore();

    await expect(
      store.createValidationRun({
        command: 'npm run lint',
        status: 'queued',
      }),
    ).resolves.toEqual({
      id: 'validation-created',
      command: 'npm run lint',
      status: 'queued',
      createdAt: now,
      updatedAt: now,
    });
  });

  it('updates mutable fields while keeping id, command, and createdAt immutable', async () => {
    const store = createStore([validationRun()], [updatedAt]);

    const updated = await store.updateValidationRun('validation-1', {
      status: 'passed',
      completedAt: '2026-07-02T10:04:00.000Z',
      exitCode: 0,
    });

    expect(updated).toEqual({
      ...validationRun(),
      status: 'passed',
      completedAt: '2026-07-02T10:04:00.000Z',
      exitCode: 0,
      updatedAt,
    });
    expect(updated.id).toBe('validation-1');
    expect(updated.command).toBe('npm run test');
    expect(updated.createdAt).toBe(createdAt);
  });

  it('leaves omitted optional fields unchanged and explicitly clears null optional fields', async () => {
    const store = createStore([validationRun()], [updatedAt, updatedAt]);

    const unchanged = await store.updateValidationRun('validation-1', {
      status: 'canceled',
    });

    expect(unchanged).toMatchObject({
      taskId: 'task-1',
      taskRunId: 'run-1',
      startedAt: '2026-07-02T09:58:00.000Z',
      completedAt: '2026-07-02T10:03:00.000Z',
      exitCode: 1,
      outputArtifactId: 'artifact-1',
    });

    const cleared = await store.updateValidationRun('validation-1', {
      taskId: null,
      taskRunId: null,
      startedAt: null,
      completedAt: null,
      exitCode: null,
      outputArtifactId: null,
    });

    expect(cleared).not.toHaveProperty('taskId');
    expect(cleared).not.toHaveProperty('taskRunId');
    expect(cleared).not.toHaveProperty('startedAt');
    expect(cleared).not.toHaveProperty('completedAt');
    expect(cleared).not.toHaveProperty('exitCode');
    expect(cleared).not.toHaveProperty('outputArtifactId');
  });

  it('throws a typed error when updating a missing validation run', async () => {
    const store = createStore();

    await expect(
      store.updateValidationRun('validation-missing', {
        status: 'failed',
      }),
    ).rejects.toThrow(ValidationRunNotFoundError);
  });

  it('queries by optional filters in created order with id tie-breakers and limits', async () => {
    const store = createStore([
      validationRun({
        id: 'validation-b',
        status: 'running',
        createdAt: '2026-07-02T10:00:01.000Z',
      }),
      validationRun({
        id: 'validation-a',
        status: 'running',
        createdAt: '2026-07-02T10:00:01.000Z',
      }),
      validationRun({
        id: 'validation-c',
        status: 'passed',
        createdAt: '2026-07-02T09:59:59.000Z',
      }),
      validationRun({
        id: 'validation-d',
        taskId: 'task-2',
        taskRunId: 'run-2',
        status: 'running',
        outputArtifactId: 'artifact-2',
        createdAt: '2026-07-02T10:00:02.000Z',
      }),
    ]);

    await expect(
      store.queryValidationRuns({
        taskId: 'task-1',
        taskRunId: 'run-1',
        status: 'running',
        outputArtifactId: 'artifact-1',
        limit: 1,
      }),
    ).resolves.toEqual([
      validationRun({
        id: 'validation-a',
        status: 'running',
        createdAt: '2026-07-02T10:00:01.000Z',
      }),
    ]);
  });

  it('returns empty query results, supports limit zero, and rejects invalid limits', async () => {
    const store = createStore([validationRun()]);

    await expect(store.queryValidationRuns({ taskId: 'missing-task' })).resolves.toEqual([]);
    await expect(store.queryValidationRuns({ limit: 0 })).resolves.toEqual([]);
    await expect(store.queryValidationRuns({ limit: -1 })).rejects.toThrow(
      'ValidationRun query limit must be a non-negative integer',
    );
    await expect(store.queryValidationRuns({ limit: 1.5 })).rejects.toThrow(
      'ValidationRun query limit must be a non-negative integer',
    );
  });

  it('returns cloned query results so callers cannot mutate stored validation runs', async () => {
    const store = createStore([validationRun()]);
    const [firstLoad] = await store.queryValidationRuns();

    firstLoad.status = 'passed';

    await expect(store.queryValidationRuns()).resolves.toEqual([validationRun()]);
  });
});

function createStore(
  validationRuns: readonly ValidationRun[] = [],
  times: readonly string[] = [now],
): InMemoryValidationRunStore {
  let callCount = 0;

  return new InMemoryValidationRunStore(
    {
      nextId: () => 'validation-created',
    },
    {
      now: () => {
        const time = times[callCount] ?? times[times.length - 1];
        callCount += 1;
        return time;
      },
    },
    validationRuns,
  );
}

function validationRun(overrides: Partial<ValidationRun> = {}): ValidationRun {
  return {
    id: 'validation-1',
    taskId: 'task-1',
    taskRunId: 'run-1',
    command: 'npm run test',
    status: 'failed',
    startedAt: '2026-07-02T09:58:00.000Z',
    completedAt: '2026-07-02T10:03:00.000Z',
    exitCode: 1,
    outputArtifactId: 'artifact-1',
    createdAt,
    updatedAt: createdAt,
    ...overrides,
  };
}
