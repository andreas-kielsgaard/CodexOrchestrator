import type { AgentSessionQuickFeatures } from '../../application/agentSessions/quickFeatures';
import { mergeSelectedQuickFeatures } from './selectedTargetQuickFeatures';
import type {
  SessionExecutionSelectionDto,
  SessionExecutionTargetDto,
} from '../../application/executionTargets/contracts';
import {
  isSessionPreparing,
  type SessionPreparationDto,
} from '../../application/agentSessions/preparation';
import { sessionErrorMessage as errorMessage } from './sessionErrors';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import type {
  AgentSessionClient,
  AgentSessionDetailsDto,
  AgentSessionUpdateDto,
  AgentSessionProfileClient,
  PinnedAgentSessionProfileDto,
} from '../../application/agentSessions';
import { samePreparedConfiguration } from './sessionPreparationState';
import { projectAgentSessionTranscript } from './transcriptProjector';
import type { ComposerQuickFeatures } from './composerQuickActions';

export interface AgentSessionWorkspaceController {
  currentProfile?: PinnedAgentSessionProfileDto | null;
  preparation?: SessionPreparationDto | null;
  preparing?: boolean;
  submissionUnavailableReason?: string;
  retryPreparation?(): Promise<void>;
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
  executionQuickFeatures?: AgentSessionQuickFeatures;
  executionSelection?: SessionExecutionSelectionDto | null;
  preparedExecution?: boolean;
  executionTarget?: SessionExecutionTargetDto | null;
  execution?: {
    readonly target?: SessionExecutionTargetDto | null;
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
  const [currentProfile, setCurrentProfile] = useState<PinnedAgentSessionProfileDto | null>(null);
  const [preparation, setPreparation] = useState<SessionPreparationDto | null>(null);
  const acceptedOptionsRef = useRef<string | null>(null);
  const sendingRef = useRef(false);
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
      const prepared =
        options.preparedExecution && options.execution?.client.loadPreparation
          ? await options.execution.client.loadPreparation(sessionId)
          : null;
      const current =
        options.preparedExecution && options.execution?.client.loadCurrentProfile
          ? await options.execution.client.loadCurrentProfile(sessionId).catch(() => null)
          : null;
      if (
        mountedRef.current &&
        selectedIdRef.current === sessionId &&
        generation === loadGenerationRef.current
      ) {
        setDetails(next);
        setPreparation(prepared);
        setCurrentProfile(current);
        setWorkingDirectory(next.session.workingDirectory ?? '');
        invocationIdsRef.current = new Set(next.invocations.map(({ invocation }) => invocation.id));
      }
      return next;
    },
    [client, options.preparedExecution, options.execution?.client],
  );

  const reconcileUpdate = useCallback(
    async (update: AgentSessionUpdateDto) => {
      if (update.sessionId !== selectedIdRef.current) {
        return;
      }
      if (
        update.kind !== 'target_transition_updated' &&
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
      setCurrentProfile(null);
      setPreparation(null);
      acceptedOptionsRef.current = null;
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
    setPreparation(null);
    setCurrentProfile(null);
    acceptedOptionsRef.current = null;
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
      if (!submittedText || sendingRef.current) return;
      const preparingNow = isSessionPreparing(preparation);
      if (preparingNow) return;
      const sendContext = contextRef.current;
      const existingSessionId = selectedIdRef.current;
      sendingRef.current = true;
      setSending(true);
      setError(null);
      try {
        await subscriptionReadyRef.current;
        const activeInvocationId =
          details && details.session.id === existingSessionId
            ? projectAgentSessionTranscript(details).activeInvocationId
            : null;
        if (existingSessionId && activeInvocationId) {
          if (
            options.preparedExecution &&
            !samePreparedConfiguration(
              options.executionSelection,
              options.execution?.selection,
              preparation,
              acceptedOptionsRef.current,
            )
          )
            throw new Error('These execution choices apply after the current turn finishes.');
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
          !options.preparedExecution &&
          existingSessionId &&
          details?.session.id === existingSessionId &&
          !details.session.workingDirectory &&
          workingDirectory.trim()
        ) {
          if (!client.resolveWorkingDirectory)
            throw new Error('Working context selection is unavailable.');
          await client.resolveWorkingDirectory(existingSessionId, workingDirectory.trim());
        }
        const preparedOptions = JSON.stringify([
          options.executionSelection,
          options.execution?.selection,
        ]);
        const acknowledgement =
          options.preparedExecution && options.execution?.client.sendPreparedMessage
            ? await options.execution.client.sendPreparedMessage({
                sessionId: existingSessionId,
                submissionId: crypto.randomUUID(),
                submittedText,
                title: options.sessionTitle ?? null,
                workingDirectory: workingDirectory.trim() || null,
                executionSelection: options.executionSelection ?? null,
                ...options.execution.selection,
                folderTarget: options.folderTarget ?? null,
              })
            : existingSessionId && options.sendExistingMessage
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
                      workingDirectory:
                        options.execution.target?.path ?? (workingDirectory.trim() || null),
                      title: options.sessionTitle ?? null,
                      ...(options.execution.target
                        ? { executionTarget: options.execution.target }
                        : {}),
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
        acceptedOptionsRef.current = preparedOptions;
        if (!options.preparedExecution) options.execution?.afterAccepted();
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
        sendingRef.current = false;
        if (mountedRef.current) setSending(false);
      }
    },
    [client, details, loadSelected, options, preparation, workingDirectory],
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
      if (isSessionPreparing(preparation) && options.execution?.client.cancelPreparation)
        await options.execution.client.cancelPreparation(activeInvocationId);
      else await client.cancelInvocation({ invocationId: activeInvocationId });
      if (selectedIdRef.current) await loadSelected(selectedIdRef.current, true);
    } catch (caught) {
      if (mountedRef.current) setError(errorMessage(caught));
    } finally {
      if (mountedRef.current) setCanceling(false);
    }
  }, [canceling, client, details, loadSelected, preparation, options.execution?.client]);

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
    () => (details ? projectAgentSessionTranscript(details, preparation) : null),
    [details, preparation],
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
  const quickExecutionTarget = options.execution?.target;
  const quickContext =
    quickExecutionTarget?.path ??
    (details?.session.id === selectedSessionId
      ? details.session.workingDirectory
      : workingDirectory);
  const quickFolderTarget = selectedSessionId ? null : options.folderTarget;
  const loadQuickFeatures = useCallback(async () => {
    const selectedFacts = options.executionQuickFeatures;
    const desired = options.executionSelection;
    if (options.preparedExecution && desired && selectedFacts) {
      if (desired.execution.connection.kind === 'ssh')
        return {
          ...selectedFacts,
          limitations: ['Native skill discovery is unavailable on remote devices.'],
        };
      if (desired.workspace.kind !== 'existing')
        return {
          ...selectedFacts,
          limitations: ['Skills are available after the working folder is prepared.'],
        };
      if (!quickFeaturesClient?.loadQuickFeatures) return selectedFacts;
      try {
        const discovered = await quickFeaturesClient.loadQuickFeatures({
          sessionId: null,
          workingDirectory: desired.workspace.target.path,
          executionTarget: desired.workspace.target,
        });
        return mergeSelectedQuickFeatures(selectedFacts, discovered);
      } catch (cause) {
        return {
          ...selectedFacts,
          limitations: [`Skill discovery unavailable: ${errorMessage(cause)}`],
        };
      }
    }
    if (options.preparedExecution && desired)
      throw new Error('Loading selected target capabilities.');
    if (!quickFeaturesClient?.loadQuickFeatures) throw new Error('Quick features are unavailable.');
    return quickFeaturesClient.loadQuickFeatures({
      sessionId: selectedSessionId,
      workingDirectory: quickContext || null,
      ...(quickFolderTarget ? { folderTarget: quickFolderTarget } : {}),
      ...(quickExecutionTarget ? { executionTarget: quickExecutionTarget } : {}),
    });
  }, [
    quickFeaturesClient,
    selectedSessionId,
    quickContext,
    quickFolderTarget,
    quickExecutionTarget,
    options.preparedExecution,
    options.executionSelection,
    options.executionQuickFeatures,
  ]);

  const preparing = isSessionPreparing(preparation);
  const sameActiveConfiguration =
    !options.preparedExecution ||
    samePreparedConfiguration(
      options.executionSelection,
      options.execution?.selection,
      preparation,
      acceptedOptionsRef.current,
    );
  return {
    preparation,
    preparing,
    currentProfile,
    submissionUnavailableReason: preparing
      ? 'Waiting for setup to finish.'
      : transcript?.activeInvocationId && !sameActiveConfiguration
        ? 'These execution choices apply after the current turn finishes.'
        : undefined,
    retryPreparation: async () => {
      if (!preparation?.canRetry || !options.execution?.client.retryPreparation) return;
      try {
        await options.execution.client.retryPreparation(preparation.invocationId);
        if (selectedIdRef.current) await loadSelected(selectedIdRef.current, true);
      } catch (cause) {
        setError(errorMessage(cause));
      }
    },
    quickFeatures:
      options.execution?.setSelection && quickFeaturesClient?.loadQuickFeatures
        ? {
            contextKey: JSON.stringify([
              selectedSessionId ?? options.draftId,
              quickContext,
              quickFolderTarget,
              quickExecutionTarget,
              options.executionSelection,
              options.executionQuickFeatures,
            ]),
            load: loadQuickFeatures,
            selection: options.execution.selection,
            setSelection: options.execution.setSelection,
          }
        : undefined,
    respondToRequest,
    steeringAvailable: Boolean(client.steerSession) && !preparing && sameActiveConfiguration,
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
