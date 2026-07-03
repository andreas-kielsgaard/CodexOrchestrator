import type { StartCodexTaskRunCommandResult } from '../application/runtimeCommandClient';
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
