import type { EntityId, IsoDateTime } from '../domain/model';
import type { CLIInstanceSnapshot } from './cliInstanceHandler';
import { AgentSessionOutputFormatter } from './agentSessionOutputFormatter';

describe('AgentSessionOutputFormatter', () => {
  it('collapses started and completed items inside an expanded finished turn', () => {
    const formatter = new AgentSessionOutputFormatter();
    const snapshot: CLIInstanceSnapshot = {
      sessionId: 'agent-session-1' as EntityId,
      status: 'completed',
      command: 'codex',
      args: ['exec', '--json', 'Find the answer'],
      startedAt: '2026-07-09T10:00:00.000Z' as IsoDateTime,
      completedAt: '2026-07-09T10:00:05.000Z' as IsoDateTime,
      output: [
        {
          id: 'chunk-1' as EntityId,
          stream: 'stdout',
          receivedAt: '2026-07-09T10:00:01.000Z' as IsoDateTime,
          content: JSON.stringify({ type: 'turn.started' }),
        },
        {
          id: 'chunk-2' as EntityId,
          stream: 'stdout',
          receivedAt: '2026-07-09T10:00:02.000Z' as IsoDateTime,
          content: JSON.stringify({
            type: 'item.started',
            item: { id: 'item_1', type: 'web_search', query: '' },
          }),
        },
        {
          id: 'chunk-3' as EntityId,
          stream: 'stdout',
          receivedAt: '2026-07-09T10:00:03.000Z' as IsoDateTime,
          content: JSON.stringify({
            type: 'item.completed',
            item: {
              id: 'item_1',
              type: 'web_search',
              query: '2026 Met Gala best dressed',
              action: { type: 'search', query: '2026 Met Gala best dressed' },
            },
          }),
        },
        {
          id: 'chunk-4' as EntityId,
          stream: 'stdout',
          receivedAt: '2026-07-09T10:00:04.000Z' as IsoDateTime,
          content: JSON.stringify({
            type: 'item.completed',
            item: { id: 'item_2', type: 'agent_message', text: '**Rihanna** looked great.' },
          }),
        },
        {
          id: 'chunk-5' as EntityId,
          stream: 'stdout',
          receivedAt: '2026-07-09T10:00:05.000Z' as IsoDateTime,
          content: JSON.stringify({
            type: 'turn.completed',
            usage: { input_tokens: 1234, cached_input_tokens: 100 },
          }),
        },
      ],
    };

    const viewModel = formatter.format(snapshot, { expandedTurnIds: new Set(['turn-1']) });
    const turn = viewModel.items.find((item) => item.kind === 'finished-turn');

    expect(turn).toMatchObject({
      kind: 'finished-turn',
      finalText: '**Rihanna** looked great.',
      expanded: true,
      hiddenItems: [
        {
          kind: 'item',
          itemType: 'web_search',
          text: 'Web search: 2026 Met Gala best dressed',
          processing: false,
        },
      ],
    });
    expect(viewModel.contextSize).toBe('1,234 tokens, 100 cached');
  });
});
