import type {
  StartAgentSessionCommandInput,
  StartAgentSessionCommandResult,
  StartCodexTaskRunCommandResult,
} from '../application/runtimeCommandClient';
import type { TaskRunDetailSnapshot } from '../application/taskRunDetailClient';
import { createTauriRuntimeCommandClient, createTauriTaskRunDetailClient } from './tauriCommands';

describe('Tauri runtime command client', () => {
  it('invokes the start Codex task run command with the serialized input payload', async () => {
    const expectedResult: StartCodexTaskRunCommandResult = {
      status: 'completed',
      taskId: 'task-1',
      taskRunId: 'run-1',
      conversationId: 'conversation-1',
      rawEventStreamArtifactId: 'artifact-raw',
      finalResponseArtifactId: 'artifact-final',
      exitCode: 0,
      statusReason: 'Codex emitted a turn.completed event',
      postRunCapture: {
        diff: {
          status: 'captured',
          artifactId: 'artifact-diff',
          eventId: 'event-diff',
          diffLength: 120,
          isEmptyDiff: false,
          worktreePath: 'C:/worktree',
        },
        validation: {
          status: 'passed',
          validationRunId: 'validation-1',
          outputArtifactId: 'artifact-validation',
          startedEventId: 'event-validation-started',
          artifactCreatedEventId: 'event-validation-artifact',
          completedEventId: 'event-validation-completed',
          exitCode: 0,
        },
      },
      task: {
        id: 'task-1',
        executionState: 'completed',
        attentionState: 'needs_review',
        conversationIds: ['conversation-1'],
        updatedAt: '2026-07-03T10:00:00.000Z',
      },
      taskRun: {
        id: 'run-1',
        executionState: 'completed',
        conversationId: 'conversation-1',
        completedAt: '2026-07-03T10:01:00.000Z',
        exitCode: 0,
        updatedAt: '2026-07-03T10:01:00.000Z',
      },
    };
    const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
    const client = createTauriRuntimeCommandClient(
      async <T>(command: string, args?: Record<string, unknown>) => {
        calls.push({ command, args });
        return expectedResult as T;
      },
    );

    const result = await client.startCodexTaskRun({
      taskId: 'task-1',
      prompt: 'Run Codex',
      cwd: 'C:/worktree',
      worktreeId: 'worktree-1',
      additionalArgs: ['--sandbox', 'workspace-write'],
      env: { CODEX_PROFILE: 'test' },
      postRunCapture: {
        collectDiff: true,
        validationCommand: {
          command: 'npm',
          args: ['run', 'test'],
          cwd: 'C:/worktree',
          env: { CI: '1' },
        },
      },
    });

    expect(result).toBe(expectedResult);
    expect(calls).toEqual([
      {
        command: 'start_codex_task_run',
        args: {
          input: {
            taskId: 'task-1',
            prompt: 'Run Codex',
            cwd: 'C:/worktree',
            worktreeId: 'worktree-1',
            additionalArgs: ['--sandbox', 'workspace-write'],
            env: { CODEX_PROFILE: 'test' },
            postRunCapture: {
              collectDiff: true,
              validationCommand: {
                command: 'npm',
                args: ['run', 'test'],
                cwd: 'C:/worktree',
                env: { CI: '1' },
              },
            },
          },
        },
      },
    ]);
  });

  it('invokes the start Agent session command with the serialized input payload', async () => {
    const expectedResult: StartAgentSessionCommandResult = {
      sessionId: 'agent-session-1',
      status: 'completed',
      command: 'codex',
      args: [
        'exec',
        '--json',
        '--model',
        'gpt-5.5',
        'resume',
        'agent-session-1',
        'Explain this codebase',
      ],
      stdout: 'Done',
      stderr: '',
      startedAt: '2026-07-03T10:00:00.000Z',
      completedAt: '2026-07-03T10:01:00.000Z',
      exitCode: 0,
    };
    const events = new FakeTauriEventBus();
    const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
    const client = createTauriRuntimeCommandClient(
      async <T>(command: string, args?: Record<string, unknown>) => {
        calls.push({ command, args });
        const streamId = (args?.input as StartAgentSessionCommandInput | undefined)?.streamId;

        if (streamId) {
          queueMicrotask(() => {
            events.emit('agent-session-cli-completed', {
              streamId,
              result: expectedResult,
            });
          });
        }

        return {
          sessionId: expectedResult.sessionId,
          streamId,
          status: 'running',
          command: expectedResult.command,
          args: expectedResult.args,
          startedAt: expectedResult.startedAt,
        } as T;
      },
      events.listen,
    );

    const result = await client.startAgentSession({
      sessionId: 'agent-session-1',
      prompt: 'Explain this codebase',
      cwd: 'C:/worktree',
      additionalArgs: ['--model', 'gpt-5.5'],
      env: { CODEX_PROFILE: 'test' },
    });

    expect(result).toBe(expectedResult);
    expect(calls).toEqual([
      {
        command: 'start_agent_session',
        args: {
          input: {
            sessionId: 'agent-session-1',
            prompt: 'Explain this codebase',
            cwd: 'C:/worktree',
            additionalArgs: ['--model', 'gpt-5.5'],
            env: { CODEX_PROFILE: 'test' },
            streamId: expect.any(String),
          },
        },
      },
    ]);
  });

  it('delivers Agent session output before the completion event resolves the command', async () => {
    const expectedResult: StartAgentSessionCommandResult = {
      sessionId: 'agent-session-streamed',
      status: 'completed',
      command: 'codex',
      args: ['exec', '--json', 'Stream please'],
      stdout: JSON.stringify({ type: 'turn.completed' }),
      stderr: '',
      outputWasStreamed: true,
      startedAt: '2026-07-03T10:00:00.000Z',
      completedAt: '2026-07-03T10:01:00.000Z',
      exitCode: 0,
    };
    const events = new FakeTauriEventBus();
    const outputs: string[] = [];
    let settled = false;
    let activeStreamId: string | undefined;
    const client = createTauriRuntimeCommandClient(
      async <T>(command: string, args?: Record<string, unknown>) => {
        expect(command).toBe('start_agent_session');
        activeStreamId = (args?.input as StartAgentSessionCommandInput | undefined)?.streamId;

        if (activeStreamId) {
          queueMicrotask(() => {
            events.emit('agent-session-cli-output', {
              streamId: activeStreamId,
              stream: 'stdout',
              content: JSON.stringify({ type: 'turn.started' }),
            });
          });
        }

        return {
          sessionId: expectedResult.sessionId,
          streamId: activeStreamId,
          status: 'running',
          command: expectedResult.command,
          args: expectedResult.args,
          startedAt: expectedResult.startedAt,
        } as T;
      },
      events.listen,
    );

    const resultPromise = client
      .startAgentSession(
        { prompt: 'Stream please' },
        { onOutput: (chunk) => outputs.push(`${chunk.stream}:${chunk.content}`) },
      )
      .then((result) => {
        settled = true;
        return result;
      });

    await new Promise((resolve) => setTimeout(resolve, 0));

    expect(outputs).toEqual([`stdout:${JSON.stringify({ type: 'turn.started' })}`]);
    expect(settled).toBe(false);

    if (!activeStreamId) {
      throw new Error('Expected stream id to be assigned.');
    }

    events.emit('agent-session-cli-completed', {
      streamId: activeStreamId,
      result: expectedResult,
    });

    await expect(resultPromise).resolves.toBe(expectedResult);
    expect(settled).toBe(true);
  });
});

describe('Tauri task run detail client', () => {
  it('invokes the load task run detail command with the task id payload', async () => {
    const expectedResult: TaskRunDetailSnapshot = {
      task: {
        record: {
          id: 'task-1',
          projectId: 'project-1',
          conversationIds: [],
          title: 'Task',
          summary: 'Summary',
          executionState: 'draft',
          attentionState: 'needs_action_now',
          priority: 'normal',
          createdAt: '2026-07-03T10:00:00.000Z',
          updatedAt: '2026-07-03T10:00:00.000Z',
        },
      },
      runs: [],
      unlinkedArtifacts: {
        finalResponses: [],
        rawEventStreams: [],
        diffs: [],
        validationLogs: [],
        notes: [],
        screenshots: [],
        handoffs: [],
        summaries: [],
        other: [],
      },
      unlinkedValidationRuns: [],
      eventTimeline: [],
    };
    const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
    const client = createTauriTaskRunDetailClient(
      async <T>(command: string, args?: Record<string, unknown>) => {
        calls.push({ command, args });
        return expectedResult as T;
      },
    );

    const result = await client.loadTaskRunDetail('task-1');

    expect(result).toBe(expectedResult);
    expect(calls).toEqual([
      {
        command: 'load_task_run_detail',
        args: { taskId: 'task-1' },
      },
    ]);
  });
});

class FakeTauriEventBus {
  private readonly listeners = new Map<string, Set<(event: { payload: unknown }) => void>>();

  listen = async <T>(
    event: string,
    handler: (event: { payload: T }) => void,
  ): Promise<() => void> => {
    const listeners = this.listeners.get(event) ?? new Set<(event: { payload: unknown }) => void>();
    const wrapped = handler as (event: { payload: unknown }) => void;
    listeners.add(wrapped);
    this.listeners.set(event, listeners);

    return () => {
      listeners.delete(wrapped);
    };
  };

  emit(event: string, payload: unknown): void {
    this.listeners.get(event)?.forEach((listener) => listener({ payload }));
  }
}
