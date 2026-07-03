import type { StartCodexTaskRunCommandResult } from '../application/runtimeCommandClient';
import { createTauriRuntimeCommandClient } from './tauriCommands';

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
          },
        },
      },
    ]);
  });
});
