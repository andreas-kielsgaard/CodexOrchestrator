import type { EntityId, IsoDateTime } from '../domain/model';
import { AgentCLISessionInterface } from './agentCLISessionInterface';
import { AgentSession } from './agentSession';
import { AgentSessionRouter } from './agentSessionRouter';
import { CLIInstanceHandler } from './cliInstanceHandler';
import { InMemoryAgentSessionDurableStore } from './agentSessionStore';
import type {
  CLIInstanceOpenInput,
  CLIInstanceRunResult,
  CLIInstanceRunner,
  CLIOutputChunk,
} from './cliInstanceHandler';

describe('AgentSessionRouter', () => {
  it('creates an Agent session handler, delivers the prompt, and formats CLI output', async () => {
    const runner = new FakeCLIInstanceRunner();
    const store = new InMemoryAgentSessionDurableStore();
    const router = new AgentSessionRouter(
      (sessionId) =>
        new AgentSession(
          sessionId,
          store,
          new AgentCLISessionInterface(new CLIInstanceHandler(runner)),
        ),
    );
    const updates: string[] = [];

    const result = await router.launch(
      {
        sessionId: 'agent-session-42' as EntityId,
        prompt: 'Explain this codebase',
        additionalArgs: ['--model', 'gpt-5.5'],
      },
      (viewModel) => updates.push(viewModel.status),
    );

    expect(result).toMatchObject({
      sessionId: 'agent-session-42',
      status: 'completed',
      commandLine:
        'codex exec --json --model gpt-5.5 resume agent-session-42 "Explain this codebase"',
      promptText: 'Explain this codebase',
      items: [
        { kind: 'user-message', text: 'Explain this codebase' },
        {
          kind: 'finished-turn',
          text: 'Finished turn',
          finalText: 'Agent response line',
          expanded: false,
        },
      ],
      metadata: {
        approval: 'never',
        codexSessionId: 'thread-42',
        model: 'gpt-5.5',
        sandbox: 'danger-full-access',
        terminalStatus: 'completed',
      },
      exitCode: 0,
    });
    expect(runner.inputs).toEqual([
      {
        sessionId: 'agent-session-42',
        command: 'codex',
        args: [
          'exec',
          '--json',
          '--model',
          'gpt-5.5',
          'resume',
          'agent-session-42',
          'Explain this codebase',
        ],
      },
    ]);
    expect(updates).toContain('running');
    expect(updates).toContain('completed');
    expect(router.reload('agent-session-42' as EntityId).status).toBe('completed');
  });

  it('appends later prompts to the durable Agent session instead of replacing prior turns', async () => {
    const runner = new FakeCLIInstanceRunner();
    const store = new InMemoryAgentSessionDurableStore();
    const router = new AgentSessionRouter(
      (sessionId) =>
        new AgentSession(
          sessionId,
          store,
          new AgentCLISessionInterface(new CLIInstanceHandler(runner)),
        ),
    );

    const first = await router.launch({
      sessionId: 'agent-session-append' as EntityId,
      prompt: 'First prompt',
    });
    const second = await router.launch({
      sessionId: first.sessionId as EntityId,
      prompt: 'Second prompt',
    });

    expect(second.items).toMatchObject([
      { kind: 'user-message', text: 'First prompt' },
      { kind: 'finished-turn', finalText: 'Response to First prompt' },
      { kind: 'user-message', text: 'Second prompt' },
      { kind: 'finished-turn', finalText: 'Response to Second prompt' },
    ]);
  });
});

class FakeCLIInstanceRunner implements CLIInstanceRunner {
  inputs: CLIInstanceOpenInput[] = [];

  async run(
    input: CLIInstanceOpenInput,
    onOutput: (chunk: Omit<CLIOutputChunk, 'id' | 'receivedAt'>) => void,
  ): Promise<CLIInstanceRunResult> {
    this.inputs.push(input);
    onOutput({ stream: 'system', content: 'Launching fake CLI' });

    return {
      sessionId: input.sessionId ?? ('agent-session-created' as EntityId),
      status: 'completed',
      command: input.command,
      args: [...input.args],
      stdout: [
        JSON.stringify({ type: 'thread.started', thread_id: 'thread-42' }),
        JSON.stringify({ type: 'turn.started' }),
        JSON.stringify({
          type: 'item.completed',
          item: {
            type: 'agent_message',
            text:
              input.args.at(-1) === 'Explain this codebase'
                ? 'Agent response line'
                : `Response to ${input.args.at(-1)}`,
          },
        }),
        JSON.stringify({ type: 'turn.completed', usage: { total_tokens: 42 } }),
      ].join('\n'),
      stderr: [
        'OpenAI Codex v0.130.0-alpha.5',
        '--------',
        'model: gpt-5.5',
        'approval: never',
        'sandbox: danger-full-access',
        'session id: stderr-session',
      ].join('\n'),
      startedAt: '2026-07-03T10:00:00.000Z' as IsoDateTime,
      completedAt: '2026-07-03T10:01:00.000Z' as IsoDateTime,
      exitCode: 0,
    };
  }
}
