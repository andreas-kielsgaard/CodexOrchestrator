import type {
  AgentSessionDetailsDto,
  AgentSessionUpdateDto,
} from '../../application/agentSessions';
import {
  AGENT_SESSION_UPDATE_EVENT,
  createTauriAgentSessionClient,
} from './tauriAgentSessionClient';

describe('Tauri Agent Session client', () => {
  it('completes listener registration before invoking send and keeps both correlation IDs', async () => {
    let finishListen: ((unlisten: () => void) => void) | undefined;
    const calls: string[] = [];
    const client = createTauriAgentSessionClient({
      listen: () =>
        new Promise((resolve) => {
          calls.push('listen');
          finishListen = resolve;
        }),
      invoke: async <T>(command: string) => {
        calls.push(command);
        return { sessionId: 'session-1', invocationId: 'invocation-1' } as T;
      },
    });

    const pending = client.sendMessage({ submittedText: 'Start now' });
    await Promise.resolve();
    expect(calls).toEqual(['listen']);

    finishListen?.(() => undefined);
    await expect(pending).resolves.toEqual({
      sessionId: 'session-1',
      invocationId: 'invocation-1',
    });
    expect(calls).toEqual(['listen', 'send_agent_session_message']);
  });

  it('correlates persisted updates and repairs a missed notification through explicit reload', async () => {
    let eventHandler: ((event: { payload: AgentSessionUpdateDto }) => void) | undefined;
    const received: AgentSessionUpdateDto[] = [];
    const durable = completedDetails();
    const client = createTauriAgentSessionClient({
      listen: async (event, handler) => {
        expect(event).toBe(AGENT_SESSION_UPDATE_EVENT);
        eventHandler = handler as (event: { payload: AgentSessionUpdateDto }) => void;
        return () => undefined;
      },
      invoke: async <T>(command: string) => {
        if (command === 'load_agent_session') {
          return durable as T;
        }
        throw new Error(`unexpected command ${command}`);
      },
    });
    const unsubscribe = await client.subscribeUpdates((update) => received.push(update));

    eventHandler?.({
      payload: {
        kind: 'invocation_terminal',
        sessionId: 'other-session',
        invocationId: 'other-invocation',
        invocation: durable.invocations[0].invocation,
      },
    });
    expect(received[0]).toMatchObject({ invocationId: 'other-invocation' });

    // No event is delivered for session-1. Durable reload still observes completion.
    await expect(client.reloadSession({ sessionId: 'session-1' })).resolves.toEqual(durable);
    unsubscribe();
    await client.disconnectUpdates();
  });
});

function completedDetails(): AgentSessionDetailsDto {
  const timestamp = '2026-07-10T12:00:00Z';
  return {
    session: {
      id: 'session-1',
      title: 'Session',
      availability: 'available',
      runtimeBinding: {
        kind: 'codex_cli',
        externalContextId: 'thread-1',
        runtimeVersion: 'codex-test',
      },
      workingDirectory: null,
      requestedOptions: { model: null, sandbox: null },
      createdAt: timestamp,
      updatedAt: timestamp,
    },
    invocations: [
      {
        invocation: {
          id: 'invocation-1',
          sessionId: 'session-1',
          submittedText: 'Hello',
          status: 'completed',
          requestedOptions: { model: null, sandbox: null },
          effectiveOptions: { model: null, sandbox: null },
          startedAt: timestamp,
          completedAt: timestamp,
          exitCode: 0,
          signal: null,
          runtimeError: null,
          diagnostics: [],
          createdAt: timestamp,
          updatedAt: timestamp,
        },
        events: [],
      },
    ],
  };
}
