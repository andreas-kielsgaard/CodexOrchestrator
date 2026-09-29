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
import { useCallback, useEffect, useMemo, useRef, useState, useSyncExternalStore } from 'react';
import type {
  AgentSessionClient,
  AgentSessionDetailsDto,
  AgentSessionUpdateDto,
  AgentSessionProfileClient,
  PinnedAgentSessionProfileDto,
  RuntimeInteractionResponseDto,
  NewAgentImageAttachmentDto,
} from '../../application/agentSessions';
import type {
  SessionQueueViewDto,
  UpdateQueuedMessageInputDto,
  UpdateTargetOperationInputDto,
} from '../../application/agentSessions/sessionQueue';
import {
  presentSessionQueue,
  type SessionQueuePresentation,
} from '../../application/agentSessions/sessionQueuePresentation';
import { samePreparedConfiguration } from './sessionPreparationState';
import {
  clearCachedComposerDraft,
  composerDraftCacheKey,
  readCachedComposerDraft,
  writeCachedComposerDraft,
} from './composerDraftCache';
import {
  AgentSessionTranscriptProjectionCache,
  projectAgentSessionTranscript,
  withoutTranscriptInvocations,
} from './transcriptProjector';
import type { ComposerQuickFeatures } from './composerQuickActions';

const EMPTY_HISTORY_SNAPSHOT = {
  details: null,
  loading: false,
  refreshing: false,
  error: null,
  revision: 0,
  changes: [],
} as const;

export interface AgentSessionWorkspaceController {
  currentProfile?: PinnedAgentSessionProfileDto | null;
  preparation?: SessionPreparationDto | null;
  preparing?: boolean;
  submissionUnavailableReason?: string;
  retryPreparation?(): Promise<void>;
  quickFeatures?: ComposerQuickFeatures;
  quickCatalogue?: AgentSessionQuickFeatures;
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
  send(attachments?: readonly NewAgentImageAttachmentDto[]): Promise<void>;
  /** Submits caller-supplied text without replacing or clearing the visible composer draft. */
  sendText?(value: string): Promise<void>;
  cancel(): Promise<void>;
  respondToRequest?(
    invocationId: string,
    requestId: string,
    response: RuntimeInteractionResponseDto,
  ): Promise<void>;
  steeringAvailable?: boolean;
  queue?: SessionQueueViewDto | null;
  queuePresentation?: SessionQueuePresentation;
  reorderQueue?(entryIds: readonly string[]): Promise<void>;
  enqueueTargetOperation?(
    destination: SessionExecutionSelectionDto,
    beforeEntryId?: string,
  ): Promise<void>;
  updateQueuedMessage?(
    input: Omit<UpdateQueuedMessageInputDto, 'sessionId' | 'expectedRevision'>,
  ): Promise<void>;
  updateTargetOperation?(
    input: Omit<UpdateTargetOperationInputDto, 'sessionId' | 'expectedRevision'>,
  ): Promise<void>;
  startQueueEntry?(entryId: string): Promise<void>;
  retryQueueEntry?(entryId: string): Promise<void>;
  cancelQueueEntry?(entryId: string): Promise<void>;
  steer?(attachments?: readonly NewAgentImageAttachmentDto[]): Promise<void>;

  reload(): Promise<void>;
  toggleProcessing(invocationId: string): void;
  clearError(): void;
}

const quickFeatureCaches = new WeakMap<object, Map<string, AgentSessionQuickFeatures>>();
const quickFeatureRequests = new WeakMap<object, Map<string, Promise<AgentSessionQuickFeatures>>>();

function quickFeatureBucket<T>(store: WeakMap<object, Map<string, T>>, owner: object) {
  const existing = store.get(owner);
  if (existing) return existing;
  const created = new Map<string, T>();
  store.set(owner, created);
  return created;
}

function cachedQuickFeatures(
  key: string,
  owner: object,
  discover: () => Promise<AgentSessionQuickFeatures>,
  refresh = false,
): Promise<AgentSessionQuickFeatures> {
  const cache = quickFeatureBucket(quickFeatureCaches, owner);
  const requests = quickFeatureBucket(quickFeatureRequests, owner);
  if (!refresh) {
    const cached = cache.get(key);
    if (cached) return Promise.resolve(cached);
    const pending = requests.get(key);
    if (pending) return pending;
  }
  const request = discover().then((catalogue) => {
    cache.set(key, catalogue);
    return catalogue;
  });
  requests.set(key, request);
  void request
    .finally(() => {
      if (requests.get(key) === request) requests.delete(key);
    })
    .catch(() => undefined);
  return request;
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
    afterAccepted(
      sessionId: string,
      selection: { readonly model: string | null; readonly reasoningMode: string | null },
    ): void;
  };
  startSession?(input: {
    readonly submittedText: string;
    readonly attachments?: readonly NewAgentImageAttachmentDto[];
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
    readonly attachments?: readonly NewAgentImageAttachmentDto[];
  }): Promise<{ readonly sessionId: string; readonly invocationId: string }>;
  /** Optional managed-composition metadata for first-session creation. */
  sessionTitle?: string;
}

export function useAgentSession(
  client: AgentSessionClient,
  options: UseAgentSessionOptions,
): AgentSessionWorkspaceController {
  const selectedSessionId = options.selectedSessionId;
  const historySource = client.historySource;
  const subscribeToHistory = useCallback(
    (listener: () => void) =>
      historySource && selectedSessionId
        ? historySource.subscribe(selectedSessionId, listener)
        : () => undefined,
    [historySource, selectedSessionId],
  );
  const readHistory = useCallback(
    () =>
      historySource && selectedSessionId
        ? historySource.getSnapshot(selectedSessionId)
        : EMPTY_HISTORY_SNAPSHOT,
    [historySource, selectedSessionId],
  );
  const historySnapshot = useSyncExternalStore(subscribeToHistory, readHistory, readHistory);
  const composerCacheKey = composerDraftCacheKey(selectedSessionId, options.folderTarget);
  const [initialComposer] = useState(() => ({
    key: composerCacheKey,
    draft: readCachedComposerDraft(composerCacheKey),
  }));
  const composerCacheKeyRef = useRef(composerCacheKey);
  const [fallbackDetails, setFallbackDetails] = useState<AgentSessionDetailsDto | null>(null);
  const details = historySource ? historySnapshot.details : fallbackDetails;
  const [draft, setDraft] = useState(initialComposer.draft?.text ?? '');
  const [hydratedComposerCacheKey, setHydratedComposerCacheKey] = useState<string | null>(
    initialComposer.key,
  );
  const draftRef = useRef(draft);
  draftRef.current = draft;
  const [currentProfile, setCurrentProfile] = useState<PinnedAgentSessionProfileDto | null>(null);
  const [preparation, setPreparation] = useState<SessionPreparationDto | null>(null);
  const [queue, setQueue] = useState<SessionQueueViewDto | null>(null);
  const acceptedOptionsRef = useRef<string | null>(null);
  const sendingRef = useRef(false);
  const [workingDirectory, setWorkingDirectory] = useState(
    initialComposer.draft?.workingDirectory ?? '',
  );
  const [loading, setLoading] = useState(true);
  const [sending, setSending] = useState(false);
  const [canceling, setCanceling] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [expandedProcessing, setExpandedProcessing] = useState<Set<string>>(() => new Set());
  const transcriptProjection = useRef(new AgentSessionTranscriptProjectionCache());
  const selectedIdRef = useRef<string | null>(null);
  const invocationIdsRef = useRef<Set<string>>(new Set());
  const mountedRef = useRef(true);
  const loadGenerationRef = useRef(0);
  const subscriptionReadyRef = useRef<Promise<void>>(Promise.resolve());

  const loadSelected = useCallback(
    async (sessionId: string, reload = false) => {
      const generation = ++loadGenerationRef.current;
      const next = historySource
        ? reload
          ? await historySource.refresh(sessionId)
          : await historySource.ensure(sessionId)
        : reload
          ? await client.reloadSession({ sessionId })
          : await client.loadSession({ sessionId });
      const [prepared, current, queued] = await Promise.all([
        options.preparedExecution && options.execution?.client.loadPreparation
          ? options.execution.client.loadPreparation(sessionId)
          : Promise.resolve(null),
        options.preparedExecution && options.execution?.client.loadCurrentProfile
          ? options.execution.client.loadCurrentProfile(sessionId).catch(() => null)
          : Promise.resolve(null),
        options.preparedExecution && options.execution?.client.loadQueue
          ? options.execution.client.loadQueue(sessionId).catch(() => null)
          : Promise.resolve(null),
      ]);
      if (
        mountedRef.current &&
        selectedIdRef.current === sessionId &&
        generation === loadGenerationRef.current
      ) {
        if (!historySource) setFallbackDetails(next);
        setPreparation(prepared);
        setCurrentProfile(current);
        setQueue(queued);
        setWorkingDirectory(
          readCachedComposerDraft(composerCacheKeyRef.current)?.workingDirectory ??
            next.session.workingDirectory ??
            '',
        );
        invocationIdsRef.current = new Set(next.invocations.map(({ invocation }) => invocation.id));
      }
      return next;
    },
    [client, historySource, options.preparedExecution, options.execution?.client],
  );

  const reconcileUpdate = useCallback(
    async (update: AgentSessionUpdateDto) => {
      if (update.sessionId !== selectedIdRef.current) {
        return;
      }
      if (historySource) {
        if (update.kind !== 'preparation_updated' && update.kind !== 'queue_updated') return;
        try {
          const [prepared, current, queued] = await Promise.all([
            options.preparedExecution && options.execution?.client.loadPreparation
              ? options.execution.client.loadPreparation(update.sessionId)
              : Promise.resolve(null),
            options.preparedExecution && options.execution?.client.loadCurrentProfile
              ? options.execution.client.loadCurrentProfile(update.sessionId).catch(() => null)
              : Promise.resolve(null),
            options.preparedExecution && options.execution?.client.loadQueue
              ? options.execution.client.loadQueue(update.sessionId).catch(() => null)
              : Promise.resolve(null),
          ]);
          if (mountedRef.current && selectedIdRef.current === update.sessionId) {
            setPreparation(prepared);
            setCurrentProfile(current);
            setQueue(queued);
          }
        } catch (caught) {
          if (mountedRef.current) setError(`Session setup refresh failed: ${errorMessage(caught)}`);
        }
        return;
      }
      if (update.kind !== 'queue_updated' && !invocationIdsRef.current.has(update.invocationId)) {
        return;
      }
      try {
        await loadSelected(update.sessionId, true);
      } catch (caught) {
        if (mountedRef.current) setError(`Session reload failed: ${errorMessage(caught)}`);
      }
    },
    [historySource, loadSelected, options.execution?.client, options.preparedExecution],
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
      if (!historySource) setFallbackDetails(null);
      setCurrentProfile(null);
      setPreparation(null);
      setQueue(null);
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
    [historySource, loadSelected],
  );

  const startNewSession = useCallback(
    (cached = readCachedComposerDraft(composerCacheKeyRef.current), hydrateComposer = true) => {
      loadGenerationRef.current += 1;
      selectedIdRef.current = null;
      setFallbackDetails(null);
      setPreparation(null);
      setQueue(null);
      setCurrentProfile(null);
      acceptedOptionsRef.current = null;
      invocationIdsRef.current = new Set();
      if (hydrateComposer) {
        setWorkingDirectory(cached?.workingDirectory ?? '');
        setDraft(cached?.text ?? '');
      }
      setError(null);
      setLoading(false);
    },
    [],
  );

  const draftKey = options.draftId ?? 'new';
  const contextKey = selectedSessionId ?? draftKey;
  const contextRef = useRef(contextKey);
  contextRef.current = contextKey;
  useEffect(() => {
    setHydratedComposerCacheKey(null);
    const cached = readCachedComposerDraft(composerCacheKey);
    const hydrateComposer = composerCacheKeyRef.current !== composerCacheKey;
    composerCacheKeyRef.current = composerCacheKey;
    if (selectedSessionId) {
      if (hydrateComposer) setDraft(cached?.text ?? '');
      if (selectedIdRef.current !== selectedSessionId) void selectSession(selectedSessionId);
    } else startNewSession(cached, hydrateComposer);
    setHydratedComposerCacheKey(composerCacheKey);
  }, [selectedSessionId, draftKey, composerCacheKey, selectSession, startNewSession]);

  useEffect(() => {
    if (hydratedComposerCacheKey !== composerCacheKey) return;
    const timer = window.setTimeout(() => {
      writeCachedComposerDraft(composerCacheKey, { text: draft, workingDirectory });
    }, 180);
    return () => {
      window.clearTimeout(timer);
      writeCachedComposerDraft(composerCacheKey, { text: draft, workingDirectory });
    };
  }, [composerCacheKey, draft, hydratedComposerCacheKey, workingDirectory]);

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
    async (
      value: string,
      clearComposer = false,
      attachments: readonly NewAgentImageAttachmentDto[] = [],
    ) => {
      const submittedText = value.trim();
      if ((!submittedText && !attachments.length) || sendingRef.current) return;
      const attachmentInput = attachments.length ? { attachments } : {};
      const sendContext = contextRef.current;
      const sendCacheKey = composerCacheKeyRef.current;
      const existingSessionId = selectedIdRef.current;
      let acceptedDelivery = false;
      sendingRef.current = true;
      setSending(true);
      setError(null);
      try {
        await subscriptionReadyRef.current;
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
                ...attachmentInput,
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
                  ...attachmentInput,
                })
              : options.execution
                ? existingSessionId
                  ? await options.execution.client.sendDirectUserMessage({
                      sessionId: existingSessionId,
                      submittedText,
                      ...attachmentInput,
                      ...options.execution.selection,
                    })
                  : await options.execution.client.startDirectUserSession({
                      submittedText,
                      ...attachmentInput,
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
                      ...attachmentInput,
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
        acceptedDelivery = true;
        if (!options.preparedExecution && options.execution) {
          const effective =
            'invocation' in acknowledgement
              ? (
                  acknowledgement.invocation as {
                    effectiveOptions: {
                      model: { id: string } | null;
                      reasoningMode: string | null;
                    };
                  }
                ).effectiveOptions
              : { model: null, reasoningMode: null };
          const resolved =
            'invocationResolution' in acknowledgement
              ? (
                  acknowledgement.invocationResolution as {
                    selections: { model: string | null; reasoningMode: string | null };
                  }
                ).selections
              : null;
          options.execution.afterAccepted(acknowledgement.sessionId, {
            model: resolved?.model ?? effective.model?.id ?? options.execution.selection.model,
            reasoningMode:
              resolved?.reasoningMode ??
              effective.reasoningMode ??
              options.execution.selection.reasoningMode,
          });
        }
        if (
          !existingSessionId &&
          contextRef.current === sendContext &&
          draftRef.current !== value
        ) {
          writeCachedComposerDraft(
            composerDraftCacheKey(acknowledgement.sessionId, options.folderTarget),
            { text: draftRef.current, workingDirectory },
          );
        }
        if (!existingSessionId && contextRef.current === sendContext)
          options.onSessionCreated?.(acknowledgement.sessionId);
        if (contextRef.current === sendContext && selectedIdRef.current === existingSessionId) {
          selectedIdRef.current = acknowledgement.sessionId;
          invocationIdsRef.current.add(acknowledgement.invocationId);
          await loadSelected(acknowledgement.sessionId, true);
        }
      } catch (caught) {
        if (mountedRef.current) setError(errorMessage(caught));
      } finally {
        // A controlled owner can switch to the created Session while its acknowledgement is still
        // settling. Clear again only when this exact sent text is still visible; a user-typed next
        // draft always wins.
        if (acceptedDelivery && clearComposer && draftRef.current === value) {
          draftRef.current = '';
          setDraft('');
          clearCachedComposerDraft(sendCacheKey);
        }
        sendingRef.current = false;
        if (mountedRef.current) setSending(false);
      }
    },
    [client, details, loadSelected, options, workingDirectory],
  );

  const send = useCallback(
    (attachments: readonly NewAgentImageAttachmentDto[] = []) =>
      sendText(draft, true, attachments),
    [draft, sendText],
  );

  const steer = useCallback(async (attachments: readonly NewAgentImageAttachmentDto[] = []) => {
    const sessionId = selectedIdRef.current;
    const submittedText = draftRef.current.trim();
    const invocationId = details ? projectAgentSessionTranscript(details).activeInvocationId : null;
    if (
      !sessionId ||
      !invocationId ||
      (!submittedText && !attachments.length) ||
      !client.steerSession
    )
      return;
    setSending(true);
    setError(null);
    try {
      const outcome = await client.steerSession({
        sessionId,
        invocationId,
        inputId: crypto.randomUUID(),
        text: submittedText,
        ...(attachments.length ? { attachments } : {}),
      });
      if (outcome.state === 'accepted') {
        if (draftRef.current.trim() === submittedText) {
          draftRef.current = '';
          setDraft('');
          clearCachedComposerDraft(composerCacheKeyRef.current);
        }
      } else {
        setError(
          outcome.result ?? `Steering ${outcome.state}. It will not be sent again automatically.`,
        );
      }
      await loadSelected(sessionId, true);
    } catch (caught) {
      setError(errorMessage(caught));
    } finally {
      if (mountedRef.current) setSending(false);
    }
  }, [client, details, loadSelected]);

  const reorderQueue = useCallback(
    async (entryIds: readonly string[]) => {
      const sessionId = selectedIdRef.current;
      const queueClient = options.execution?.client;
      if (!sessionId || !queue || !queueClient?.reorderQueue) return;
      try {
        setQueue(
          await queueClient.reorderQueue({
            sessionId,
            expectedRevision: queue.revision,
            entryIds,
          }),
        );
      } catch (caught) {
        setError(errorMessage(caught));
        await loadSelected(sessionId, true);
      }
    },
    [loadSelected, options.execution?.client, queue],
  );

  const enqueueTargetOperation = useCallback(
    async (destination: SessionExecutionSelectionDto, beforeEntryId?: string) => {
      const sessionId = selectedIdRef.current;
      const queueClient = options.execution?.client;
      if (!sessionId || !queueClient?.enqueueTargetOperation) return;
      try {
        const appended = await queueClient.enqueueTargetOperation({ sessionId, destination });
        if (!beforeEntryId || !queueClient.reorderQueue) {
          setQueue(appended);
          return;
        }
        const unfinished = appended.entries.filter(
          ({ entry }) => entry.state !== 'completed' && entry.state !== 'canceled',
        );
        const previousIds = new Set(queue?.entries.map(({ entry }) => entry.id));
        const inserted = unfinished.find(
          ({ entry, targetOperation }) => targetOperation && !previousIds.has(entry.id),
        );
        const ids = unfinished.map(({ entry }) => entry.id);
        if (!inserted || !ids.includes(beforeEntryId)) {
          setQueue(appended);
          return;
        }
        const next = ids.filter((id) => id !== inserted.entry.id);
        next.splice(next.indexOf(beforeEntryId), 0, inserted.entry.id);
        setQueue(
          await queueClient.reorderQueue({
            sessionId,
            expectedRevision: appended.revision,
            entryIds: next,
          }),
        );
      } catch (caught) {
        setError(errorMessage(caught));
      }
    },
    [options.execution?.client, queue],
  );

  const updateQueuedMessage = useCallback(
    async (input: Omit<UpdateQueuedMessageInputDto, 'sessionId' | 'expectedRevision'>) => {
      const sessionId = selectedIdRef.current;
      const queueClient = options.execution?.client;
      if (!sessionId || !queue || !queueClient?.updateQueuedMessage) return;
      try {
        setQueue(
          await queueClient.updateQueuedMessage({
            ...input,
            sessionId,
            expectedRevision: queue.revision,
          }),
        );
        await loadSelected(sessionId, true);
      } catch (caught) {
        setError(errorMessage(caught));
      }
    },
    [loadSelected, options.execution?.client, queue],
  );

  const updateTargetOperation = useCallback(
    async (input: Omit<UpdateTargetOperationInputDto, 'sessionId' | 'expectedRevision'>) => {
      const sessionId = selectedIdRef.current;
      const queueClient = options.execution?.client;
      if (!sessionId || !queue || !queueClient?.updateTargetOperation) return;
      try {
        setQueue(
          await queueClient.updateTargetOperation({
            ...input,
            sessionId,
            expectedRevision: queue.revision,
          }),
        );
      } catch (caught) {
        setError(errorMessage(caught));
      }
    },
    [options.execution?.client, queue],
  );

  const actOnQueueEntry = useCallback(
    async (kind: 'start' | 'retry' | 'cancel', entryId: string) => {
      const sessionId = selectedIdRef.current;
      const queueClient = options.execution?.client;
      if (!sessionId || !queueClient) return;
      try {
        if (kind === 'start') await queueClient.startQueueEntry?.(sessionId, entryId);
        else if (kind === 'retry') await queueClient.retryQueueEntry?.(sessionId, entryId);
        else await queueClient.cancelQueueEntry?.(sessionId, entryId);
        await loadSelected(sessionId, true);
      } catch (caught) {
        setError(errorMessage(caught));
      }
    },
    [loadSelected, options.execution?.client],
  );

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
    async (invocationId: string, requestId: string, response: RuntimeInteractionResponseDto) => {
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

  const queuePresentation = useMemo(() => presentSessionQueue(queue), [queue]);
  const transcript = useMemo(() => {
    if (!details) return null;
    return withoutTranscriptInvocations(
      transcriptProjection.current.project(details, preparation),
      queuePresentation.queuedInvocationIds,
    );
  }, [details, preparation, queuePresentation]);

  useEffect(() => {
    if (details) {
      invocationIdsRef.current = new Set(
        details.invocations.map(({ invocation }) => invocation.id),
      );
    }
  }, [details]);

  useEffect(() => {
    const sessionId = selectedSessionId;
    if (historySource || !sessionId || !transcript?.activeInvocationId) return;

    const interval = window.setInterval(() => {
      void loadSelected(sessionId, true).catch((caught) => {
        if (mountedRef.current) setError(`Session reconciliation failed: ${errorMessage(caught)}`);
      });
    }, 1500);

    return () => window.clearInterval(interval);
  }, [historySource, loadSelected, selectedSessionId, transcript?.activeInvocationId]);

  const quickFeaturesClient = options.execution?.client;
  const quickFeaturesClientRef = useRef(quickFeaturesClient);
  quickFeaturesClientRef.current = quickFeaturesClient;
  const quickFeatureOwner = quickFeaturesClient?.loadQuickFeatures ?? quickFeaturesClient;
  const quickExecutionTarget = options.execution?.target;
  const quickContext =
    quickExecutionTarget?.path ??
    (details?.session.id === selectedSessionId
      ? details.session.workingDirectory
      : workingDirectory);
  const quickFolderTarget = selectedSessionId ? null : options.folderTarget;
  const quickFeatureKey = JSON.stringify([
    selectedSessionId ?? options.draftId,
    quickContext,
    quickFolderTarget,
    quickExecutionTarget,
    options.executionSelection,
    options.executionQuickFeatures,
  ]);
  const quickFeatureInput = useRef({
    selectedFacts: options.executionQuickFeatures,
    desired: options.executionSelection,
    prepared: options.preparedExecution,
    sessionId: selectedSessionId,
    context: quickContext,
    folderTarget: quickFolderTarget,
    executionTarget: quickExecutionTarget,
  });
  quickFeatureInput.current = {
    selectedFacts: options.executionQuickFeatures,
    desired: options.executionSelection,
    prepared: options.preparedExecution,
    sessionId: selectedSessionId,
    context: quickContext,
    folderTarget: quickFolderTarget,
    executionTarget: quickExecutionTarget,
  };
  const discoverQuickFeatures = useCallback(async () => {
    const client = quickFeaturesClientRef.current;
    const input = quickFeatureInput.current;
    const selectedFacts = input.selectedFacts;
    const desired = input.desired;
    if (input.prepared && desired?.execution.connection.kind === 'ssh') {
      if (selectedFacts)
        return {
          ...selectedFacts,
          limitations: ['Native skill discovery is unavailable on remote devices.'],
        };
      throw new Error('Native skill discovery is unavailable on remote devices.');
    }
    if (input.prepared && desired && selectedFacts) {
      if (!client?.loadQuickFeatures) return selectedFacts;
      try {
        const discovered = await client.loadQuickFeatures({
          sessionId: null,
          workingDirectory:
            desired.workspace.kind === 'existing' ? desired.workspace.target.path : null,
          configuration: {
            provider: desired.execution.provider,
            configurationId: desired.execution.configurationRef,
          },
          ...(desired.workspace.kind === 'existing'
            ? { executionTarget: desired.workspace.target }
            : {}),
        });
        return mergeSelectedQuickFeatures(selectedFacts, discovered);
      } catch (cause) {
        return {
          ...selectedFacts,
          limitations: [`Skill discovery unavailable: ${errorMessage(cause)}`],
        };
      }
    }
    if (!client?.loadQuickFeatures) throw new Error('Quick features are unavailable.');
    return client.loadQuickFeatures({
      sessionId: input.sessionId,
      workingDirectory: input.context || null,
      ...(input.folderTarget ? { folderTarget: input.folderTarget } : {}),
      ...(input.executionTarget ? { executionTarget: input.executionTarget } : {}),
    });
  }, []);
  const loadQuickFeatures = useCallback(
    () =>
      quickFeatureOwner
        ? cachedQuickFeatures(quickFeatureKey, quickFeatureOwner, discoverQuickFeatures)
        : discoverQuickFeatures(),
    [discoverQuickFeatures, quickFeatureKey, quickFeatureOwner],
  );
  const refreshQuickFeatures = useCallback(
    () =>
      quickFeatureOwner
        ? cachedQuickFeatures(quickFeatureKey, quickFeatureOwner, discoverQuickFeatures, true)
        : discoverQuickFeatures(),
    [discoverQuickFeatures, quickFeatureKey, quickFeatureOwner],
  );
  const [quickCatalogue, setQuickCatalogue] = useState<AgentSessionQuickFeatures | undefined>(
    () =>
      (quickFeatureOwner
        ? quickFeatureBucket(quickFeatureCaches, quickFeatureOwner).get(quickFeatureKey)
        : undefined) ?? options.executionQuickFeatures,
  );
  useEffect(() => {
    let current = true;
    setQuickCatalogue(
      (quickFeatureOwner
        ? quickFeatureBucket(quickFeatureCaches, quickFeatureOwner).get(quickFeatureKey)
        : undefined) ?? quickFeatureInput.current.selectedFacts,
    );
    if (
      quickFeatureInput.current.prepared &&
      quickFeatureInput.current.desired &&
      !quickFeatureInput.current.selectedFacts
    )
      return;
    if (
      !quickFeaturesClientRef.current?.loadQuickFeatures &&
      !quickFeatureInput.current.selectedFacts
    )
      return;
    void loadQuickFeatures().then(
      (catalogue) => {
        if (current) setQuickCatalogue(catalogue);
      },
      () => undefined,
    );
    return () => {
      current = false;
    };
  }, [loadQuickFeatures, quickFeatureKey, quickFeatureOwner]);

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
    submissionUnavailableReason: undefined,
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
            catalogue: quickCatalogue,
            load: loadQuickFeatures,
            refresh: async () => {
              const catalogue = await refreshQuickFeatures();
              setQuickCatalogue(catalogue);
              return catalogue;
            },
            selection: options.execution.selection,
            setSelection: options.execution.setSelection,
          }
        : undefined,
    quickCatalogue,
    respondToRequest,
    steeringAvailable: Boolean(client.steerSession) && !preparing && sameActiveConfiguration,
    steer,
    queue,
    queuePresentation,
    enqueueTargetOperation,
    reorderQueue,
    updateQueuedMessage,
    updateTargetOperation,
    startQueueEntry: (entryId) => actOnQueueEntry('start', entryId),
    retryQueueEntry: (entryId) => actOnQueueEntry('retry', entryId),
    cancelQueueEntry: (entryId) => actOnQueueEntry('cancel', entryId),
    selectedSessionId,
    details,
    transcript,
    draft,
    workingDirectory,
    loading: loading || historySnapshot.loading,
    sending,
    canceling,
    error: error ?? historySnapshot.error,
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
