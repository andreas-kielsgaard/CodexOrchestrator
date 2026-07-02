import { CodexJsonlParseError } from './jsonlEvents';
import {
  buildCodexExecArgs,
  createCodexRuntime,
  runCodexExec,
  type CodexProcessRunInput,
  type CodexProcessRunResult,
  type CodexProcessRunner,
} from './codexRuntime';

describe('buildCodexExecArgs', () => {
  it('builds the normal codex exec --json command arguments with the prompt last', () => {
    expect(buildCodexExecArgs({ prompt: 'Implement the task' })).toEqual([
      'exec',
      '--json',
      'Implement the task',
    ]);
  });

  it('places additional exec options before the prompt', () => {
    expect(
      buildCodexExecArgs({
        prompt: 'Implement the task',
        additionalArgs: ['--model', 'gpt-5-codex'],
      }),
    ).toEqual(['exec', '--json', '--model', 'gpt-5-codex', 'Implement the task']);
  });
});

describe('runCodexExec', () => {
  it('invokes Codex through the runner, preserves raw output, parses events, and summarizes success', async () => {
    const stdout = jsonl([
      { type: 'thread.started', thread_id: 'thread-123' },
      { type: 'turn.started' },
      { type: 'item.completed', item: { type: 'agent_message', text: 'Done' } },
      { type: 'turn.completed', usage: { total_tokens: 9 } },
    ]);
    const runner = new FakeCodexProcessRunner({ stdout, stderr: 'minor warning', exitCode: 0 });
    const stdoutChunks: string[] = [];
    const stderrChunks: string[] = [];

    const result = await runCodexExec(
      {
        prompt: 'Ship it',
        cwd: 'C:/repo/worktree',
        env: { CODEX_HOME: 'C:/codex-home' },
        onStdoutChunk: (chunk) => stdoutChunks.push(chunk),
        onStderrChunk: (chunk) => stderrChunks.push(chunk),
      },
      { command: 'codex', runner },
    );

    expect(runner.inputs).toEqual([
      {
        command: 'codex',
        args: ['exec', '--json', 'Ship it'],
        cwd: 'C:/repo/worktree',
        env: { CODEX_HOME: 'C:/codex-home' },
        onStdoutChunk: expect.any(Function) as (chunk: string) => void,
        onStderrChunk: expect.any(Function) as (chunk: string) => void,
      },
    ]);
    expect(stdoutChunks).toEqual([stdout]);
    expect(stderrChunks).toEqual(['minor warning']);
    expect(result).toMatchObject({
      command: 'codex',
      args: ['exec', '--json', 'Ship it'],
      cwd: 'C:/repo/worktree',
      exitCode: 0,
      signal: null,
      status: 'completed',
      statusReason: 'Codex emitted a turn.completed event',
      stdoutJsonl: stdout,
      stderr: 'minor warning',
      summary: {
        threadId: 'thread-123',
        finalAgentMessageText: 'Done',
        terminalStatus: { kind: 'completed', lineNumber: 4 },
        tokenUsage: { total_tokens: 9 },
        itemCountsByType: { agent_message: 1 },
      },
    });
    expect(result.events).toHaveLength(4);
  });

  it('returns a structured failed result for a non-zero Codex exit when JSONL is parseable', async () => {
    const stdout = jsonl([
      { type: 'thread.started', thread_id: 'thread-456' },
      { type: 'turn.completed' },
    ]);
    const runner = new FakeCodexProcessRunner({
      stdout,
      stderr: 'process exited badly',
      exitCode: 2,
    });

    const result = await runCodexExec({ prompt: 'Try it' }, { command: 'codex', runner });

    expect(result.status).toBe('failed');
    expect(result.statusReason).toBe('Codex process exited with code 2');
    expect(result.stdoutJsonl).toBe(stdout);
    expect(result.stderr).toBe('process exited badly');
    expect(result.summary.terminalStatus).toEqual({ kind: 'completed', lineNumber: 2 });
  });

  it('classifies parseable output without a terminal event as failed', async () => {
    const stdout = jsonl([
      { type: 'thread.started', thread_id: 'thread-789' },
      { type: 'item.completed', item: { type: 'agent_message', text: 'Almost done' } },
    ]);
    const runner = new FakeCodexProcessRunner({ stdout, stderr: '', exitCode: 0 });

    const result = await runCodexExec({ prompt: 'Try it' }, { command: 'codex', runner });

    expect(result.status).toBe('failed');
    expect(result.statusReason).toBe('Codex output did not include a terminal event');
    expect(result.summary.finalAgentMessageText).toBe('Almost done');
  });

  it('classifies process signal exits as failed even when JSONL completed', async () => {
    const stdout = jsonl([{ type: 'turn.completed' }]);
    const runner = new FakeCodexProcessRunner({
      stdout,
      stderr: 'terminated',
      exitCode: null,
      signal: 'SIGTERM',
    });

    const result = await runCodexExec({ prompt: 'Try it' }, { command: 'codex', runner });

    expect(result.status).toBe('failed');
    expect(result.statusReason).toBe('Codex process exited on signal SIGTERM');
    expect(result.exitCode).toBeNull();
    expect(result.signal).toBe('SIGTERM');
    expect(result.summary.terminalStatus).toEqual({ kind: 'completed', lineNumber: 1 });
  });

  it('classifies Codex turn.failed events as failed even with a zero process exit', async () => {
    const stdout = jsonl([{ type: 'turn.failed', error: { message: 'model failed' } }]);
    const runner = new FakeCodexProcessRunner({ stdout, stderr: '', exitCode: 0 });

    const result = await runCodexExec({ prompt: 'Try it' }, { command: 'codex', runner });

    expect(result.status).toBe('failed');
    expect(result.statusReason).toBe('Codex emitted a turn.failed event');
  });

  it('classifies Codex error events as error', async () => {
    const stdout = jsonl([{ type: 'error', message: 'fatal' }]);
    const runner = new FakeCodexProcessRunner({ stdout, stderr: '', exitCode: 0 });

    const result = await runCodexExec({ prompt: 'Try it' }, { command: 'codex', runner });

    expect(result.status).toBe('error');
    expect(result.statusReason).toBe('Codex emitted an error event');
  });

  it('throws parser errors when stdout is not trustworthy JSONL', async () => {
    const runner = new FakeCodexProcessRunner({ stdout: '{', stderr: 'stderr still captured' });

    try {
      await runCodexExec({ prompt: 'Try it' }, { command: 'codex', runner });
    } catch (error) {
      expect(error).toBeInstanceOf(CodexJsonlParseError);
      expect((error as CodexJsonlParseError).lineNumber).toBe(1);
      expect((error as CodexJsonlParseError).message).toContain('Invalid JSON');
      return;
    }

    throw new Error('Expected CodexJsonlParseError');
  });

  it('propagates runner launch failures', async () => {
    const runner: CodexProcessRunner = {
      run: async () => {
        throw new Error('spawn EACCES');
      },
    };

    await expect(runCodexExec({ prompt: 'Try it' }, { command: 'codex', runner })).rejects.toThrow(
      'spawn EACCES',
    );
  });
});

describe('createCodexRuntime', () => {
  it('uses the configured command and runner', async () => {
    const runner = new FakeCodexProcessRunner({
      stdout: jsonl([{ type: 'turn.completed' }]),
      stderr: '',
    });
    const runtime = createCodexRuntime({ command: 'codex-preview', runner });

    const result = await runtime.exec({ prompt: 'Hello' });

    expect(result.command).toBe('codex-preview');
    expect(runner.inputs[0]?.command).toBe('codex-preview');
  });
});

class FakeCodexProcessRunner implements CodexProcessRunner {
  readonly inputs: CodexProcessRunInput[] = [];

  constructor(private readonly result: Partial<CodexProcessRunResult>) {}

  async run(input: CodexProcessRunInput): Promise<CodexProcessRunResult> {
    this.inputs.push(input);
    const stdout = this.result.stdout ?? '';
    const stderr = this.result.stderr ?? '';
    input.onStdoutChunk?.(stdout);
    input.onStderrChunk?.(stderr);

    return {
      stdout,
      stderr,
      exitCode: this.result.exitCode === undefined ? 0 : this.result.exitCode,
      signal: this.result.signal === undefined ? null : this.result.signal,
    };
  }
}

function jsonl(events: readonly object[]): string {
  return events.map((event) => JSON.stringify(event)).join('\n');
}
