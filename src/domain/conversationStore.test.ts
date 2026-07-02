import type { Conversation } from './model';
import { ConversationNotFoundError, InMemoryConversationStore } from './conversationStore';

const now = '2026-07-02T10:00:00.000Z';
const updatedAt = '2026-07-02T10:05:00.000Z';
const createdAt = '2026-07-01T09:00:00.000Z';

describe('InMemoryConversationStore', () => {
  it('creates conversations with deterministic ids, timestamps, and optional fields', async () => {
    const store = createStore();

    await expect(
      store.createConversation({
        taskId: 'task-1',
        taskRunId: 'run-1',
        provider: 'codex',
        externalThreadId: 'thread-1',
        title: 'Worker conversation',
        summary: 'A useful worker trace.',
      }),
    ).resolves.toEqual({
      id: 'conversation-created',
      taskId: 'task-1',
      taskRunId: 'run-1',
      provider: 'codex',
      externalThreadId: 'thread-1',
      title: 'Worker conversation',
      summary: 'A useful worker trace.',
      createdAt: now,
      updatedAt: now,
    });
  });

  it('leaves omitted create optionals unset', async () => {
    const store = createStore();

    await expect(
      store.createConversation({
        provider: 'manual',
        title: 'Manual note',
      }),
    ).resolves.toEqual({
      id: 'conversation-created',
      provider: 'manual',
      title: 'Manual note',
      createdAt: now,
      updatedAt: now,
    });
  });

  it('updates mutable fields while keeping id, provider, and createdAt immutable', async () => {
    const store = createStore([conversation()], [updatedAt]);

    const updated = await store.updateConversation('conversation-1', {
      title: 'Renamed conversation',
      summary: 'Updated summary',
    });

    expect(updated).toEqual({
      ...conversation(),
      title: 'Renamed conversation',
      summary: 'Updated summary',
      updatedAt,
    });
    expect(updated.id).toBe('conversation-1');
    expect(updated.provider).toBe('codex');
    expect(updated.createdAt).toBe(createdAt);
  });

  it('leaves omitted optional fields unchanged and explicitly clears null optional fields', async () => {
    const store = createStore([conversation()], [updatedAt, updatedAt]);

    const unchanged = await store.updateConversation('conversation-1', {
      title: 'Retitled',
    });

    expect(unchanged).toMatchObject({
      taskId: 'task-1',
      taskRunId: 'run-1',
      externalThreadId: 'thread-1',
      summary: 'Initial summary',
    });

    const cleared = await store.updateConversation('conversation-1', {
      taskId: null,
      taskRunId: null,
      externalThreadId: null,
      summary: null,
    });

    expect(cleared).not.toHaveProperty('taskId');
    expect(cleared).not.toHaveProperty('taskRunId');
    expect(cleared).not.toHaveProperty('externalThreadId');
    expect(cleared).not.toHaveProperty('summary');
  });

  it('throws a typed error when updating a missing conversation', async () => {
    const store = createStore();

    await expect(
      store.updateConversation('conversation-missing', {
        title: 'Missing',
      }),
    ).rejects.toThrow(ConversationNotFoundError);
  });

  it('queries by optional filters in created order with id tie-breakers and limits', async () => {
    const store = createStore([
      conversation({
        id: 'conversation-b',
        provider: 'codex',
        createdAt: '2026-07-02T10:00:01.000Z',
      }),
      conversation({
        id: 'conversation-a',
        provider: 'codex',
        createdAt: '2026-07-02T10:00:01.000Z',
      }),
      conversation({
        id: 'conversation-c',
        provider: 'codex',
        externalThreadId: 'thread-2',
        createdAt: '2026-07-02T09:59:59.000Z',
      }),
      conversation({
        id: 'conversation-d',
        taskId: 'task-2',
        taskRunId: 'run-2',
        provider: 'manual',
        createdAt: '2026-07-02T10:00:02.000Z',
      }),
    ]);

    await expect(
      store.queryConversations({
        provider: 'codex',
        taskId: 'task-1',
        taskRunId: 'run-1',
        externalThreadId: 'thread-1',
        limit: 1,
      }),
    ).resolves.toEqual([
      conversation({
        id: 'conversation-a',
        provider: 'codex',
        createdAt: '2026-07-02T10:00:01.000Z',
      }),
    ]);
  });

  it('returns empty query results, supports limit zero, and rejects invalid limits', async () => {
    const store = createStore([conversation()]);

    await expect(store.queryConversations({ taskId: 'missing-task' })).resolves.toEqual([]);
    await expect(store.queryConversations({ limit: 0 })).resolves.toEqual([]);
    await expect(store.queryConversations({ limit: -1 })).rejects.toThrow(
      'Conversation query limit must be a non-negative integer',
    );
    await expect(store.queryConversations({ limit: 1.5 })).rejects.toThrow(
      'Conversation query limit must be a non-negative integer',
    );
  });

  it('returns cloned query results so callers cannot mutate stored conversations', async () => {
    const store = createStore([conversation()]);
    const [firstLoad] = await store.queryConversations();

    firstLoad.title = 'Caller mutation';

    await expect(store.queryConversations()).resolves.toEqual([conversation()]);
  });
});

function createStore(
  conversations: readonly Conversation[] = [],
  times: readonly string[] = [now],
): InMemoryConversationStore {
  let callCount = 0;

  return new InMemoryConversationStore(
    {
      nextId: () => 'conversation-created',
    },
    {
      now: () => {
        const time = times[callCount] ?? times[times.length - 1];
        callCount += 1;
        return time;
      },
    },
    conversations,
  );
}

function conversation(overrides: Partial<Conversation> = {}): Conversation {
  return {
    id: 'conversation-1',
    taskId: 'task-1',
    taskRunId: 'run-1',
    provider: 'codex',
    externalThreadId: 'thread-1',
    title: 'Worker conversation',
    summary: 'Initial summary',
    createdAt,
    updatedAt: createdAt,
    ...overrides,
  };
}
