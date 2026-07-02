import { InMemoryArtifactStore } from '../domain/artifactStore';
import { InMemoryConversationStore } from '../domain/conversationStore';
import { InMemoryEventStore } from '../domain/eventStore';
import type { DomainRecords, IsoDateTime, Task } from '../domain/model';
import { InMemoryOpenTaskWriteStore } from '../domain/openTaskWriteStore';
import { seedDomainRecords } from '../domain/seedData';
import { InMemoryTaskRunStore } from '../domain/taskRunStore';
import {
  completeTaskRunLifecycle,
  failTaskRunLifecycle,
  startTaskRunLifecycle,
  TaskRunLifecycleTaskNotFoundError,
  type TaskRunLifecycleRecorder,
} from './taskRunLifecycle';

const baseTask: Task = {
  id: 'task-lifecycle',
  projectId: 'project-orchestrator',
  repoId: 'repo-orchestrator',
  branchId: 'branch-main',
  worktreeId: 'worktree-main',
  conversationIds: ['conversation-existing'],
  title: 'Lifecycle task',
  summary: 'Record task run state.',
  executionState: 'queued',
  attentionState: 'consider_later',
  priority: 'normal',
  createdAt: '2026-07-02T08:00:00.000Z',
  updatedAt: '2026-07-02T08:00:00.000Z',
};

describe('task run lifecycle recorder', () => {
  it('starts a task run, creates linked conversation metadata, updates task state, and emits an event', async () => {
    const fixture = createRecorderFixture();

    const result = await startTaskRunLifecycle(fixture.recorder, {
      taskId: 'task-lifecycle',
      worktreeId: 'worktree-worker',
      startedAt: '2026-07-02T10:00:00.000Z',
      conversation: {
        title: 'Worker 025',
        externalThreadId: 'thread-025',
        summary: 'Lifecycle recorder implementation.',
      },
    });

    expect(result.taskRun).toMatchObject({
      id: 'run-001',
      taskId: 'task-lifecycle',
      conversationId: 'conversation-001',
      worktreeId: 'worktree-worker',
      executionState: 'running',
      startedAt: '2026-07-02T10:00:00.000Z',
    });
    expect(result.conversation).toMatchObject({
      id: 'conversation-001',
      taskId: 'task-lifecycle',
      taskRunId: 'run-001',
      provider: 'codex',
      externalThreadId: 'thread-025',
      title: 'Worker 025',
      summary: 'Lifecycle recorder implementation.',
    });
    expect(result.task).toMatchObject({
      executionState: 'running',
      attentionState: 'waiting_on_agent',
      conversationIds: ['conversation-existing', 'conversation-001'],
    });
    expect(result.event).toMatchObject({
      id: 'event-001',
      kind: 'run_started',
      projectId: 'project-orchestrator',
      taskId: 'task-lifecycle',
      taskRunId: 'run-001',
      conversationId: 'conversation-001',
      payload: {
        taskId: 'task-lifecycle',
        taskRunId: 'run-001',
        worktreeId: 'worktree-worker',
        startedAt: '2026-07-02T10:00:00.000Z',
        conversationId: 'conversation-001',
      },
    });
  });

  it('preserves conversation ids when no new conversation is created', async () => {
    const fixture = createRecorderFixture();

    const result = await startTaskRunLifecycle(fixture.recorder, {
      taskId: 'task-lifecycle',
    });

    expect(result.conversation).toBeUndefined();
    expect(result.task.conversationIds).toEqual(['conversation-existing']);
    expect(result.taskRun).not.toHaveProperty('conversationId');
    expect(result.event).not.toHaveProperty('conversationId');
  });

  it('records successful completion with a final-response artifact and completion event', async () => {
    const fixture = createRecorderFixture();
    const started = await startTaskRunLifecycle(fixture.recorder, {
      taskId: 'task-lifecycle',
      conversation: { title: 'Worker 025' },
    });

    const result = await completeTaskRunLifecycle(fixture.recorder, {
      taskId: 'task-lifecycle',
      taskRunId: started.taskRun.id,
      completedAt: '2026-07-02T10:30:00.000Z',
      exitCode: 0,
      finalResponse: {
        title: 'Worker report',
        content: 'Implemented lifecycle recorder.',
      },
    });

    expect(result.taskRun).toMatchObject({
      id: 'run-001',
      executionState: 'completed',
      completedAt: '2026-07-02T10:30:00.000Z',
      exitCode: 0,
      conversationId: 'conversation-001',
    });
    expect(result.task).toMatchObject({
      executionState: 'completed',
      attentionState: 'needs_review',
    });
    expect(result.artifact).toMatchObject({
      id: 'artifact-001',
      kind: 'final_response',
      title: 'Worker report',
      taskId: 'task-lifecycle',
      taskRunId: 'run-001',
      conversationId: 'conversation-001',
      content: 'Implemented lifecycle recorder.',
    });
    expect(result.event).toMatchObject({
      id: 'event-002',
      kind: 'run_completed',
      taskRunId: 'run-001',
      conversationId: 'conversation-001',
      artifactId: 'artifact-001',
      payload: {
        outcome: 'completed',
        taskId: 'task-lifecycle',
        taskRunId: 'run-001',
        completedAt: '2026-07-02T10:30:00.000Z',
        exitCode: 0,
        artifactId: 'artifact-001',
      },
    });
  });

  it('records failed completion without creating an artifact', async () => {
    const fixture = createRecorderFixture();
    const started = await startTaskRunLifecycle(fixture.recorder, {
      taskId: 'task-lifecycle',
    });

    const result = await failTaskRunLifecycle(fixture.recorder, {
      taskId: 'task-lifecycle',
      taskRunId: started.taskRun.id,
      completedAt: '2026-07-02T10:20:00.000Z',
      exitCode: 1,
      error: 'Codex process exited non-zero.',
    });

    expect(result.taskRun).toMatchObject({
      executionState: 'failed',
      completedAt: '2026-07-02T10:20:00.000Z',
      exitCode: 1,
    });
    expect(result.task).toMatchObject({
      executionState: 'failed',
      attentionState: 'needs_action_now',
    });
    expect(fixture.artifactStore.snapshot()).toEqual([]);
    expect(result.event).toMatchObject({
      kind: 'run_completed',
      payload: {
        outcome: 'failed',
        taskId: 'task-lifecycle',
        taskRunId: 'run-001',
        completedAt: '2026-07-02T10:20:00.000Z',
        exitCode: 1,
        error: 'Codex process exited non-zero.',
      },
    });
  });

  it('preflights missing tasks before creating dependent records', async () => {
    const fixture = createRecorderFixture({ tasks: [] });

    await expect(
      startTaskRunLifecycle(fixture.recorder, {
        taskId: 'task-missing',
        conversation: { title: 'Should not exist' },
      }),
    ).rejects.toThrow(TaskRunLifecycleTaskNotFoundError);

    expect(fixture.taskRunStore.snapshot()).toEqual([]);
    expect(fixture.conversationStore.snapshot()).toEqual([]);
    expect(fixture.artifactStore.snapshot()).toEqual([]);
    expect(fixture.eventStore.snapshot()).toEqual([]);
  });

  it('emits queryable start and terminal events with linked ids', async () => {
    const fixture = createRecorderFixture();
    const started = await startTaskRunLifecycle(fixture.recorder, {
      taskId: 'task-lifecycle',
      conversation: { title: 'Worker 025' },
    });

    await completeTaskRunLifecycle(fixture.recorder, {
      taskId: 'task-lifecycle',
      taskRunId: started.taskRun.id,
      finalResponse: { content: 'Done.' },
    });

    const events = await fixture.eventStore.queryEvents({
      taskId: 'task-lifecycle',
      taskRunId: 'run-001',
    });

    expect(events.map((event) => event.kind)).toEqual(['run_started', 'run_completed']);
    expect(events[0]?.conversationId).toBe('conversation-001');
    expect(events[1]?.artifactId).toBe('artifact-001');
  });
});

interface RecorderFixture {
  recorder: TaskRunLifecycleRecorder;
  taskStore: InMemoryOpenTaskWriteStore;
  taskRunStore: InMemoryTaskRunStore;
  conversationStore: InMemoryConversationStore;
  artifactStore: InMemoryArtifactStore;
  eventStore: InMemoryEventStore;
}

function createRecorderFixture(overrides: Partial<DomainRecords> = {}): RecorderFixture {
  const records: DomainRecords = {
    ...seedDomainRecords,
    tasks: [baseTask],
    taskRuns: [],
    conversations: [],
    artifacts: [],
    events: [],
    validationRuns: [],
    ...overrides,
  };
  const taskStore = new InMemoryOpenTaskWriteStore(
    records,
    sequenceIds('task-unused'),
    fixedClock('2026-07-02T10:05:00.000Z'),
  );
  const taskRunStore = new InMemoryTaskRunStore(
    sequenceIds('run'),
    fixedClock('2026-07-02T10:01:00.000Z'),
  );
  const conversationStore = new InMemoryConversationStore(
    sequenceIds('conversation'),
    fixedClock('2026-07-02T10:02:00.000Z'),
  );
  const artifactStore = new InMemoryArtifactStore(
    sequenceIds('artifact'),
    fixedClock('2026-07-02T10:03:00.000Z'),
  );
  const eventStore = new InMemoryEventStore(
    sequenceIds('event'),
    fixedClock('2026-07-02T10:04:00.000Z'),
  );

  return {
    recorder: {
      openTaskDashboardStore: taskStore,
      openTaskWriteStore: taskStore,
      taskRunStore,
      conversationStore,
      artifactStore,
      eventStore,
    },
    taskStore,
    taskRunStore,
    conversationStore,
    artifactStore,
    eventStore,
  };
}

function fixedClock(now: IsoDateTime): { now(): IsoDateTime } {
  return {
    now: () => now,
  };
}

function sequenceIds(prefix: string): { nextId(): string } {
  let next = 1;

  return {
    nextId: () => `${prefix}-${String(next++).padStart(3, '0')}`,
  };
}
