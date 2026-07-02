import { loadOpenTaskDashboard } from './openTaskDashboardStore';
import {
  InMemoryOpenTaskWriteStore,
  OpenTaskNotFoundError,
  type IdProvider,
  type TimeProvider,
} from './openTaskWriteStore';
import { seedDomainRecords } from './seedData';
import type { DomainRecords, Task } from './model';

const baseTask: Task = {
  id: 'task-existing',
  projectId: 'project-orchestrator',
  repoId: 'repo-orchestrator',
  branchId: 'branch-main',
  worktreeId: 'worktree-main',
  conversationIds: ['conversation-original'],
  title: 'Existing task',
  summary: 'Existing task summary.',
  executionState: 'draft',
  attentionState: 'consider_later',
  priority: 'normal',
  dueAt: '2026-07-05T09:00:00.000Z',
  snoozedUntil: '2026-07-04T09:00:00.000Z',
  createdAt: '2026-07-01T09:00:00.000Z',
  updatedAt: '2026-07-01T09:00:00.000Z',
};

function recordsWithTasks(tasks: Task[]): DomainRecords {
  return {
    ...seedDomainRecords,
    tasks,
  };
}

function createStore(records = recordsWithTasks([baseTask])): InMemoryOpenTaskWriteStore {
  const ids: IdProvider = {
    nextId: () => 'task-created',
  };
  const clock: TimeProvider = {
    now: () => '2026-07-02T10:00:00.000Z',
  };

  return new InMemoryOpenTaskWriteStore(records, ids, clock);
}

describe('InMemoryOpenTaskWriteStore', () => {
  it('creates a task with deterministic id, timestamps, defaults, and optional anchors', async () => {
    const store = createStore(recordsWithTasks([]));

    const created = await store.createTask({
      projectId: 'project-orchestrator',
      repoId: 'repo-orchestrator',
      branchId: 'branch-main',
      worktreeId: 'worktree-main',
      conversationIds: ['conversation-a', 'conversation-b'],
      title: 'New task',
      summary: 'Create the useful next slice.',
      dueAt: '2026-07-10T09:00:00.000Z',
    });

    expect(created).toEqual({
      id: 'task-created',
      projectId: 'project-orchestrator',
      repoId: 'repo-orchestrator',
      branchId: 'branch-main',
      worktreeId: 'worktree-main',
      conversationIds: ['conversation-a', 'conversation-b'],
      title: 'New task',
      summary: 'Create the useful next slice.',
      executionState: 'draft',
      attentionState: 'consider_later',
      priority: 'normal',
      dueAt: '2026-07-10T09:00:00.000Z',
      createdAt: '2026-07-02T10:00:00.000Z',
      updatedAt: '2026-07-02T10:00:00.000Z',
    });
    expect(store.snapshot().tasks).toEqual([created]);
  });

  it('updates scalar fields and replaces conversation ids without reordering them', async () => {
    const store = createStore();

    const updated = await store.updateTask('task-existing', {
      title: 'Updated task',
      summary: 'Updated summary.',
      executionState: 'running',
      attentionState: 'waiting_on_agent',
      priority: 'high',
      conversationIds: ['conversation-z', 'conversation-a', 'conversation-m'],
    });

    expect(updated).toMatchObject({
      title: 'Updated task',
      summary: 'Updated summary.',
      executionState: 'running',
      attentionState: 'waiting_on_agent',
      priority: 'high',
      conversationIds: ['conversation-z', 'conversation-a', 'conversation-m'],
      updatedAt: '2026-07-02T10:00:00.000Z',
    });
  });

  it('treats null as an explicit clear for optional anchors and timestamps', async () => {
    const store = createStore();

    const updated = await store.updateTask('task-existing', {
      repoId: null,
      branchId: null,
      worktreeId: null,
      dueAt: null,
      snoozedUntil: null,
    });

    expect(updated).not.toHaveProperty('repoId');
    expect(updated).not.toHaveProperty('branchId');
    expect(updated).not.toHaveProperty('worktreeId');
    expect(updated).not.toHaveProperty('dueAt');
    expect(updated).not.toHaveProperty('snoozedUntil');
    expect(updated.updatedAt).toBe('2026-07-02T10:00:00.000Z');
  });

  it('leaves optional fields untouched when update values are omitted', async () => {
    const store = createStore();

    const updated = await store.updateTask('task-existing', {
      title: 'Only the title changes',
    });

    expect(updated).toMatchObject({
      repoId: 'repo-orchestrator',
      branchId: 'branch-main',
      worktreeId: 'worktree-main',
      dueAt: '2026-07-05T09:00:00.000Z',
      snoozedUntil: '2026-07-04T09:00:00.000Z',
      title: 'Only the title changes',
    });
  });

  it('throws a typed error when mutating a missing task', async () => {
    const store = createStore();

    await expect(store.updateTask('task-missing', { title: 'Nope' })).rejects.toThrow(
      OpenTaskNotFoundError,
    );
    await expect(store.archiveTask('task-missing')).rejects.toThrow('Open task not found');
  });

  it('archives tasks by state and relies on dashboard projection to omit closed tasks', async () => {
    const store = createStore(
      recordsWithTasks([
        baseTask,
        {
          ...baseTask,
          id: 'task-open',
          title: 'Still open',
          updatedAt: '2026-07-01T10:00:00.000Z',
        },
      ]),
    );

    const archived = await store.archiveTask('task-existing');
    const groups = await loadOpenTaskDashboard(store);
    const dashboardTaskIds = groups.flatMap((group) => group.tasks).map((task) => task.id);

    expect(archived.executionState).toBe('archived');
    expect(dashboardTaskIds).toEqual(['task-open']);
  });

  it('preserves unrelated domain records while mutating tasks', async () => {
    const records = recordsWithTasks([baseTask]);
    const store = createStore(records);

    await store.updateTask('task-existing', { title: 'Changed' });

    const snapshot = store.snapshot();
    expect(withoutTasks(snapshot)).toEqual(withoutTasks(records));
    expect(snapshot.tasks).toHaveLength(1);
    expect(snapshot.tasks[0]?.title).toBe('Changed');
  });
});

function withoutTasks(records: DomainRecords): Omit<DomainRecords, 'tasks'> {
  return {
    projects: records.projects,
    repos: records.repos,
    branches: records.branches,
    worktrees: records.worktrees,
    conversations: records.conversations,
    taskRuns: records.taskRuns,
    artifacts: records.artifacts,
    validationRuns: records.validationRuns,
    events: records.events,
  };
}
