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
import { useAgentSessionController } from './useAgentSessionController';

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

class FakeAgentSessionClient implements AgentSessionClient {
  calls: string[] = [];
  sent: SendAgentSessionMessageCommandDto[] = [];
  canceled: CancelAgentInvocationCommandDto[] = [];
  listener?: AgentSessionUpdateListener;
  private empty: boolean;
  private running: boolean;

  constructor(options: { empty?: boolean; running?: boolean } = {}) {
    this.empty = options.empty ?? false;
    this.running = options.running ?? false;
  }
  async createSession(): Promise<AgentSessionDto> {
    return sessionDetails().session;
  }
  async listSessions() {
    this.calls.push('list');
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
