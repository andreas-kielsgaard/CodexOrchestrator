import type { Artifact } from './model';
import { InMemoryArtifactStore } from './artifactStore';

const now = '2026-07-02T10:00:00.000Z';
const createdAt = '2026-07-01T09:00:00.000Z';

describe('InMemoryArtifactStore', () => {
  it('creates artifacts with deterministic ids, timestamps, and optional links/content', async () => {
    const store = createStore();

    await expect(
      store.createArtifact({
        kind: 'final_response',
        title: 'Worker completion report',
        taskId: 'task-1',
        taskRunId: 'run-1',
        conversationId: 'conversation-1',
        uri: 'file:///tmp/report.md',
        content: 'Task complete.',
      }),
    ).resolves.toEqual({
      id: 'artifact-created',
      taskId: 'task-1',
      taskRunId: 'run-1',
      conversationId: 'conversation-1',
      kind: 'final_response',
      title: 'Worker completion report',
      uri: 'file:///tmp/report.md',
      content: 'Task complete.',
      createdAt: now,
    });
  });

  it('does not invent optional defaults when creating an artifact', async () => {
    const store = createStore();

    await expect(
      store.createArtifact({
        kind: 'note',
        title: 'Loose note',
      }),
    ).resolves.toEqual({
      id: 'artifact-created',
      kind: 'note',
      title: 'Loose note',
      createdAt: now,
    });
  });

  it('queries by optional filters in created order with id tie-breakers and limits', async () => {
    const store = createStore([
      artifact({
        id: 'artifact-b',
        kind: 'diff',
        createdAt: '2026-07-02T10:00:01.000Z',
      }),
      artifact({
        id: 'artifact-a',
        kind: 'diff',
        createdAt: '2026-07-02T10:00:01.000Z',
      }),
      artifact({
        id: 'artifact-c',
        kind: 'note',
        createdAt: '2026-07-02T09:59:59.000Z',
      }),
      artifact({
        id: 'artifact-d',
        taskId: 'task-2',
        taskRunId: 'run-2',
        conversationId: 'conversation-2',
        kind: 'diff',
        createdAt: '2026-07-02T10:00:02.000Z',
      }),
    ]);

    await expect(
      store.queryArtifacts({
        kind: 'diff',
        taskId: 'task-1',
        taskRunId: 'run-1',
        conversationId: 'conversation-1',
        limit: 1,
      }),
    ).resolves.toEqual([
      artifact({
        id: 'artifact-a',
        kind: 'diff',
        createdAt: '2026-07-02T10:00:01.000Z',
      }),
    ]);
  });

  it('returns empty query results, supports limit zero, and rejects invalid limits', async () => {
    const store = createStore([artifact()]);

    await expect(store.queryArtifacts({ taskId: 'missing-task' })).resolves.toEqual([]);
    await expect(store.queryArtifacts({ limit: 0 })).resolves.toEqual([]);
    await expect(store.queryArtifacts({ limit: -1 })).rejects.toThrow(
      'Artifact query limit must be a non-negative integer',
    );
  });

  it('returns cloned query results so callers cannot mutate stored artifacts', async () => {
    const store = createStore([artifact()]);
    const [firstLoad] = await store.queryArtifacts();

    firstLoad.title = 'Mutated outside the store';

    await expect(store.queryArtifacts()).resolves.toEqual([artifact()]);
  });
});

function createStore(
  artifacts: readonly Artifact[] = [],
  times: readonly string[] = [now],
): InMemoryArtifactStore {
  let callCount = 0;

  return new InMemoryArtifactStore(
    {
      nextId: () => 'artifact-created',
    },
    {
      now: () => {
        const time = times[callCount] ?? times[times.length - 1];
        callCount += 1;
        return time;
      },
    },
    artifacts,
  );
}

function artifact(overrides: Partial<Artifact> = {}): Artifact {
  return {
    id: 'artifact-1',
    taskId: 'task-1',
    taskRunId: 'run-1',
    conversationId: 'conversation-1',
    kind: 'final_response',
    title: 'Worker completion report',
    uri: 'file:///tmp/report.md',
    content: 'Task complete.',
    createdAt,
    ...overrides,
  };
}
