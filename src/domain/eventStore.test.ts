import type { Event } from './model';
import { InMemoryEventStore } from './eventStore';

const now = '2026-07-02T10:00:00.000Z';

describe('InMemoryEventStore', () => {
  it('appends events with deterministic ids, timestamps, optional links, and cloned payloads', async () => {
    const payload = {
      nested: {
        status: 'running',
      },
      sequence: 1,
    };
    const store = createStore();

    const appended = await store.appendEvent({
      kind: 'run_event',
      projectId: 'project-1',
      taskId: 'task-1',
      taskRunId: 'run-1',
      conversationId: 'conversation-1',
      artifactId: 'artifact-1',
      validationRunId: 'validation-1',
      payload,
    });

    payload.nested.status = 'mutated-after-append';

    expect(appended).toEqual({
      id: 'event-created',
      kind: 'run_event',
      occurredAt: now,
      projectId: 'project-1',
      taskId: 'task-1',
      taskRunId: 'run-1',
      conversationId: 'conversation-1',
      artifactId: 'artifact-1',
      validationRunId: 'validation-1',
      payload: {
        nested: {
          status: 'running',
        },
        sequence: 1,
      },
    });
    appended.payload.nested = { status: 'mutated-return-value' };

    expect(store.snapshot()).toEqual([
      {
        ...appended,
        payload: {
          nested: {
            status: 'running',
          },
          sequence: 1,
        },
      },
    ]);
  });

  it('queries events by kind and linked ids in chronological order with id tie-breakers', async () => {
    const store = createStore([
      event({
        id: 'event-b',
        kind: 'run_event',
        occurredAt: '2026-07-02T10:00:01.000Z',
        taskId: 'task-1',
      }),
      event({
        id: 'event-a',
        kind: 'run_event',
        occurredAt: '2026-07-02T10:00:01.000Z',
        taskId: 'task-1',
      }),
      event({
        id: 'event-c',
        kind: 'run_completed',
        occurredAt: '2026-07-02T09:59:59.000Z',
        taskId: 'task-1',
      }),
      event({
        id: 'event-d',
        kind: 'run_event',
        occurredAt: '2026-07-02T10:00:02.000Z',
        taskId: 'task-2',
      }),
    ]);

    await expect(store.queryEvents({ kind: 'run_event', taskId: 'task-1' })).resolves.toEqual([
      event({
        id: 'event-a',
        kind: 'run_event',
        occurredAt: '2026-07-02T10:00:01.000Z',
        taskId: 'task-1',
      }),
      event({
        id: 'event-b',
        kind: 'run_event',
        occurredAt: '2026-07-02T10:00:01.000Z',
        taskId: 'task-1',
      }),
    ]);
  });

  it('returns empty query results and supports simple limits', async () => {
    const store = createStore([
      event({ id: 'event-1', occurredAt: '2026-07-02T10:00:01.000Z' }),
      event({ id: 'event-2', occurredAt: '2026-07-02T10:00:02.000Z' }),
    ]);

    await expect(store.queryEvents({ artifactId: 'missing-artifact' })).resolves.toEqual([]);
    await expect(store.queryEvents({ limit: 1 })).resolves.toEqual([
      event({ id: 'event-1', occurredAt: '2026-07-02T10:00:01.000Z' }),
    ]);
    await expect(store.queryEvents({ limit: 0 })).resolves.toEqual([]);
    await expect(store.queryEvents({ limit: -1 })).rejects.toThrow(
      'Event query limit must be a non-negative integer',
    );
  });

  it('returns cloned query results so callers cannot mutate stored payloads', async () => {
    const store = createStore([event()]);
    const [firstLoad] = await store.queryEvents();

    firstLoad.payload.status = 'mutated';

    await expect(store.queryEvents()).resolves.toEqual([event()]);
  });
});

function createStore(events: readonly Event[] = []): InMemoryEventStore {
  return new InMemoryEventStore(
    {
      nextId: () => 'event-created',
    },
    {
      now: () => now,
    },
    events,
  );
}

function event(overrides: Partial<Event> = {}): Event {
  return {
    id: 'event-1',
    kind: 'run_event',
    occurredAt: now,
    taskId: 'task-1',
    payload: {
      status: 'running',
    },
    ...overrides,
  };
}
