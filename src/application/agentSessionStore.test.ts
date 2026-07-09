import type { EntityId } from '../domain/model';
import {
  createAgentSessionTurnRecord,
  createEmptyAgentSessionRecord,
  InMemoryAgentSessionDurableStore,
} from './agentSessionStore';

describe('InMemoryAgentSessionDurableStore', () => {
  it('appends turns, updates latest status, and returns defensive copies', () => {
    const store = new InMemoryAgentSessionDurableStore();
    const sessionId = 'agent-session-store' as EntityId;
    store.saveSession(createEmptyAgentSessionRecord(sessionId));

    const firstTurn = createAgentSessionTurnRecord({
      id: 'turn-1' as EntityId,
      prompt: 'First',
    });
    store.appendTurn(sessionId, firstTurn);
    store.updateTurn(sessionId, firstTurn.id, {
      status: 'completed',
      output: [
        {
          id: 'chunk-1' as EntityId,
          stream: 'stdout',
          content: 'done',
          receivedAt: '2026-07-09T10:00:00.000Z',
        },
      ],
    });

    const loaded = store.loadSession(sessionId);
    expect(loaded).toMatchObject({
      id: sessionId,
      status: 'completed',
      turns: [{ id: firstTurn.id, prompt: 'First', status: 'completed' }],
    });

    loaded?.turns.push(createAgentSessionTurnRecord({ id: 'mutated' as EntityId, prompt: 'Nope' }));
    expect(store.loadSession(sessionId)?.turns).toHaveLength(1);
  });

  it('renames pending sessions to the real CLI session id without losing turns', () => {
    const store = new InMemoryAgentSessionDurableStore();
    const pendingId = 'agent-session-pending' as EntityId;
    const realId = 'agent-session-real' as EntityId;
    store.saveSession(createEmptyAgentSessionRecord(pendingId));
    store.appendTurn(
      pendingId,
      createAgentSessionTurnRecord({ id: 'turn-1' as EntityId, prompt: 'Hello' }),
    );

    const renamed = store.renameSession(pendingId, realId);

    expect(renamed.id).toBe(realId);
    expect(renamed.turns).toMatchObject([{ prompt: 'Hello' }]);
    expect(store.loadSession(pendingId)).toBeNull();
    expect(store.loadSession(realId)?.turns).toHaveLength(1);
  });
});
