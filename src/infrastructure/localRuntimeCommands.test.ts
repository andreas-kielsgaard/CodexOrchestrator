import { InMemoryArtifactStore } from '../domain/artifactStore';
import { InMemoryConversationStore } from '../domain/conversationStore';
import { InMemoryEventStore } from '../domain/eventStore';
import type { DomainRecords, EntityId, IsoDateTime, Task } from '../domain/model';
import { InMemoryOpenTaskWriteStore } from '../domain/openTaskWriteStore';
import { seedDomainRecords } from '../domain/seedData';
import { InMemoryTaskRunStore } from '../domain/taskRunStore';
import type {
  CodexRunRuntime,
  CodexRunRuntimeResult,
  RunCompositionService,
} from '../application/runComposition';
import type { TaskRunLifecycleRecorder } from '../application/taskRunLifecycle';
import type { LocalRuntimeServiceComposition } from './localRuntimeComposition';
import { createLocalRuntimeCommandHandler, startCodexTaskRun } from './localRuntimeCommands';

const baseTask: Task = {
  id: 'task-command',
  projectId: 'project-orchestrator',
  repoId: 'repo-orchestrator',
  branchId: 'branch-main',
  worktreeId: 'worktree-main',
  conversationIds: [],
  title: 'Runtime command task',
  summary: 'Start a Codex task run through the command boundary.',
  executionState: 'queued',
  attentionState: 'consider_later',
  priority: 'normal',
  createdAt: '2026-07-03T08:00:00.000Z',
  updatedAt: '2026-07-03T08:00:00.000Z',
};

describe('local runtime commands', () => {
  it('starts a Codex task run through the composed local runtime service', async () => {
    const fixture = createCommandFixture({
      runtime: new FakeCodexRuntime(
        createRuntimeResult({
          summary: {
            threadId: 'thread-command',
            finalAgentMessageText: 'Runtime command boundary is ready.',
            terminalStatus: { kind: 'completed', lineNumber: 3 },
            itemCountsByType: { agent_message: 1 },
          },
        }),
      ),
    });

    const result = await startCodexTaskRun(
      fixture.composition,
      {
        taskId: 'task-command',
        prompt: 'Build the runtime command boundary',
        cwd: 'C:/worktrees/codex-orchestrator',
        worktreeId: 'worktree-worker',
        conversationTitle: 'Worker 039',
        additionalArgs: ['--sandbox', 'workspace-write'],
        env: { CODEX_PROFILE: 'test' },
      },
      {
        startedAt: '2026-07-03T10:00:00.000Z',
        completedAt: '2026-07-03T10:10:00.000Z',
      },
    );

    expect(fixture.runtime.calls).toEqual([
      {
        prompt: 'Build the runtime command boundary',
        cwd: 'C:/worktrees/codex-orchestrator',
        additionalArgs: ['--sandbox', 'workspace-write'],
        env: { CODEX_PROFILE: 'test' },
      },
    ]);
    expect(result).toEqual({
      status: 'completed',
      taskId: 'task-command',
      taskRunId: 'run-001',
      conversationId: 'conversation-001',
      rawEventStreamArtifactId: 'artifact-001',
      finalResponseArtifactId: 'artifact-002',
      exitCode: 0,
      statusReason: 'Codex emitted a turn.completed event',
      task: {
        id: 'task-command',
        executionState: 'completed',
        attentionState: 'needs_review',
        conversationIds: ['conversation-001'],
        repoId: 'repo-orchestrator',
        branchId: 'branch-main',
        worktreeId: 'worktree-main',
        updatedAt: '2026-07-03T10:05:00.000Z',
      },
      taskRun: {
        id: 'run-001',
        executionState: 'completed',
        conversationId: 'conversation-001',
        worktreeId: 'worktree-worker',
        startedAt: '2026-07-03T10:00:00.000Z',
        completedAt: '2026-07-03T10:10:00.000Z',
        exitCode: 0,
        updatedAt: '2026-07-03T10:01:00.000Z',
      },
    });
  });

  it('returns compact failed run state when Codex launch fails', async () => {
    const fixture = createCommandFixture({
      runtime: new FakeCodexRuntime(new Error('codex launch failed')),
    });
    const handler = createLocalRuntimeCommandHandler(fixture.composition, {
      completedAt: '2026-07-03T10:15:00.000Z',
    });

    const result = await handler.startCodexTaskRun({
      taskId: 'task-command',
      prompt: 'Run Codex',
    });

    expect(result).toEqual({
      status: 'failed',
      taskId: 'task-command',
      taskRunId: 'run-001',
      conversationId: 'conversation-001',
      error: 'codex launch failed',
      task: {
        id: 'task-command',
        executionState: 'failed',
        attentionState: 'needs_action_now',
        conversationIds: ['conversation-001'],
        repoId: 'repo-orchestrator',
        branchId: 'branch-main',
        worktreeId: 'worktree-main',
        updatedAt: '2026-07-03T10:05:00.000Z',
      },
      taskRun: {
        id: 'run-001',
        executionState: 'failed',
        conversationId: 'conversation-001',
        completedAt: '2026-07-03T10:15:00.000Z',
        updatedAt: '2026-07-03T10:01:00.000Z',
      },
    });
    expect(fixture.artifactStore.snapshot()).toEqual([]);
  });
});

interface CommandFixture {
  composition: LocalRuntimeServiceComposition;
  runtime: FakeCodexRuntime;
  artifactStore: InMemoryArtifactStore;
}

function createCommandFixture(options: { runtime: FakeCodexRuntime }): CommandFixture {
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
    fixedClock('2026-07-03T10:05:00.000Z'),
  );
  const taskRunStore = new InMemoryTaskRunStore(
    sequenceIds('run'),
    fixedClock('2026-07-03T10:01:00.000Z'),
  );
  const conversationStore = new InMemoryConversationStore(
    sequenceIds('conversation'),
    fixedClock('2026-07-03T10:02:00.000Z'),
  );
  const artifactStore = new InMemoryArtifactStore(
    sequenceIds('artifact'),
    fixedClock('2026-07-03T10:03:00.000Z'),
  );
  const eventStore = new InMemoryEventStore(
    sequenceIds('event'),
    fixedClock('2026-07-03T10:04:00.000Z'),
  );
  const recorder: TaskRunLifecycleRecorder = {
    openTaskDashboardStore: taskStore,
    openTaskWriteStore: taskStore,
    taskRunStore,
    conversationStore,
    artifactStore,
    eventStore,
  };
  const service: RunCompositionService = {
    recorder,
    runtime: options.runtime,
  };

  return {
    composition: {
      services: {
        runCompositionService: service,
      },
    } as LocalRuntimeServiceComposition,
    runtime: options.runtime,
    artifactStore,
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

const completedJsonl = [
  JSON.stringify({ type: 'thread.started', thread_id: 'thread-command' }),
  JSON.stringify({
    type: 'item.completed',
    item: { type: 'agent_message', text: 'Runtime command boundary is ready.' },
  }),
  JSON.stringify({ type: 'turn.completed' }),
].join('\n');

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

function sequenceIds(prefix: string): { nextId(): EntityId } {
  let next = 1;

  return {
    nextId: () => `${prefix}-${String(next++).padStart(3, '0')}`,
  };
}
