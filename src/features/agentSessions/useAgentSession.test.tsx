import { useState } from 'react';
import { act, renderHook, waitFor } from '@testing-library/react';
import { beforeEach } from 'vitest';
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
import { useAgentSession, type UseAgentSessionOptions } from './useAgentSession';
import { useAgentSessionCollection } from './useAgentSessionCollection';
import {
  composerDraftCacheKey,
  readCachedComposerDraft,
  writeCachedComposerDraft,
} from './composerDraftCache';

beforeEach(() => window.localStorage.clear());

describe('conversation state', () => {
  it('does not let a late steering acknowledgement invalidate navigation or clear another draft', async () => {
    let acknowledge!: () => void;
    const client = Object.assign(new FakeAgentSessionClient({ running: true }), {
      steerSession: vi.fn(
        (input: { inputId: string }) =>
          new Promise<import('../../application/agentSessions').SessionInteractionDto>(
            (resolve) => {
              acknowledge = () =>
                resolve({
                  id: input.inputId,
                  invocationId: 'invocation-1',
                  sequence: 3,
                  kind: 'steering',
                  state: 'accepted',
                  content: { text: 'Correction' },
                  result: null,
                });
            },
          ),
      ),
    });
    let loadSecond!: (details: AgentSessionDetailsDto) => void;
    const second = sessionDetails('completed');
    second.session.id = 'session-2';
    vi.spyOn(client, 'loadSession').mockImplementation(async ({ sessionId }) =>
      sessionId === 'session-2'
        ? new Promise<AgentSessionDetailsDto>((resolve) => {
            loadSecond = resolve;
          })
        : sessionDetails('running'),
    );
    const reload = vi.spyOn(client, 'reloadSession');
    const { result, rerender } = renderHook(
      ({ sessionId }) => useAgentSession(client, { selectedSessionId: sessionId }),
      { initialProps: { sessionId: 'session-1' } },
    );
    await waitFor(() => expect(result.current.loading).toBe(false));
    act(() => result.current.setDraft('Correction'));
    let steering!: Promise<void>;
    act(() => {
      steering = result.current.steer!();
    });
    rerender({ sessionId: 'session-2' });
    await waitFor(() => expect(loadSecond).toBeDefined());
    act(() => result.current.setDraft('Correction'));
    await act(async () => {
      acknowledge();
      await steering;
    });
    await act(async () => {
      loadSecond(second);
    });
    expect(result.current.details?.session.id).toBe('session-2');
    expect(result.current.draft).toBe('Correction');
    expect(reload).not.toHaveBeenCalled();
  });

  it('debounces draft storage and flushes the last edit on unmount', async () => {
    const client = new FakeAgentSessionClient();
    const { result, unmount } = renderHook(() => useConversationHost(client));
    await waitFor(() => expect(result.current.loading).toBe(false));
    const writes = vi.spyOn(Storage.prototype, 'setItem');
    vi.useFakeTimers();
    try {
      writes.mockClear();
      for (let i = 1; i <= 8; i++) act(() => result.current.setDraft('x'.repeat(i)));
      expect(writes).not.toHaveBeenCalled();
      act(() => vi.advanceTimersByTime(500));
      expect(writes).toHaveBeenCalledTimes(1);
      act(() => result.current.setDraft('Final unsaved edit'));
      unmount();
      expect(readCachedComposerDraft(composerDraftCacheKey('session-1'))?.text).toBe(
        'Final unsaved edit',
      );
    } finally {
      vi.useRealTimers();
      writes.mockRestore();
    }
  });

  it('releases the composer after acceptance without waiting for history reads', async () => {
    const client = new FakeAgentSessionClient();
    const { result } = renderHook(() => useConversationHost(client));
    await waitFor(() => expect(result.current.loading).toBe(false));
    vi.spyOn(client, 'reloadSession').mockImplementation(() => new Promise(() => {}));
    act(() => result.current.setDraft('Send now'));
    await act(async () => {
      await result.current.send();
    });
    expect(result.current.sending).toBe(false);
    expect(result.current.draft).toBe('');
    act(() => result.current.setDraft('Still responsive'));
    expect(result.current.draft).toBe('Still responsive');
  });

  it('steers an active turn through the shared client without another domain delivery', async () => {
    let acceptSteering!: (value: {
      id: string;
      invocationId: string;
      sequence: number;
      kind: 'steering';
      state: 'accepted';
      content: { text: string };
      result: null;
    }) => void;
    const steerSession = vi.fn((command: { inputId: string }) => {
      expect(command.inputId).toBeTruthy();
      return new Promise<Parameters<typeof acceptSteering>[0]>((resolve) => {
        acceptSteering = resolve;
      });
    });
    const client = Object.assign(new FakeAgentSessionClient({ running: true }), { steerSession });
    const sendExistingMessage = vi.fn();
    const { result } = renderHook(() => useConversationHost(client, { sendExistingMessage }));
    await waitFor(() => expect(result.current.loading).toBe(false));
    act(() => result.current.setDraft('Change direction'));
    let steering!: Promise<void>;
    act(() => {
      steering = result.current.steer?.() ?? Promise.resolve();
    });
    await waitFor(() => expect(result.current.steeringQueue).toHaveLength(1));
    expect(result.current.steeringQueue[0]).toMatchObject({
      state: 'pending',
      content: { text: 'Change direction' },
    });
    expect(steerSession).toHaveBeenCalledWith({
      sessionId: 'session-1',
      invocationId: 'invocation-1',
      inputId: expect.any(String),
      text: 'Change direction',
    });
    await act(async () => {
      const inputId = steerSession.mock.calls[0][0].inputId;
      acceptSteering({
        id: inputId,
        invocationId: 'invocation-1',
        sequence: 3,
        kind: 'steering',
        state: 'accepted',
        content: { text: 'Change direction' },
        result: null,
      });
      await steering;
    });
    expect(sendExistingMessage).not.toHaveBeenCalled();
    expect(client.sent).toEqual([]);
    expect(result.current.draft).toBe('');
    expect(result.current.steeringQueue).toEqual([]);
  });

  it('retains rejected steering text without sending a fallback turn', async () => {
    const client = Object.assign(new FakeAgentSessionClient({ running: true }), {
      steerSession: vi.fn().mockResolvedValue({ state: 'rejected', result: 'Turn completed' }),
    });
    const { result } = renderHook(() => useConversationHost(client));
    await waitFor(() => expect(result.current.loading).toBe(false));
    act(() => result.current.setDraft('Keep this correction'));
    await act(() => result.current.steer?.());
    expect(result.current.draft).toBe('Keep this correction');
    expect(result.current.error).toBe('Turn completed');
    expect(client.sent).toEqual([]);
  });

  it('does not replace a new draft or selection when an earlier send is acknowledged', async () => {
    let acknowledge!: (result: { sessionId: string; invocationId: string }) => void;
    const sendExistingMessage = () =>
      new Promise<{ sessionId: string; invocationId: string }>((resolve) => {
        acknowledge = resolve;
      });
    const client = new FakeAgentSessionClient();
    const { result } = renderHook(() => useConversationHost(client, { sendExistingMessage }));
    await waitFor(() => expect(result.current.loading).toBe(false));
    act(() => result.current.setDraft('First message'));
    let sending!: Promise<void>;
    act(() => {
      sending = result.current.send();
    });
    await waitFor(() => expect(acknowledge).toBeDefined());
    act(() => {
      result.current.startNewSession();
    });
    act(() => result.current.setDraft('New conversation draft'));
    await act(async () => {
      acknowledge({ sessionId: 'session-1', invocationId: 'later' });
      await sending;
    });
    expect(result.current.selectedSessionId).toBeNull();
    expect(result.current.draft).toBe('New conversation draft');
  });
  it('subscribes before opening and loads durable state to close notification gaps', async () => {
    const client = new FakeAgentSessionClient();
    const { result } = renderHook(() => useConversationHost(client));

    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(client.calls.slice(0, 2)).toEqual(['subscribe', 'load:session-1']);
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

    const { unmount } = renderHook(() => useConversationHost(client));
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
    const { result } = renderHook(() => useConversationHost(client, {}, null));
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

  it('restores a semantic new-conversation draft after navigating away and back', async () => {
    const client = new FakeAgentSessionClient({ empty: true });
    const repository = { kind: 'repository' as const, repositoryId: 'orchestrator' };
    const { result, rerender } = renderHook(
      ({ folderTarget }) =>
        useAgentSession(client, {
          selectedSessionId: null,
          draftId: 'new-conversation',
          folderTarget,
        }),
      {
        initialProps: {
          folderTarget: repository as
            import('../../application/agentSessions/organization').SessionFolderTarget | null,
        },
      },
    );
    await waitFor(() => expect(result.current.loading).toBe(false));

    act(() => {
      result.current.setDraft('Keep this repository draft');
      result.current.setWorkingDirectory('C:/orchestrator');
    });
    rerender({ folderTarget: null });
    await waitFor(() => expect(result.current.draft).toBe(''));

    rerender({ folderTarget: repository });
    await waitFor(() => expect(result.current.draft).toBe('Keep this repository draft'));
    expect(result.current.workingDirectory).toBe('C:/orchestrator');
  });

  it('reconciles correlated updates from durable reload and refreshes terminal summaries', async () => {
    const client = new FakeAgentSessionClient();
    const { result } = renderHook(() => useConversationHost(client));
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
    expect(client.calls).not.toContain('list');
  });

  it('owns processing expansion and cancellation for the active invocation', async () => {
    const client = new FakeAgentSessionClient({ running: true });
    const { result } = renderHook(() => useConversationHost(client));
    await waitFor(() => expect(result.current.transcript?.activeInvocationId).toBe('invocation-1'));

    act(() => result.current.toggleProcessing('invocation-1'));
    expect(result.current.expandedProcessing.has('invocation-1')).toBe(true);
    await act(() => result.current.cancel());
    expect(client.canceled).toEqual([{ invocationId: 'invocation-1' }]);
  });
});

describe('extracted Agent Session boundaries', () => {
  it('loads and subscribes to collection state without loading a session', async () => {
    const client = new FakeAgentSessionClient();
    const { result } = renderHook(() => useAgentSessionCollection(client));
    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(client.calls).toEqual(['list', 'subscribe']);
    expect(result.current.summaries[0].id).toBe('session-1');
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

  it('delegates an existing Session message through the injected replacement boundary', async () => {
    const client = new FakeAgentSessionClient();
    const sendExistingMessage = vi.fn().mockResolvedValue({
      sessionId: 'session-1',
      invocationId: 'profiled-invocation-1',
    });
    const { result } = renderHook(() =>
      useAgentSession(client, { selectedSessionId: 'session-1', sendExistingMessage }),
    );
    await waitFor(() => expect(result.current.details).not.toBeNull());

    act(() => result.current.setDraft('Use the pinned profile'));
    await act(() => result.current.send());

    expect(sendExistingMessage).toHaveBeenCalledWith({
      sessionId: 'session-1',
      submittedText: 'Use the pinned profile',
    });
    expect(client.sent).toEqual([]);
    expect(client.calls).toContain('reload:session-1');
  });

  it('clears submitted text without deleting the accepted conversation model and effort', async () => {
    const client = new FakeAgentSessionClient();
    const key = composerDraftCacheKey('session-1');
    writeCachedComposerDraft(key, {
      text: 'Use Sol',
      runtimeSelection: { model: 'gpt-5.6-sol', reasoningMode: 'high' },
    });
    const { result } = renderHook(() =>
      useAgentSession(client, {
        selectedSessionId: 'session-1',
        sendExistingMessage: vi.fn().mockResolvedValue({
          sessionId: 'session-1',
          invocationId: 'profiled-invocation-1',
        }),
        execution: {
          client: client as never,
          selection: { model: 'gpt-5.6-sol', reasoningMode: 'high' },
          afterAccepted: (sessionId, selection) =>
            writeCachedComposerDraft(composerDraftCacheKey(sessionId), {
              runtimeSelection: selection,
            }),
        },
      }),
    );
    await waitFor(() => expect(result.current.details).not.toBeNull());
    act(() => result.current.setDraft('Use Sol'));

    await act(() => result.current.send());

    expect(readCachedComposerDraft(key)?.runtimeSelection).toEqual({
      model: 'gpt-5.6-sol',
      reasoningMode: 'high',
    });
    expect(readCachedComposerDraft(key)?.text ?? '').toBe('');
  });

  it('attributes collection reload failures to the collection without session work', async () => {
    const client = new FakeAgentSessionClient({ listError: new Error('list unavailable') });
    const { result } = renderHook(() => useAgentSessionCollection(client));

    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.error).toBe('Session list reload failed: list unavailable');
    expect(client.calls).toEqual(['list', 'subscribe']);
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
    // The parent owns selection; acknowledgment does not replace its input.
    expect(result.current.selectedSessionId).toBeNull();
    expect(client.calls).not.toContain('list');
  });
});

function useConversationHost(
  client: AgentSessionClient,
  options: Partial<UseAgentSessionOptions> = {},
  initial: string | null = 'session-1',
) {
  const [selected, setSelected] = useState(initial);
  const [draftId, setDraftId] = useState('draft-1');
  const conversation = useAgentSession(client, {
    ...options,
    selectedSessionId: selected,
    draftId,
    onSessionCreated: setSelected,
  });
  return {
    ...conversation,
    get draft() {
      return conversation.draftSource!.getSnapshot();
    },
    startNewSession: () => {
      setSelected(null);
      setDraftId(crypto.randomUUID());
    },
  };
}

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
