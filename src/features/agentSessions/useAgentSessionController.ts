import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import type {
  AgentSessionClient,
  AgentSessionDetailsDto,
  AgentSessionSummaryDto,
  AgentSessionUpdateDto,
} from '../../application/agentSessions';
import { projectAgentSessionTranscript } from './transcriptProjector';

export interface AgentSessionController {
  summaries: AgentSessionSummaryDto[];
  selectedSessionId: string | null;
  details: AgentSessionDetailsDto | null;
  transcript: ReturnType<typeof projectAgentSessionTranscript> | null;
  draft: string;
  workingDirectory: string;
  loading: boolean;
  sending: boolean;
  canceling: boolean;
  error: string | null;
  expandedProcessing: ReadonlySet<string>;
  selectSession(sessionId: string): Promise<void>;
  startNewSession(): void;
  setDraft(value: string): void;
  setWorkingDirectory(value: string): void;
  send(): Promise<void>;
  /** Submits caller-supplied text without replacing or clearing the visible composer draft. */
  sendText?(value: string): Promise<void>;
  cancel(): Promise<void>;
  reload(): Promise<void>;
  toggleProcessing(invocationId: string): void;
  clearError(): void;
}

export type AgentSessionCollectionController = Pick<
  AgentSessionController,
  'summaries' | 'selectedSessionId' | 'loading' | 'selectSession' | 'startNewSession' | 'reload'
> & {
  error: string | null;
  clearError(): void;
};
export type AgentSessionWorkspaceController = Omit<
  AgentSessionController,
  'summaries' | 'selectSession' | 'startNewSession'
>;

export function useAgentSessionCollection(
  client: AgentSessionClient,
): AgentSessionCollectionController {
  const [summaries, setSummaries] = useState<AgentSessionSummaryDto[]>([]);
  const [selectedSessionId, setSelectedSessionId] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const reload = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const next = await client.listSessions({ availability: 'available' });
      setSummaries(next);
      setSelectedSessionId((current) => current ?? next[0]?.id ?? null);
    } catch (caught) {
      if (mountedRef.current) setError(`Session list reload failed: ${errorMessage(caught)}`);
    } finally {
      setLoading(false);
    }
  }, [client]);
  const mountedRef = useRef(true);
  useEffect(() => {
    mountedRef.current = true;
    void reload();
    return () => {
      mountedRef.current = false;
    };
  }, [reload]);
  return {
    summaries,
    selectedSessionId,
    loading,
    error,
    selectSession: async (id) => {
      setError(null);
      setSelectedSessionId(id);
    },
    startNewSession: () => {
      setError(null);
      setSelectedSessionId(null);
    },
    reload,
    clearError: () => setError(null),
  };
}

export interface UseAgentSessionOptions {
  selectedSessionId: string | null;
  onSessionCreated?(sessionId: string): void;
  /** Optional managed-composition metadata for first-session creation. */
  sessionTitle?: string;
}

export function useAgentSession(
  client: AgentSessionClient,
  options: UseAgentSessionOptions,
): AgentSessionWorkspaceController {
  return useAgentSessionController(client, {
    controlledSessionId: options.selectedSessionId,
    skipCollection: true,
    onSessionCreated: options.onSessionCreated,
    sessionTitle: options.sessionTitle,
  });
}

interface ControllerOptions {
  controlledSessionId?: string | null;
  skipCollection?: boolean;
  onSessionCreated?(id: string): void;
  sessionTitle?: string;
}
export function useAgentSessionController(
  client: AgentSessionClient,
  options: ControllerOptions = {},
): AgentSessionController {
  const [summaries, setSummaries] = useState<AgentSessionSummaryDto[]>([]);
  const [selectedSessionId, setSelectedSessionId] = useState<string | null>(null);
  const [details, setDetails] = useState<AgentSessionDetailsDto | null>(null);
  const [draft, setDraft] = useState('');
  const [workingDirectory, setWorkingDirectory] = useState('');
  const [loading, setLoading] = useState(true);
  const [sending, setSending] = useState(false);
  const [canceling, setCanceling] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [expandedProcessing, setExpandedProcessing] = useState<Set<string>>(() => new Set());
  const selectedIdRef = useRef<string | null>(null);
  const invocationIdsRef = useRef<Set<string>>(new Set());
  const mountedRef = useRef(true);
  const loadGenerationRef = useRef(0);
  const subscriptionReadyRef = useRef<Promise<void>>(Promise.resolve());

  const refreshSummaries = useCallback(async () => {
    if (options.skipCollection) return [];
    const next = await client.listSessions({ availability: 'available' });
    if (mountedRef.current) setSummaries(next);
    return next;
  }, [client, options.skipCollection]);

  const loadSelected = useCallback(
    async (sessionId: string, reload = false) => {
      const generation = ++loadGenerationRef.current;
      const next = reload
        ? await client.reloadSession({ sessionId })
        : await client.loadSession({ sessionId });
      if (
        mountedRef.current &&
        selectedIdRef.current === sessionId &&
        generation === loadGenerationRef.current
      ) {
        setDetails(next);
        setWorkingDirectory(next.session.workingDirectory ?? '');
        invocationIdsRef.current = new Set(next.invocations.map(({ invocation }) => invocation.id));
      }
      return next;
    },
    [client],
  );

  const reconcileUpdate = useCallback(
    async (update: AgentSessionUpdateDto) => {
      if (
        update.sessionId !== selectedIdRef.current ||
        !invocationIdsRef.current.has(update.invocationId)
      ) {
        return;
      }
      try {
        await loadSelected(update.sessionId, true);
        if (
          update.kind !== 'event_persisted' ||
          update.event.normalized?.kind === 'invocation_completed'
        ) {
          await refreshSummaries();
        }
      } catch (caught) {
        if (mountedRef.current) setError(`Session reload failed: ${errorMessage(caught)}`);
      }
    },
    [loadSelected, refreshSummaries],
  );

  useEffect(() => {
    mountedRef.current = true;
    let canceled = false;
    let unsubscribe: (() => void) | undefined;
    const ready = (async () => {
      unsubscribe = await client.subscribeUpdates((update) => void reconcileUpdate(update));
      if (canceled) {
        unsubscribe();
        unsubscribe = undefined;
        return;
      }
      const nextSummaries = await refreshSummaries();
      if (canceled) return;
      const initialId = options.skipCollection
        ? null
        : (selectedIdRef.current ?? nextSummaries[0]?.id ?? null);
      if (initialId) {
        selectedIdRef.current = initialId;
        setSelectedSessionId(initialId);
        await loadSelected(initialId);
      }
    })()
      .catch((caught) => {
        if (mountedRef.current) setError(errorMessage(caught));
      })
      .finally(() => {
        if (mountedRef.current) setLoading(false);
      });
    subscriptionReadyRef.current = ready;

    return () => {
      canceled = true;
      mountedRef.current = false;
      unsubscribe?.();
    };
  }, [client, loadSelected, options.skipCollection, reconcileUpdate, refreshSummaries]);

  const selectSession = useCallback(
    async (sessionId: string) => {
      selectedIdRef.current = sessionId;
      setSelectedSessionId(sessionId);
      setDetails(null);
      setLoading(true);
      setError(null);
      try {
        await loadSelected(sessionId);
      } catch (caught) {
        if (mountedRef.current) setError(errorMessage(caught));
      } finally {
        if (mountedRef.current) setLoading(false);
      }
    },
    [loadSelected],
  );

  const startNewSession = useCallback(() => {
    loadGenerationRef.current += 1;
    selectedIdRef.current = null;
    setSelectedSessionId(null);
    setDetails(null);
    invocationIdsRef.current = new Set();
    setWorkingDirectory('');
    setDraft('');
    setError(null);
    setLoading(false);
  }, []);

  useEffect(() => {
    if (
      options.controlledSessionId === undefined ||
      options.controlledSessionId === selectedIdRef.current
    )
      return;
    if (options.controlledSessionId) void selectSession(options.controlledSessionId);
    else startNewSession();
  }, [options.controlledSessionId, selectSession, startNewSession]);

  const reload = useCallback(async () => {
    const sessionId = selectedIdRef.current;
    if (!sessionId) {
      await refreshSummaries();
      return;
    }
    setLoading(true);
    setError(null);
    try {
      await loadSelected(sessionId, true);
      await refreshSummaries();
    } catch (caught) {
      if (mountedRef.current) setError(errorMessage(caught));
    } finally {
      if (mountedRef.current) setLoading(false);
    }
  }, [loadSelected, refreshSummaries]);

  const sendText = useCallback(
    async (value: string, clearComposer = false) => {
      const submittedText = value.trim();
      if (!submittedText || sending) return;
      setSending(true);
      setError(null);
      try {
        await subscriptionReadyRef.current;
        const existingSessionId = selectedIdRef.current;
        const acknowledgement = await client.sendMessage({
          ...(existingSessionId ? { sessionId: existingSessionId } : {}),
          submittedText,
          ...(!existingSessionId && options.sessionTitle ? { title: options.sessionTitle } : {}),
          ...(!existingSessionId && workingDirectory.trim()
            ? { workingDirectory: workingDirectory.trim() }
            : {}),
        });
        selectedIdRef.current = acknowledgement.sessionId;
        invocationIdsRef.current.add(acknowledgement.invocationId);
        setSelectedSessionId(acknowledgement.sessionId);
        if (!existingSessionId) options.onSessionCreated?.(acknowledgement.sessionId);
        if (clearComposer) setDraft('');
        await loadSelected(acknowledgement.sessionId, true);
        await refreshSummaries();
      } catch (caught) {
        if (mountedRef.current) setError(errorMessage(caught));
      } finally {
        if (mountedRef.current) setSending(false);
      }
    },
    [client, loadSelected, options, refreshSummaries, sending, workingDirectory],
  );

  const send = useCallback(() => sendText(draft, true), [draft, sendText]);

  const cancel = useCallback(async () => {
    const activeInvocationId = details
      ? projectAgentSessionTranscript(details).activeInvocationId
      : null;
    if (!activeInvocationId || canceling) return;
    setCanceling(true);
    setError(null);
    try {
      await client.cancelInvocation({ invocationId: activeInvocationId });
      if (selectedIdRef.current) await loadSelected(selectedIdRef.current, true);
      await refreshSummaries();
    } catch (caught) {
      if (mountedRef.current) setError(errorMessage(caught));
    } finally {
      if (mountedRef.current) setCanceling(false);
    }
  }, [canceling, client, details, loadSelected, refreshSummaries]);

  const toggleProcessing = useCallback((invocationId: string) => {
    setExpandedProcessing((current) => {
      const next = new Set(current);
      if (next.has(invocationId)) next.delete(invocationId);
      else next.add(invocationId);
      return next;
    });
  }, []);

  const transcript = useMemo(
    () => (details ? projectAgentSessionTranscript(details) : null),
    [details],
  );

  useEffect(() => {
    const sessionId = selectedSessionId;
    if (!sessionId || !transcript?.activeInvocationId) return;

    const interval = window.setInterval(() => {
      void loadSelected(sessionId, true)
        .then((next) => {
          const stillActive = next.invocations.some(({ invocation }) =>
            ['pending', 'running'].includes(invocation.status),
          );
          if (!stillActive) return refreshSummaries().then(() => undefined);
        })
        .catch((caught) => {
          if (mountedRef.current)
            setError(`Session reconciliation failed: ${errorMessage(caught)}`);
        });
    }, 1500);

    return () => window.clearInterval(interval);
  }, [loadSelected, refreshSummaries, selectedSessionId, transcript?.activeInvocationId]);

  return {
    summaries,
    selectedSessionId,
    details,
    transcript,
    draft,
    workingDirectory,
    loading,
    sending,
    canceling,
    error,
    expandedProcessing,
    selectSession,
    startNewSession,
    setDraft,
    setWorkingDirectory,
    send,
    sendText: (value) => sendText(value),
    cancel,
    reload,
    toggleProcessing,
    clearError: () => setError(null),
  };
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}
