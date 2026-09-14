import { sessionErrorMessage as errorMessage } from './sessionErrors';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import type {
  AgentSessionClient,
  AgentSessionDetailsDto,
  AgentSessionUpdateDto,
  AgentSessionProfileClient,
} from '../../application/agentSessions';
import { projectAgentSessionTranscript } from './transcriptProjector';
import type { ComposerQuickFeatures } from './composerQuickActions';

export interface AgentSessionWorkspaceController {
  quickFeatures?: ComposerQuickFeatures;
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
  setDraft(value: string): void;
  setWorkingDirectory(value: string): void;
  send(): Promise<void>;
  /** Submits caller-supplied text without replacing or clearing the visible composer draft. */
  sendText?(value: string): Promise<void>;
  cancel(): Promise<void>;
  respondToRequest?(invocationId: string, requestId: string, response: unknown): Promise<void>;
  steeringAvailable?: boolean;

  reload(): Promise<void>;
  toggleProcessing(invocationId: string): void;
  clearError(): void;
}

export interface UseAgentSessionOptions {
  execution?: {
    readonly client: AgentSessionProfileClient;
    readonly selection: { readonly model: string | null; readonly reasoningMode: string | null };
    readonly setSelection?: ComposerQuickFeatures['setSelection'];
    afterAccepted(): void;
  };
  startSession?(input: {
    readonly submittedText: string;
    readonly workingDirectory: string | null;
    readonly title: string | null;
  }): Promise<{ readonly sessionId: string; readonly invocationId: string }>;
  selectedSessionId: string | null;
  draftId?: string;
  folderTarget?: import('../../application/agentSessions/organization').SessionFolderTarget | null;
  onSessionCreated?(sessionId: string): void;
  /** Optional replacement boundary for messages sent to an already-created Session. */
  sendExistingMessage?(input: {
    readonly sessionId: string;
    readonly submittedText: string;
  }): Promise<{ readonly sessionId: string; readonly invocationId: string }>;
  /** Optional managed-composition metadata for first-session creation. */
  sessionTitle?: string;
}

export function useAgentSession(
  client: AgentSessionClient,
  options: UseAgentSessionOptions,
): AgentSessionWorkspaceController {
  const selectedSessionId = options.selectedSessionId;
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
      } catch (caught) {
        if (mountedRef.current) setError(`Session reload failed: ${errorMessage(caught)}`);
      }
    },
    [loadSelected],
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
    })().catch((caught) => {
      if (mountedRef.current) setError(errorMessage(caught));
    });
    subscriptionReadyRef.current = ready;

    return () => {
      canceled = true;
      mountedRef.current = false;
      unsubscribe?.();
    };
  }, [client, reconcileUpdate]);

  const selectSession = useCallback(
    async (sessionId: string) => {
      selectedIdRef.current = sessionId;
      setDetails(null);
      setLoading(true);
      setError(null);
      try {
        await loadSelected(sessionId);
      } catch (caught) {
        if (mountedRef.current) setError(errorMessage(caught));
      } finally {
        if (mountedRef.current && selectedIdRef.current === sessionId) setLoading(false);
      }
    },
    [loadSelected],
  );

  const startNewSession = useCallback(() => {
    loadGenerationRef.current += 1;
    selectedIdRef.current = null;
    setDetails(null);
    invocationIdsRef.current = new Set();
    setWorkingDirectory('');
    setDraft('');
    setError(null);
    setLoading(false);
  }, []);

  const draftKey = options.draftId ?? 'new';
  const contextKey = selectedSessionId ?? draftKey;
  const contextRef = useRef(contextKey);
  contextRef.current = contextKey;
  useEffect(() => {
    if (selectedSessionId) {
      if (selectedIdRef.current !== selectedSessionId) void selectSession(selectedSessionId);
    } else startNewSession();
  }, [selectedSessionId, draftKey, selectSession, startNewSession]);

  const reload = useCallback(async () => {
    const sessionId = selectedIdRef.current;
    if (!sessionId) {
      return;
    }
    setLoading(true);
    setError(null);
    try {
      await loadSelected(sessionId, true);
    } catch (caught) {
      if (mountedRef.current) setError(errorMessage(caught));
    } finally {
      if (mountedRef.current && selectedIdRef.current === sessionId) setLoading(false);
    }
  }, [loadSelected]);

  const sendText = useCallback(
    async (value: string, clearComposer = false) => {
      const submittedText = value.trim();
      if (!submittedText || sending) return;
      const sendContext = contextRef.current;
      const existingSessionId = selectedIdRef.current;
      setSending(true);
      setError(null);
      try {
        await subscriptionReadyRef.current;
        const activeInvocationId =
          details && details.session.id === existingSessionId
            ? projectAgentSessionTranscript(details).activeInvocationId
            : null;
        if (existingSessionId && activeInvocationId) {
          if (!client.steerSession)
            throw new Error('Turn steering is unavailable for this connection.');
          const outcome = await client.steerSession({
            sessionId: existingSessionId,
            invocationId: activeInvocationId,
            inputId: crypto.randomUUID(),
            text: submittedText,
          });
          if (outcome.state === 'accepted' || outcome.state === 'uncertain') {
            if (clearComposer && selectedIdRef.current === existingSessionId)
              setDraft((current) => (current === value ? '' : current));
          }
          if (outcome.state !== 'accepted')
            setError(
              outcome.result ??
                `Steering ${outcome.state}. It will not be sent again automatically.`,
            );
          if (selectedIdRef.current === existingSessionId)
            await loadSelected(existingSessionId, true);
          return;
        }
        if (
          existingSessionId &&
          details?.session.id === existingSessionId &&
          !details.session.workingDirectory &&
          workingDirectory.trim()
        ) {
          if (!client.resolveWorkingDirectory)
            throw new Error('Working context selection is unavailable.');
          await client.resolveWorkingDirectory(existingSessionId, workingDirectory.trim());
        }
        const acknowledgement =
          existingSessionId && options.sendExistingMessage
            ? await options.sendExistingMessage({
                sessionId: existingSessionId,
                submittedText,
              })
            : options.execution
              ? existingSessionId
                ? await options.execution.client.sendDirectUserMessage({
                    sessionId: existingSessionId,
                    submittedText,
                    ...options.execution.selection,
                  })
                : await options.execution.client.startDirectUserSession({
                    submittedText,
                    workingDirectory: workingDirectory.trim() || null,
                    title: options.sessionTitle ?? null,
                    ...options.execution.selection,
                    ...(options.folderTarget ? { folderTarget: options.folderTarget } : {}),
                  })
              : !existingSessionId && options.startSession
                ? await options.startSession({
                    submittedText,
                    workingDirectory: workingDirectory.trim() || null,
                    title: options.sessionTitle ?? null,
                  })
                : await client.sendMessage({
                    ...(existingSessionId ? { sessionId: existingSessionId } : {}),
                    submittedText,
                    ...(!existingSessionId && options.sessionTitle
                      ? { title: options.sessionTitle }
                      : {}),
                    ...(!existingSessionId && workingDirectory.trim()
                      ? { workingDirectory: workingDirectory.trim() }
                      : {}),
                  });
        options.execution?.afterAccepted();
        if (!existingSessionId && contextRef.current === sendContext)
          options.onSessionCreated?.(acknowledgement.sessionId);
        if (contextRef.current === sendContext && selectedIdRef.current === existingSessionId) {
          selectedIdRef.current = acknowledgement.sessionId;
          invocationIdsRef.current.add(acknowledgement.invocationId);
          if (clearComposer) setDraft((current) => (current === value ? '' : current));
          await loadSelected(acknowledgement.sessionId, true);
        }
      } catch (caught) {
        if (mountedRef.current) setError(errorMessage(caught));
      } finally {
        if (mountedRef.current) setSending(false);
      }
    },
    [client, details, loadSelected, options, sending, workingDirectory],
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
    } catch (caught) {
      if (mountedRef.current) setError(errorMessage(caught));
    } finally {
      if (mountedRef.current) setCanceling(false);
    }
  }, [canceling, client, details, loadSelected]);

  const respondToRequest = useCallback(
    async (invocationId: string, requestId: string, response: unknown) => {
      const sessionId = selectedIdRef.current;
      if (!sessionId || !client.respondToRuntimeRequest) return;
      try {
        await client.respondToRuntimeRequest({ sessionId, invocationId, requestId, response });
      } catch (caught) {
        if (mountedRef.current && selectedIdRef.current === sessionId)
          setError(errorMessage(caught));
      } finally {
        if (selectedIdRef.current === sessionId) await loadSelected(sessionId, true);
      }
    },
    [client, loadSelected],
  );

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
      void loadSelected(sessionId, true).catch((caught) => {
        if (mountedRef.current) setError(`Session reconciliation failed: ${errorMessage(caught)}`);
      });
    }, 1500);

    return () => window.clearInterval(interval);
  }, [loadSelected, selectedSessionId, transcript?.activeInvocationId]);

  const quickFeaturesClient = options.execution?.client;
  const quickContext =
    details?.session.id === selectedSessionId ? details.session.workingDirectory : workingDirectory;
  const quickFolderTarget = selectedSessionId ? null : options.folderTarget;
  const loadQuickFeatures = useCallback(() => {
    if (!quickFeaturesClient?.loadQuickFeatures)
      return Promise.reject(new Error('Quick features are unavailable.'));
    return quickFeaturesClient.loadQuickFeatures({
      sessionId: selectedSessionId,
      workingDirectory: quickContext || null,
      ...(quickFolderTarget ? { folderTarget: quickFolderTarget } : {}),
    });
  }, [quickFeaturesClient, selectedSessionId, quickContext, quickFolderTarget]);

  return {
    quickFeatures:
      options.execution?.setSelection && quickFeaturesClient?.loadQuickFeatures
        ? {
            contextKey: JSON.stringify([
              selectedSessionId ?? options.draftId,
              quickContext,
              quickFolderTarget,
            ]),
            load: loadQuickFeatures,
            selection: options.execution.selection,
            setSelection: options.execution.setSelection,
          }
        : undefined,
    respondToRequest,
    steeringAvailable: Boolean(client.steerSession),
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
