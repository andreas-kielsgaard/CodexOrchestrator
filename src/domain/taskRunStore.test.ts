import type { TaskRun } from './model';
import { InMemoryTaskRunStore, TaskRunNotFoundError } from './taskRunStore';

const now = '2026-07-02T10:00:00.000Z';
const updatedAt = '2026-07-02T10:05:00.000Z';
const createdAt = '2026-07-01T09:00:00.000Z';

describe('InMemoryTaskRunStore', () => {
  it('creates task runs with deterministic ids, timestamps, and optional links', async () => {
    const store = createStore();

    await expect(
      store.createTaskRun({
        taskId: 'task-1',
        conversationId: 'conversation-1',
        worktreeId: 'worktree-1',
        executionState: 'running',
        startedAt: '2026-07-02T09:59:00.000Z',
        exitCode: 0,
      }),
    ).resolves.toEqual({
      id: 'run-created',
      taskId: 'task-1',
      conversationId: 'conversation-1',
      worktreeId: 'worktree-1',
      executionState: 'running',
      startedAt: '2026-07-02T09:59:00.000Z',
      exitCode: 0,
      createdAt: now,
      updatedAt: now,
    });
  });

  it('updates mutable fields while keeping taskId and createdAt immutable', async () => {
    const store = createStore([taskRun()], [updatedAt]);

    const updated = await store.updateTaskRun('run-1', {
      executionState: 'completed',
      completedAt: '2026-07-02T10:04:00.000Z',
      exitCode: 0,
    });

    expect(updated).toEqual({
      ...taskRun(),
      executionState: 'completed',
      completedAt: '2026-07-02T10:04:00.000Z',
      exitCode: 0,
      updatedAt,
    });
    expect(updated.taskId).toBe('task-1');
    expect(updated.createdAt).toBe(createdAt);
  });

  it('leaves omitted optional fields unchanged and explicitly clears null optional fields', async () => {
    const store = createStore([taskRun()], [updatedAt, updatedAt]);

    const unchanged = await store.updateTaskRun('run-1', {
      executionState: 'blocked',
    });

    expect(unchanged).toMatchObject({
      conversationId: 'conversation-1',
      worktreeId: 'worktree-1',
      startedAt: '2026-07-02T09:58:00.000Z',
      completedAt: '2026-07-02T10:03:00.000Z',
      exitCode: 1,
    });

    const cleared = await store.updateTaskRun('run-1', {
      conversationId: null,
      worktreeId: null,
      startedAt: null,
      completedAt: null,
      exitCode: null,
    });

    expect(cleared).not.toHaveProperty('conversationId');
    expect(cleared).not.toHaveProperty('worktreeId');
    expect(cleared).not.toHaveProperty('startedAt');
    expect(cleared).not.toHaveProperty('completedAt');
    expect(cleared).not.toHaveProperty('exitCode');
  });

  it('throws a typed error when updating a missing task run', async () => {
    const store = createStore();

    await expect(
      store.updateTaskRun('run-missing', {
        executionState: 'failed',
      }),
    ).rejects.toThrow(TaskRunNotFoundError);
  });

  it('queries by optional filters in created order with id tie-breakers and limits', async () => {
    const store = createStore([
      taskRun({
        id: 'run-b',
        taskId: 'task-1',
        executionState: 'running',
        createdAt: '2026-07-02T10:00:01.000Z',
      }),
      taskRun({
        id: 'run-a',
        taskId: 'task-1',
        executionState: 'running',
        createdAt: '2026-07-02T10:00:01.000Z',
      }),
      taskRun({
        id: 'run-c',
        taskId: 'task-1',
        executionState: 'completed',
        createdAt: '2026-07-02T09:59:59.000Z',
      }),
      taskRun({
        id: 'run-d',
        taskId: 'task-2',
        conversationId: 'conversation-2',
        executionState: 'running',
        createdAt: '2026-07-02T10:00:02.000Z',
      }),
    ]);

    await expect(
      store.queryTaskRuns({
        taskId: 'task-1',
        conversationId: 'conversation-1',
        worktreeId: 'worktree-1',
        executionState: 'running',
        limit: 1,
      }),
    ).resolves.toEqual([
      taskRun({
        id: 'run-a',
        taskId: 'task-1',
        executionState: 'running',
        createdAt: '2026-07-02T10:00:01.000Z',
      }),
    ]);
  });

  it('returns empty query results, supports limit zero, and rejects invalid limits', async () => {
    const store = createStore([taskRun()]);

    await expect(store.queryTaskRuns({ taskId: 'missing-task' })).resolves.toEqual([]);
    await expect(store.queryTaskRuns({ limit: 0 })).resolves.toEqual([]);
    await expect(store.queryTaskRuns({ limit: -1 })).rejects.toThrow(
      'TaskRun query limit must be a non-negative integer',
    );
  });

  it('returns cloned query results so callers cannot mutate stored task runs', async () => {
    const store = createStore([taskRun()]);
    const [firstLoad] = await store.queryTaskRuns();

    firstLoad.executionState = 'failed';

    await expect(store.queryTaskRuns()).resolves.toEqual([taskRun()]);
  });
});

function createStore(
  taskRuns: readonly TaskRun[] = [],
  times: readonly string[] = [now],
): InMemoryTaskRunStore {
  let callCount = 0;

  return new InMemoryTaskRunStore(
    {
      nextId: () => 'run-created',
    },
    {
      now: () => {
        const time = times[callCount] ?? times[times.length - 1];
        callCount += 1;
        return time;
      },
    },
    taskRuns,
  );
}

function taskRun(overrides: Partial<TaskRun> = {}): TaskRun {
  return {
    id: 'run-1',
    taskId: 'task-1',
    conversationId: 'conversation-1',
    worktreeId: 'worktree-1',
    executionState: 'failed',
    startedAt: '2026-07-02T09:58:00.000Z',
    completedAt: '2026-07-02T10:03:00.000Z',
    exitCode: 1,
    createdAt,
    updatedAt: createdAt,
    ...overrides,
  };
}
