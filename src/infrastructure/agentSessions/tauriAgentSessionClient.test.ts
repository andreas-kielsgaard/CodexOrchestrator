import type {
  AgentSessionDetailsDto,
  AgentSessionUpdateDto,
} from '../../application/agentSessions';
import {
  AGENT_SESSION_UPDATE_EVENT,
  createTauriAgentSessionClient,
} from './tauriAgentSessionClient';

describe('Tauri Agent Session client', () => {
  it('retries an unacknowledged queue mutation with the same identity', async () => {
    const invoke = vi
      .fn()
      .mockRejectedValueOnce(new Error('connection lost'))
      .mockResolvedValue({});
    const client = createTauriAgentSessionClient({ invoke });
    await expect(client.enqueueCompaction!('session-1')).rejects.toThrow('connection lost');
    await client.enqueueCompaction!('session-1');
    expect(invoke.mock.calls[1]).toEqual(invoke.mock.calls[0]);
    await client.enqueueCompaction!('session-1');
    expect(invoke.mock.calls[2][1].input.submissionId).not.toEqual(
      invoke.mock.calls[0][1].input.submissionId,
    );
  });
  it('renames a Session through the dedicated command', async () => {
    const invoke = vi.fn().mockResolvedValue(completedDetails().session);
    const client = createTauriAgentSessionClient({ invoke });

    await client.renameSession?.({ sessionId: 'session-1', title: 'Planning notes' });

    expect(invoke).toHaveBeenCalledWith('rename_agent_session', {
      input: { sessionId: 'session-1', title: 'Planning notes' },
    });
  });

  it('sends Session-owned model selection and clearing through the dedicated command', async () => {
    const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
    const client = createTauriAgentSessionClient({
      invoke: async <T>(command: string, args?: Record<string, unknown>) => {
        calls.push({ command, args });
        return completedDetails().session as T;
      },
    });

    await client.updateModelOverride?.({ sessionId: 'session-1', model: 'selected-model' });
    await client.updateModelOverride?.({ sessionId: 'session-1', model: null });

    expect(calls).toEqual([
      {
        command: 'update_agent_session_model_override',
        args: { input: { sessionId: 'session-1', model: 'selected-model' } },
      },
      {
        command: 'update_agent_session_model_override',
        args: { input: { sessionId: 'session-1', model: null } },
      },
    ]);
  });

  it('completes listener registration before invoking send and keeps both correlation IDs', async () => {
    const finishListen: Array<(unlisten: () => void) => void> = [];
    const calls: string[] = [];
    const client = createTauriAgentSessionClient({
      listen: () =>
        new Promise((resolve) => {
          calls.push('listen');
          finishListen.push(resolve);
        }),
      invoke: async <T>(command: string) => {
        calls.push(command);
        return { sessionId: 'session-1', invocationId: 'invocation-1' } as T;
      },
    });

    const pending = client.sendMessage({ submittedText: 'Start now' });
    await Promise.resolve();
    expect(calls).toEqual(['listen', 'listen']);

    finishListen.forEach((finish) => finish(() => undefined));
    await expect(pending).resolves.toEqual({
      sessionId: 'session-1',
      invocationId: 'invocation-1',
    });
    expect(calls).toEqual(['listen', 'listen', 'send_agent_session_message']);
  });

  it('correlates persisted updates and repairs a missed notification through explicit reload', async () => {
    let eventHandler: ((event: { payload: AgentSessionUpdateDto }) => void) | undefined;
    const received: AgentSessionUpdateDto[] = [];
    const durable = completedDetails();
    const client = createTauriAgentSessionClient({
      listen: async (event, handler) => {
        if (event === AGENT_SESSION_UPDATE_EVENT)
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
          inputProvenance: 'user',
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
        observation: {
          launchAcceptedAt: null,
          externalContext: null,
          providerActivity: null,
          providerTerminal: null,
          processTerminal: {
            status: 'completed',
            completedAt: timestamp,
            exitCode: 0,
            signal: null,
          },
          mcpToolActivities: [],
        },
        events: [],
      },
    ],
  };
}
