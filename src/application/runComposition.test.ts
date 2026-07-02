import { InMemoryArtifactStore } from '../domain/artifactStore';
import { InMemoryConversationStore } from '../domain/conversationStore';
import { InMemoryEventStore } from '../domain/eventStore';
import type { DomainRecords, IsoDateTime, Task } from '../domain/model';
import { InMemoryOpenTaskWriteStore } from '../domain/openTaskWriteStore';
import { seedDomainRecords } from '../domain/seedData';
import { InMemoryTaskRunStore } from '../domain/taskRunStore';
import {
  composeCodexTaskRun,
  type CodexRunRuntime,
  type CodexRunRuntimeResult,
  type RunCompositionService,
} from './runComposition';
import type { TaskRunLifecycleRecorder } from './taskRunLifecycle';

const baseTask: Task = {
  id: 'task-compose',
  projectId: 'project-orchestrator',
  repoId: 'repo-orchestrator',
  branchId: 'branch-main',
  worktreeId: 'worktree-main',
  conversationIds: ['conversation-existing'],
  title: 'Composition task',
  summary: 'Persist a composed Codex run.',
  executionState: 'queued',
  attentionState: 'consider_later',
  priority: 'normal',
  createdAt: '2026-07-02T08:00:00.000Z',
  updatedAt: '2026-07-02T08:00:00.000Z',
};

describe('run composition service', () => {
  it('starts a run, executes Codex, stores raw JSONL, updates conversation metadata, and completes with a final response', async () => {
    const runtimeResult = createRuntimeResult({
      status: 'completed',
      statusReason: 'Codex emitted a turn.completed event',
      stdoutJsonl: completedJsonl,
      exitCode: 0,
      summary: {
        threadId: 'thread-123',
        finalAgentMessageText: 'Implemented the composition service.',
        terminalStatus: { kind: 'completed', lineNumber: 3 },
        tokenUsage: { input_tokens: 10, output_tokens: 20 },
        itemCountsByType: { agent_message: 1 },
      },
    });
    const fixture = createCompositionFixture({ runtime: new FakeCodexRuntime(runtimeResult) });

    const result = await composeCodexTaskRun(fixture.service, {
      taskId: 'task-compose',
      prompt: 'Build FS-07',
      cwd: 'C:/worktree',
      worktreeId: 'worktree-worker',
      startedAt: '2026-07-02T10:00:00.000Z',
      completedAt: '2026-07-02T10:30:00.000Z',
      conversationTitle: 'Worker 029',
      additionalArgs: ['--sandbox', 'workspace-write'],
      env: { CODEX_PROFILE: 'test' },
    });

    expect(result.status).toBe('completed');
    if (result.status !== 'completed') {
      throw new Error('Expected completed composition result');
    }
    expect(fixture.runtime.calls).toEqual([
      {
        prompt: 'Build FS-07',
        cwd: 'C:/worktree',
        additionalArgs: ['--sandbox', 'workspace-write'],
        env: { CODEX_PROFILE: 'test' },
      },
    ]);
    expect(result.started.taskRun).toMatchObject({
      id: 'run-001',
      taskId: 'task-compose',
      conversationId: 'conversation-001',
      worktreeId: 'worktree-worker',
      executionState: 'running',
    });
    expect(result.rawEventStreamArtifact).toMatchObject({
      id: 'artifact-001',
      kind: 'raw_event_stream',
      title: 'Raw Codex JSONL',
      taskId: 'task-compose',
      taskRunId: 'run-001',
      conversationId: 'conversation-001',
      content: completedJsonl,
    });
    expect(result.artifactCreatedEvent).toMatchObject({
      id: 'event-002',
      kind: 'artifact_created',
      projectId: 'project-orchestrator',
      taskId: 'task-compose',
      taskRunId: 'run-001',
      conversationId: 'conversation-001',
      artifactId: 'artifact-001',
      payload: {
        artifactKind: 'raw_event_stream',
        artifactId: 'artifact-001',
        codexStatus: 'completed',
        stdoutJsonlLength: completedJsonl.length,
        exitCode: 0,
      },
    });
    expect(result.conversation).toMatchObject({
      id: 'conversation-001',
      externalThreadId: 'thread-123',
      summary: 'Codex completed: Implemented the composition service.',
      title: 'Worker 029',
    });
    expect(result.completed.taskRun).toMatchObject({
      id: 'run-001',
      executionState: 'completed',
      completedAt: '2026-07-02T10:30:00.000Z',
      exitCode: 0,
    });
    expect(result.completed.artifact).toMatchObject({
      id: 'artifact-002',
      kind: 'final_response',
      title: 'Final Codex response',
      content: 'Implemented the composition service.',
      conversationId: 'conversation-001',
    });
    expect(fixture.eventStore.snapshot().map((event) => event.kind)).toEqual([
      'run_started',
      'artifact_created',
      'run_completed',
    ]);
    expect(
      fixture.taskStore.snapshot().tasks.find((task) => task.id === 'task-compose'),
    ).toMatchObject({
      executionState: 'completed',
      attentionState: 'needs_review',
      conversationIds: ['conversation-existing', 'conversation-001'],
    });
  });

  it('stores structured failed Codex output before failing the lifecycle', async () => {
    const runtimeResult = createRuntimeResult({
      status: 'failed',
      statusReason: 'Codex emitted a turn.failed event',
      stdoutJsonl: failedJsonl,
      stderr: 'permission denied',
      exitCode: 1,
      summary: {
        threadId: 'thread-failed',
        terminalStatus: { kind: 'failed', lineNumber: 2 },
        itemCountsByType: {},
      },
    });
    const fixture = createCompositionFixture({ runtime: new FakeCodexRuntime(runtimeResult) });

    const result = await composeCodexTaskRun(fixture.service, {
      taskId: 'task-compose',
      prompt: 'Try FS-07',
      completedAt: '2026-07-02T10:20:00.000Z',
    });

    expect(result.status).toBe('failed');
    if (result.status !== 'failed') {
      throw new Error('Expected failed composition result');
    }
    expect(result.rawEventStreamArtifact).toMatchObject({
      id: 'artifact-001',
      kind: 'raw_event_stream',
      content: failedJsonl,
    });
    expect(result.conversation).toMatchObject({
      externalThreadId: 'thread-failed',
      summary: 'Codex failed: Codex emitted a turn.failed event',
    });
    expect(result.failed.taskRun).toMatchObject({
      id: 'run-001',
      executionState: 'failed',
      completedAt: '2026-07-02T10:20:00.000Z',
      exitCode: 1,
    });
    expect(result.failed.event).toMatchObject({
      id: 'event-003',
      kind: 'run_completed',
      payload: {
        outcome: 'failed',
        taskId: 'task-compose',
        taskRunId: 'run-001',
        completedAt: '2026-07-02T10:20:00.000Z',
        exitCode: 1,
        error: 'Codex emitted a turn.failed event: permission denied',
      },
    });
    expect(fixture.artifactStore.snapshot()).toHaveLength(1);
    expect(fixture.eventStore.snapshot().map((event) => event.kind)).toEqual([
      'run_started',
      'artifact_created',
      'run_completed',
    ]);
  });

  it('stores structured Codex error output before failing the lifecycle', async () => {
    const runtimeResult = createRuntimeResult({
      status: 'error',
      statusReason: 'Codex emitted an error event',
      stdoutJsonl: errorJsonl,
      stderr: '',
      exitCode: null,
      summary: {
        threadId: 'thread-error',
        terminalStatus: { kind: 'error', lineNumber: 2 },
        itemCountsByType: {},
      },
    });
    const fixture = createCompositionFixture({ runtime: new FakeCodexRuntime(runtimeResult) });

    const result = await composeCodexTaskRun(fixture.service, {
      taskId: 'task-compose',
      prompt: 'Try FS-07',
      completedAt: '2026-07-02T10:25:00.000Z',
    });

    expect(result.status).toBe('failed');
    if (result.status !== 'failed') {
      throw new Error('Expected failed composition result');
    }
    expect(result.error).toBe('Codex emitted an error event');
    expect(result.rawEventStreamArtifact).toMatchObject({
      id: 'artifact-001',
      kind: 'raw_event_stream',
      content: errorJsonl,
    });
    expect(result.artifactCreatedEvent).toMatchObject({
      id: 'event-002',
      payload: {
        artifactKind: 'raw_event_stream',
        artifactId: 'artifact-001',
        codexStatus: 'error',
        stdoutJsonlLength: errorJsonl.length,
      },
    });
    expect(result.conversation).toMatchObject({
      externalThreadId: 'thread-error',
      summary: 'Codex error: Codex emitted an error event',
    });
    expect(result.failed.taskRun).toMatchObject({
      id: 'run-001',
      executionState: 'failed',
      completedAt: '2026-07-02T10:25:00.000Z',
    });
    expect(result.failed.taskRun).not.toHaveProperty('exitCode');
    expect(result.failed.event).toMatchObject({
      id: 'event-003',
      kind: 'run_completed',
      payload: {
        outcome: 'failed',
        taskId: 'task-compose',
        taskRunId: 'run-001',
        completedAt: '2026-07-02T10:25:00.000Z',
        error: 'Codex emitted an error event',
      },
    });
  });

  it('fails the started lifecycle when the runtime throws without fabricating raw artifacts', async () => {
    const fixture = createCompositionFixture({
      runtime: new FakeCodexRuntime(new Error('codex launch failed')),
    });

    const result = await composeCodexTaskRun(fixture.service, {
      taskId: 'task-compose',
      prompt: 'Run Codex',
      completedAt: '2026-07-02T10:10:00.000Z',
    });

    expect(result.status).toBe('failed');
    if (result.status !== 'failed') {
      throw new Error('Expected failed composition result');
    }
    expect(result.error).toBe('codex launch failed');
    expect(result.rawEventStreamArtifact).toBeUndefined();
    expect(result.artifactCreatedEvent).toBeUndefined();
    expect(result.runtimeResult).toBeUndefined();
    expect(result.failed.taskRun).toMatchObject({
      id: 'run-001',
      executionState: 'failed',
      completedAt: '2026-07-02T10:10:00.000Z',
    });
    expect(result.failed.taskRun).not.toHaveProperty('exitCode');
    expect(result.failed.event).toMatchObject({
      id: 'event-002',
      kind: 'run_completed',
      payload: {
        outcome: 'failed',
        taskId: 'task-compose',
        taskRunId: 'run-001',
        completedAt: '2026-07-02T10:10:00.000Z',
        error: 'codex launch failed',
      },
    });
    expect(fixture.artifactStore.snapshot()).toEqual([]);
    expect(fixture.conversationStore.snapshot()).toEqual([
      expect.objectContaining({
        id: 'conversation-001',
        taskId: 'task-compose',
        taskRunId: 'run-001',
        provider: 'codex',
        title: 'Codex run',
      }),
    ]);
    expect(fixture.eventStore.snapshot().map((event) => event.kind)).toEqual([
      'run_started',
      'run_completed',
    ]);
  });
});

const completedJsonl = [
  JSON.stringify({ type: 'thread.started', thread_id: 'thread-123' }),
  JSON.stringify({ type: 'item.completed', item: { type: 'agent_message', text: 'Done' } }),
  JSON.stringify({ type: 'turn.completed', usage: { input_tokens: 10, output_tokens: 20 } }),
].join('\n');

const failedJsonl = [
  JSON.stringify({ type: 'thread.started', thread_id: 'thread-failed' }),
  JSON.stringify({ type: 'turn.failed' }),
].join('\n');

const errorJsonl = [
  JSON.stringify({ type: 'thread.started', thread_id: 'thread-error' }),
  JSON.stringify({ type: 'error', message: 'fatal' }),
].join('\n');

interface CompositionFixture {
  service: RunCompositionService;
  runtime: FakeCodexRuntime;
  taskStore: InMemoryOpenTaskWriteStore;
  taskRunStore: InMemoryTaskRunStore;
  conversationStore: InMemoryConversationStore;
  artifactStore: InMemoryArtifactStore;
  eventStore: InMemoryEventStore;
}

function createCompositionFixture(options: { runtime: FakeCodexRuntime }): CompositionFixture {
  const records: DomainRecords = {
    ...seedDomainRecords,
    tasks: [baseTask],
    taskRuns: [],
    conversations: [],
    artifacts: [],
    events: [],
    validationRuns: [],
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
  const recorder: TaskRunLifecycleRecorder = {
    openTaskDashboardStore: taskStore,
    openTaskWriteStore: taskStore,
    taskRunStore,
    conversationStore,
    artifactStore,
    eventStore,
  };

  return {
    service: {
      recorder,
      runtime: options.runtime,
    },
    runtime: options.runtime,
    taskStore,
    taskRunStore,
    conversationStore,
    artifactStore,
    eventStore,
  };
}

function createRuntimeResult(
  overrides: Partial<CodexRunRuntimeResult> = {},
): CodexRunRuntimeResult {
  return {
    command: 'codex',
    args: ['exec', '--json', 'prompt'],
    exitCode: 0,
    signal: null,
    status: 'completed',
    statusReason: 'Codex emitted a turn.completed event',
    stdoutJsonl: completedJsonl,
    stderr: '',
    summary: {
      itemCountsByType: {},
    },
    ...overrides,
  };
}

class FakeCodexRuntime implements CodexRunRuntime {
  readonly calls: Array<{
    prompt: string;
    cwd?: string;
    additionalArgs?: readonly string[];
    env?: Record<string, string | undefined>;
  }> = [];

  constructor(private readonly result: CodexRunRuntimeResult | Error) {}

  async exec(input: Parameters<CodexRunRuntime['exec']>[0]): Promise<CodexRunRuntimeResult> {
    this.calls.push({
      prompt: input.prompt,
      ...(input.cwd === undefined ? {} : { cwd: input.cwd }),
      ...(input.additionalArgs === undefined ? {} : { additionalArgs: [...input.additionalArgs] }),
      ...(input.env === undefined ? {} : { env: { ...input.env } }),
    });

    if (this.result instanceof Error) {
      throw this.result;
    }

    return this.result;
  }
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
