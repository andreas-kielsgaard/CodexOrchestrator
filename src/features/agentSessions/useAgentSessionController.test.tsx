import { act, renderHook, waitFor } from '@testing-library/react';
import type {
  AgentInvocationDto,
  AgentSessionClient,
  AgentSessionDetailsDto,
  AgentSessionDto,
  AgentSessionUpdateListener,
  CancelAgentInvocationCommandDto,
  LoadAgentSessionQueryDto,
  SendAgentSessionMessageCommandDto,
} from '../../application/agentSessions';
import { sessionDetails, sessionSummary } from './testFixtures';
import {
  useAgentSession,
  useAgentSessionCollection,
  useAgentSessionController,
} from './useAgentSessionController';

describe('useAgentSessionController', () => {
  it('subscribes before opening and loads durable state to close notification gaps', async () => {
    const client = new FakeAgentSessionClient();
    const { result } = renderHook(() => useAgentSessionController(client));

    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(client.calls.slice(0, 3)).toEqual(['subscribe', 'list', 'load:session-1']);
    expect(result.current.details?.session.id).toBe('session-1');
  });

  it('releases a delayed subscription that resolves after unmount', async () => {
    let resolveSubscription: ((unsubscribe: () => void) => void) | undefined;
    const unsubscribe = vi.fn();
    const client = new FakeAgentSessionClient();
    client.subscribeUpdates = vi.fn(
      () =>
        new Promise<() => void>((resolve) => {
          resolveSubscription = resolve;
        }),
    );

    const { unmount } = renderHook(() => useAgentSessionController(client));
    unmount();

    await act(async () => {
      resolveSubscription?.(unsubscribe);
      await Promise.resolve();
    });

    expect(unsubscribe).toHaveBeenCalledOnce();
    expect(client.calls).not.toContain('list');
  });

  it('lazily creates a first session through send, selects acknowledged IDs, and reloads', async () => {
    const client = new FakeAgentSessionClient({ empty: true });
    const { result } = renderHook(() => useAgentSessionController(client));
    await waitFor(() => expect(result.current.loading).toBe(false));

    act(() => {
      result.current.setDraft('  First request  ');
      result.current.setWorkingDirectory(' C:/new-workspace ');
    });
    await act(() => result.current.send());

    expect(client.sent).toEqual([
      { submittedText: 'First request', workingDirectory: 'C:/new-workspace' },
    ]);
    expect(client.calls).toContain('reload:session-1');
    expect(result.current.selectedSessionId).toBe('session-1');
    expect(result.current.draft).toBe('');
  });

  it('reconciles correlated updates from durable reload and refreshes terminal summaries', async () => {
    const client = new FakeAgentSessionClient();
    const { result } = renderHook(() => useAgentSessionController(client));
    await waitFor(() => expect(result.current.details).not.toBeNull());

    client.listener?.({
      kind: 'invocation_terminal',
      sessionId: 'session-1',
      invocationId: 'other-invocation',
      invocation: sessionDetails('completed').invocations[0].invocation,
    });
    await Promise.resolve();
    expect(client.calls).not.toContain('reload:session-1');

    await act(async () => client.emitTerminal());
    await waitFor(() =>
      expect(client.calls.filter((call) => call === 'reload:session-1')).toHaveLength(1),
    );
    expect(client.calls.filter((call) => call === 'list')).toHaveLength(2);
  });

  it('owns processing expansion and cancellation for the active invocation', async () => {
    const client = new FakeAgentSessionClient({ running: true });
    const { result } = renderHook(() => useAgentSessionController(client));
    await waitFor(() => expect(result.current.transcript?.activeInvocationId).toBe('invocation-1'));

    act(() => result.current.toggleProcessing('invocation-1'));
    expect(result.current.expandedProcessing.has('invocation-1')).toBe(true);
    await act(() => result.current.cancel());
    expect(client.canceled).toEqual([{ invocationId: 'invocation-1' }]);
  });
});

describe('extracted Agent Session boundaries', () => {
  it('loads collection state without subscribing or loading a session', async () => {
    const client = new FakeAgentSessionClient();
    const { result } = renderHook(() => useAgentSessionCollection(client));
    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(client.calls).toEqual(['list']);
    expect(result.current.selectedSessionId).toBe('session-1');
  });

  it('mounts a controlled session without listing collection state', async () => {
    const client = new FakeAgentSessionClient();
    const { result } = renderHook(() =>
      useAgentSession(client, { selectedSessionId: 'session-1' }),
    );
    await waitFor(() => expect(result.current.details).not.toBeNull());
    expect(client.calls.slice(0, 2)).toEqual(['subscribe', 'load:session-1']);
    expect(client.calls).not.toContain('list');
  });

  it('attributes collection reload failures to the collection without session work', async () => {
    const client = new FakeAgentSessionClient({ listError: new Error('list unavailable') });
    const { result } = renderHook(() => useAgentSessionCollection(client));

    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.error).toBe('Session list reload failed: list unavailable');
    expect(client.calls).toEqual(['list']);
  });

  it('responds to controlled selected-session changes without listing collection state', async () => {
    const client = new FakeAgentSessionClient();
    const { result, rerender } = renderHook(
      ({ selectedSessionId }) => useAgentSession(client, { selectedSessionId }),
      { initialProps: { selectedSessionId: 'session-1' as string | null } },
    );
    await waitFor(() => expect(result.current.details?.session.id).toBe('session-1'));

    rerender({ selectedSessionId: 'session-2' });
    await waitFor(() => expect(result.current.selectedSessionId).toBe('session-2'));
    expect(client.calls).toContain('load:session-2');

    rerender({ selectedSessionId: null });
    await waitFor(() => expect(result.current.selectedSessionId).toBeNull());
    expect(result.current.details).toBeNull();
    expect(client.calls).not.toContain('list');
  });

  it('notifies the collection owner when a controlled new session is acknowledged', async () => {
    const client = new FakeAgentSessionClient({ empty: true });
    const onSessionCreated = vi.fn();
    const { result } = renderHook(() =>
      useAgentSession(client, { selectedSessionId: null, onSessionCreated }),
    );
    await waitFor(() => expect(result.current.loading).toBe(false));

    act(() => result.current.setDraft('First controlled request'));
    await act(() => result.current.send());

    expect(onSessionCreated).toHaveBeenCalledWith('session-1');
    expect(result.current.selectedSessionId).toBe('session-1');
    expect(client.calls).not.toContain('list');
  });
});

class FakeAgentSessionClient implements AgentSessionClient {
  calls: string[] = [];
  sent: SendAgentSessionMessageCommandDto[] = [];
  canceled: CancelAgentInvocationCommandDto[] = [];
  listener?: AgentSessionUpdateListener;
  private empty: boolean;
  private running: boolean;

  private listError?: Error;

  constructor(options: { empty?: boolean; running?: boolean; listError?: Error } = {}) {
    this.empty = options.empty ?? false;
    this.running = options.running ?? false;
    this.listError = options.listError;
  }
  async createSession(): Promise<AgentSessionDto> {
    return sessionDetails().session;
  }
  async listSessions() {
    this.calls.push('list');
    if (this.listError) throw this.listError;
    return this.empty ? [] : [sessionSummary(this.running)];
  }
  async loadSession(query: LoadAgentSessionQueryDto) {
    this.calls.push(`load:${query.sessionId}`);
    return this.currentDetails();
  }
  async reloadSession(query: LoadAgentSessionQueryDto) {
    this.calls.push(`reload:${query.sessionId}`);
    return this.currentDetails();
  }
  async subscribeUpdates(listener: AgentSessionUpdateListener) {
    this.calls.push('subscribe');
    this.listener = listener;
    return () => {
      this.listener = undefined;
    };
  }
  async sendMessage(command: SendAgentSessionMessageCommandDto) {
    this.calls.push('send');
    this.sent.push(command);
    this.empty = false;
    this.running = true;
    return { sessionId: 'session-1', invocationId: 'invocation-1' };
  }
  async cancelInvocation(command: CancelAgentInvocationCommandDto): Promise<AgentInvocationDto> {
    this.canceled.push(command);
    this.running = false;
    return sessionDetails('canceled').invocations[0].invocation;
  }
  async disconnectUpdates() {}
  async emitTerminal() {
    this.listener?.({
      kind: 'invocation_terminal',
      sessionId: 'session-1',
      invocationId: 'invocation-1',
      invocation: sessionDetails('completed').invocations[0].invocation,
    });
    await Promise.resolve();
  }
  private currentDetails(): AgentSessionDetailsDto {
    return sessionDetails(this.running ? 'running' : 'completed');
  }
}
