import type { EntityId, IsoDateTime } from '../domain/model';
import { AgentCLISessionInterface, type AgentSessionPromptInput } from './agentCLISessionInterface';
import { AgentSession } from './agentSession';
import {
  CLIInstanceHandler,
  type CLIInstanceSnapshot,
  type CLIInstanceListener,
} from './cliInstanceHandler';
import { InMemoryAgentSessionDurableStore } from './agentSessionStore';

describe('AgentSession', () => {
  it('adopts the real CLI session id on first prompt and appends a later prompt to that session', async () => {
    const store = new InMemoryAgentSessionDurableStore();
    const cli = new FakeAgentCLISessionInterface([
      completedSnapshot({
        sessionId: 'agent-session-real' as EntityId,
        prompt: 'First prompt',
        finalText: 'First answer',
      }),
      completedSnapshot({
        sessionId: 'agent-session-real' as EntityId,
        prompt: 'Second prompt',
        finalText: 'Second answer',
      }),
    ]);
    const session = new AgentSession('agent-session-pending' as EntityId, store, cli);

    const first = await session.deliverPrompt({ prompt: 'First prompt' });
    const second = await session.deliverPrompt({ prompt: 'Second prompt' });

    expect(first.id).toBe('agent-session-real');
    expect(second.id).toBe('agent-session-real');
    expect(second.turns).toMatchObject([
      { prompt: 'First prompt', status: 'completed' },
      { prompt: 'Second prompt', status: 'completed' },
    ]);
    expect(cli.inputs).toEqual([
      { prompt: 'First prompt' },
      { prompt: 'Second prompt', sessionId: 'agent-session-real' },
    ]);
  });

  it('persists failed first turns so the frontend can show the prompt and error', async () => {
    const store = new InMemoryAgentSessionDurableStore();
    const cli = new FakeAgentCLISessionInterface([
      {
        sessionId: 'agent-session-failed' as EntityId,
        status: 'failed',
        command: 'codex',
        args: ['exec', '--json', 'Broken prompt'],
        output: [],
        startedAt: '2026-07-09T10:00:00.000Z' as IsoDateTime,
        completedAt: '2026-07-09T10:00:01.000Z' as IsoDateTime,
        exitCode: 2,
        error: 'Codex session failed with exit code 2',
      },
    ]);
    const session = new AgentSession('agent-session-pending' as EntityId, store, cli);

    const record = await session.deliverPrompt({ prompt: 'Broken prompt' });

    expect(record).toMatchObject({
      id: 'agent-session-failed',
      status: 'failed',
      turns: [
        {
          prompt: 'Broken prompt',
          status: 'failed',
          error: 'Codex session failed with exit code 2',
          exitCode: 2,
        },
      ],
    });
  });
});

class FakeAgentCLISessionInterface extends AgentCLISessionInterface {
  inputs: AgentSessionPromptInput[] = [];

  constructor(private readonly snapshots: CLIInstanceSnapshot[]) {
    super(new CLIInstanceHandler({ run: async () => completedRunResult() }));
  }

  override subscribe(listener: CLIInstanceListener): () => void {
    listener({
      sessionId: null,
      status: 'idle',
      command: null,
      args: [],
      output: [],
    });
    return () => {};
  }

  override async deliverPrompt(input: AgentSessionPromptInput): Promise<CLIInstanceSnapshot> {
    this.inputs.push(input);
    const next = this.snapshots.shift();

    if (!next) {
      throw new Error('No fake CLI snapshot queued.');
    }

    return next;
  }
}

function completedSnapshot(input: {
  sessionId: EntityId;
  prompt: string;
  finalText: string;
}): CLIInstanceSnapshot {
  return {
    sessionId: input.sessionId,
    status: 'completed',
    command: 'codex',
    args: ['exec', '--json', input.prompt],
    output: [
      {
        id: `${input.prompt}-stdout` as EntityId,
        stream: 'stdout',
        receivedAt: '2026-07-09T10:00:00.000Z' as IsoDateTime,
        content: [
          JSON.stringify({ type: 'turn.started' }),
          JSON.stringify({
            type: 'item.completed',
            item: { type: 'agent_message', text: input.finalText },
          }),
          JSON.stringify({ type: 'turn.completed' }),
        ].join('\n'),
      },
    ],
    startedAt: '2026-07-09T10:00:00.000Z' as IsoDateTime,
    completedAt: '2026-07-09T10:00:01.000Z' as IsoDateTime,
    exitCode: 0,
  };
}

function completedRunResult() {
  return {
    sessionId: 'unused' as EntityId,
    status: 'completed' as const,
    command: 'codex',
    args: [],
    stdout: '',
    stderr: '',
    startedAt: '2026-07-09T10:00:00.000Z' as IsoDateTime,
    completedAt: '2026-07-09T10:00:00.000Z' as IsoDateTime,
  };
}
