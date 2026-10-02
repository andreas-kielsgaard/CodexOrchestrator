import { useSessionQueue } from './useSessionQueue';
import { createComposerDraftStore, type ComposerDraftStore } from './composerDraftStore';
import type { TargetSwitchRequestDto } from '../../application/agentSessions/sessionQueue';
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
  SessionInteractionDto,
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
import {
  clearCachedComposerDraft,
  clearCachedComposerText,
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
  loadEventDetails?(
    id: string,
  ): Promise<import('../../application/agentSessions').AgentRuntimeEventDto>;
  quickCatalogue?: AgentSessionQuickFeatures;
  selectedSessionId: string | null;
  details: AgentSessionDetailsDto | null;
  transcript: ReturnType<typeof projectAgentSessionTranscript> | null;
  draft: string;
  draftSource?: ComposerDraftStore;
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
  steeringQueue: readonly SessionInteractionDto[];
  reorderQueue?(entryIds: readonly string[]): Promise<void>;
  enqueueTargetOperation?(request: TargetSwitchRequestDto, beforeEntryId?: string): Promise<void>;
  compact?(): Promise<void>;
  runTargetOperation?(entryId: string): Promise<void>;
  updateQueuedMessage?(
    input: Omit<UpdateQueuedMessageInputDto, 'sessionId' | 'expectedRevision'>,
  ): Promise<void>;
  updateTargetOperation?(
    input: Omit<UpdateTargetOperationInputDto, 'sessionId' | 'expectedRevision'>,
  ): Promise<void>;
  recalculateTargetProjection?(entryId: string): Promise<void>;
  retryQueueEntry?(entryId: string): Promise<void>;
  cancelQueueEntry?(entryId: string): Promise<void>;
  steer?(attachments?: readonly NewAgentImageAttachmentDto[]): Promise<void>;
  editSteering?(
    interaction: SessionInteractionDto,
    action: 'begin' | 'save' | 'resume' | 'cancel',
    text?: string,
  ): Promise<SessionInteractionDto>;

  reload(): Promise<void>;
  toggleProcessing(invocationId: string): void;
  sendQueueEntryNow?(
    entryId: string,
    message?: Omit<UpdateQueuedMessageInputDto, 'sessionId' | 'entryId' | 'expectedRevision'>,
  ): Promise<boolean>;
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
  const [draftSource] = useState(() => createComposerDraftStore(composerCacheKey));
  const draft = draftSource.getSnapshot();
  const [hydratedComposerCacheKey, setHydratedComposerCacheKey] = useState<string | null>(
    initialComposer.key,
  );
  const draftRef = useRef(draft);
  const setDraft = useCallback(
    (value: string) => {
      draftRef.current = value;
      draftSource.setText(value);
    },
    [draftSource],
  );
  const [currentProfile, setCurrentProfile] = useState<PinnedAgentSessionProfileDto | null>(null);
  const [preparation, setPreparation] = useState<SessionPreparationDto | null>(null);
  const sendingRef = useRef(false);
  const [workingDirectory, setWorkingDirectory] = useState(
    initialComposer.draft?.workingDirectory ?? '',
  );
  const [loading, setLoading] = useState(true);
  const [sending, setSending] = useState(false);
  const [canceling, setCanceling] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [optimisticSteering, setOptimisticSteering] = useState<SessionInteractionDto[]>([]);
  const [expandedProcessing, setExpandedProcessing] = useState<Set<string>>(() => new Set());
  const transcriptProjection = useRef(new AgentSessionTranscriptProjectionCache());
  const selectedIdRef = useRef<string | null>(null);
  const invocationIdsRef = useRef<Set<string>>(new Set());
  const mountedRef = useRef(true);
  const loadGenerationRef = useRef(0);
  const profileRefreshKeyRef = useRef<string | null>(null);
  const subscriptionReadyRef = useRef<Promise<void>>(Promise.resolve());

  const reloadHistoryRef = useRef<(sessionId: string, reload?: boolean) => Promise<unknown>>(
    async () => undefined,
  );
  const reloadQueueHistory = useCallback(
    (sessionId: string, reload?: boolean) => reloadHistoryRef.current(sessionId, reload),
    [],
  );
  const {
    queue,
    setQueue,
    reloadQueue,
    controls: queueControls,
  } = useSessionQueue({
    client,
    queueClient: options.execution?.client,
    selectedIdRef,
    enabled: Boolean(options.preparedExecution),
    reloadHistory: reloadQueueHistory,
    setError,
  });
  const { compact } = queueControls;
  const loadSelected = useCallback(
    async (sessionId: string, reload = false) => {
      if (selectedIdRef.current !== sessionId) return;
      const generation = ++loadGenerationRef.current;
      const historyRequest = historySource
        ? reload
          ? historySource.refresh(sessionId)
          : historySource.ensure(sessionId)
        : reload
          ? client.reloadSession({ sessionId })
          : client.loadSession({ sessionId });
      void historyRequest.then(
        () => {
          if (
            mountedRef.current &&
            selectedIdRef.current === sessionId &&
            generation === loadGenerationRef.current
          )
            setLoading(false);
        },
        () => undefined,
      );
      const [next, prepared, current] = await Promise.all([
        historyRequest,
        options.preparedExecution && options.execution?.client.loadPreparation
          ? options.execution.client.loadPreparation(sessionId)
          : Promise.resolve(null),
        options.preparedExecution && options.execution?.client.loadCurrentProfile
          ? options.execution.client.loadCurrentProfile(sessionId).catch(() => null)
          : Promise.resolve(null),
        reloadQueue(sessionId).catch(() => null),
      ]);
      if (
        mountedRef.current &&
        selectedIdRef.current === sessionId &&
        generation === loadGenerationRef.current
      ) {
        if (!historySource) setFallbackDetails(next);
        setPreparation(prepared);
        setCurrentProfile(current);
        profileRefreshKeyRef.current = JSON.stringify([
          next.session.id,
          next.session.executionTarget,
          next.session.harnessVersion,
        ]);
        setWorkingDirectory(
          readCachedComposerDraft(composerCacheKeyRef.current)?.workingDirectory ??
            next.session.workingDirectory ??
            '',
        );
        invocationIdsRef.current = new Set(next.invocations.map(({ invocation }) => invocation.id));
      }
      return next;
    },
    [client, historySource, options.preparedExecution, options.execution?.client, reloadQueue],
  );

  reloadHistoryRef.current = loadSelected;

  const reconcileUpdate = useCallback(
    async (update: AgentSessionUpdateDto) => {
      if (update.sessionId !== selectedIdRef.current) {
        return;
      }
      if (historySource) {
        if (update.kind === 'queue_updated') {
          const key =
            update.session &&
            JSON.stringify([
              update.session.id,
              update.session.executionTarget,
              update.session.harnessVersion,
            ]);
          if (key && key === profileRefreshKeyRef.current) return;
          profileRefreshKeyRef.current = key ?? null;
        } else if (update.kind !== 'preparation_updated') return;
        try {
          const [prepared, current] = await Promise.all([
            options.preparedExecution && options.execution?.client.loadPreparation
              ? options.execution.client.loadPreparation(update.sessionId)
              : Promise.resolve(null),
            options.preparedExecution && options.execution?.client.loadCurrentProfile
              ? options.execution.client.loadCurrentProfile(update.sessionId).catch(() => null)
              : Promise.resolve(null),
            Promise.resolve(null),
          ]);
          if (mountedRef.current && selectedIdRef.current === update.sessionId) {
            setPreparation(prepared);
            setCurrentProfile(current);
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
      setOptimisticSteering([]);
      if (!historySource) setFallbackDetails(null);
      setCurrentProfile(null);
      setPreparation(null);
      setQueue(null);
      setLoading(!historySource?.getSnapshot(sessionId).details);
      setError(null);
      try {
        await loadSelected(sessionId);
      } catch (caught) {
        if (mountedRef.current) setError(errorMessage(caught));
      } finally {
        if (mountedRef.current && selectedIdRef.current === sessionId) setLoading(false);
      }
    },
    [historySource, loadSelected, setQueue],
  );

  const startNewSession = useCallback(
    (cached = readCachedComposerDraft(composerCacheKeyRef.current), hydrateComposer = true) => {
      loadGenerationRef.current += 1;
      selectedIdRef.current = null;
      setOptimisticSteering([]);
      setFallbackDetails(null);
      setPreparation(null);
      setQueue(null);
      setCurrentProfile(null);
      invocationIdsRef.current = new Set();
      if (hydrateComposer) {
        setWorkingDirectory(cached?.workingDirectory ?? '');
        setDraft(cached?.text ?? '');
      }
      setError(null);
      setLoading(false);
    },
    [setQueue, setDraft],
  );

  const draftKey = options.draftId ?? 'new';
  const contextKey = selectedSessionId ?? draftKey;
  const contextRef = useRef(contextKey);
  contextRef.current = contextKey;
  useEffect(() => {
    setHydratedComposerCacheKey(null);
    draftSource.switchContext(composerCacheKey);
    draftRef.current = draftSource.getSnapshot();
    const cached = readCachedComposerDraft(composerCacheKey);
    const hydrateComposer = composerCacheKeyRef.current !== composerCacheKey;
    composerCacheKeyRef.current = composerCacheKey;
    if (selectedSessionId) {
      if (hydrateComposer) setDraft(cached?.text ?? '');
      if (selectedIdRef.current !== selectedSessionId) void selectSession(selectedSessionId);
    } else startNewSession(cached, hydrateComposer);
    setHydratedComposerCacheKey(composerCacheKey);
  }, [
    selectedSessionId,
    draftKey,
    composerCacheKey,
    selectSession,
    startNewSession,
    draftSource,
    setDraft,
  ]);

  useEffect(() => {
    if (hydratedComposerCacheKey !== composerCacheKey) return;
    draftSource.setDirectory(workingDirectory);
  }, [composerCacheKey, hydratedComposerCacheKey, workingDirectory, draftSource]);

  // Flush on navigation/unmount, not on each keystroke's effect cleanup.
  useEffect(() => () => draftSource.flush(), [draftSource]);

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

  const clearSubmittedText = useCallback((key: string, submittedText: string) => {
    if (readCachedComposerDraft(key)?.text?.trim() === submittedText.trim())
      clearCachedComposerText(key);
  }, []);

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
      let acceptedSessionId: string | null = null;
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
        acceptedSessionId = acknowledgement.sessionId;
        if (options.execution) {
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
          void loadSelected(acknowledgement.sessionId, true).catch((caught) => {
            if (mountedRef.current && selectedIdRef.current === acknowledgement.sessionId)
              setError(`Session refresh failed: ${errorMessage(caught)}`);
          });
        }
      } catch (caught) {
        if (mountedRef.current) setError(errorMessage(caught));
      } finally {
        if (acceptedSessionId && clearComposer) clearSubmittedText(sendCacheKey, value);
        // A controlled owner can switch to the created Session while its acknowledgement is still
        // settling. Clear again only when this exact sent text is still visible; a user-typed next
        // draft always wins.
        if (
          acceptedSessionId &&
          clearComposer &&
          (contextRef.current === sendContext || selectedIdRef.current === acceptedSessionId) &&
          draftRef.current === value
        ) {
          draftRef.current = '';
          setDraft('');
          if (!existingSessionId) clearCachedComposerDraft(sendCacheKey);
        }
        sendingRef.current = false;
        if (mountedRef.current) setSending(false);
      }
    },
    [client, details, loadSelected, options, workingDirectory, clearSubmittedText, setDraft],
  );

  const send = useCallback(
    (attachments: readonly NewAgentImageAttachmentDto[] = []) =>
      sendText(draftRef.current, true, attachments),
    [sendText],
  );

  const steer = useCallback(
    async (attachments: readonly NewAgentImageAttachmentDto[] = []) => {
      const sessionId = selectedIdRef.current;
      const sendCacheKey = composerCacheKeyRef.current;
      const submittedText = draftRef.current.trim();
      const invocationId = details?.invocations.find(
        ({ invocation }) => invocation.status === 'running',
      )?.invocation.id;
      if (
        !sessionId ||
        !invocationId ||
        (!submittedText && !attachments.length) ||
        !client.steerSession ||
        sendingRef.current
      )
        return;
      const inputId = crypto.randomUUID();
      sendingRef.current = true;
      const optimistic: SessionInteractionDto = {
        id: inputId,
        invocationId,
        sequence: Number.MAX_SAFE_INTEGER,
        kind: 'steering',
        state: 'pending',
        content: { text: submittedText },
        result: null,
      };
      setOptimisticSteering((current) => [optimistic, ...current]);
      setSending(true);
      setError(null);
      try {
        const outcome = await client.steerSession({
          sessionId,
          invocationId,
          inputId,
          text: submittedText,
          ...(attachments.length ? { attachments } : {}),
        });
        setOptimisticSteering((current) =>
          outcome.state === 'rejected'
            ? current.filter(({ id }) => id !== inputId)
            : current.map((item) => (item.id === inputId ? outcome : item)),
        );
        if (outcome.state === 'accepted' || outcome.state === 'queued') {
          clearSubmittedText(sendCacheKey, submittedText);
          if (selectedIdRef.current === sessionId && draftRef.current.trim() === submittedText) {
            draftRef.current = '';
            setDraft('');
          }
        } else {
          setError(
            outcome.result ?? `Steering ${outcome.state}. It will not be sent again automatically.`,
          );
        }
        void loadSelected(sessionId, true).catch((caught) => {
          if (mountedRef.current && selectedIdRef.current === sessionId)
            setError(`Session refresh failed: ${errorMessage(caught)}`);
        });
      } catch (caught) {
        setOptimisticSteering((current) => current.filter(({ id }) => id !== inputId));
        setError(errorMessage(caught));
      } finally {
        sendingRef.current = false;
        if (mountedRef.current) setSending(false);
      }
    },
    [client, details, loadSelected, clearSubmittedText, setDraft],
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
  const steeringQueue = useMemo(() => {
    const visibleStates = new Set([
      'queued',
      'pending',
      'expired',
      'editing',
      'rejected',
      'uncertain',
    ]);
    const entries = new Map<string, SessionInteractionDto>();
    for (const interaction of details?.interactions ?? [])
      if (interaction.kind === 'steering' && visibleStates.has(interaction.state))
        entries.set(interaction.id, interaction);
    const durableIds = new Set((details?.interactions ?? []).map((i) => i.id));
    for (const interaction of optimisticSteering)
      if (visibleStates.has(interaction.state) && !durableIds.has(interaction.id))
        entries.set(interaction.id, interaction);
    return [...entries.values()].sort(
      (left, right) => right.sequence - left.sequence || right.id.localeCompare(left.id),
    );
  }, [details?.interactions, optimisticSteering]);

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
  const quickSessionScope =
    options.executionSelection || options.executionQuickFeatures || quickExecutionTarget
      ? null
      : (selectedSessionId ?? options.draftId);
  const quickFeatureKey = JSON.stringify([
    quickSessionScope,
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
  const runningInvocationId = details?.invocations.find(
    ({ invocation }) => invocation.status === 'running',
  )?.invocation.id;
  const runningTurnPreparing = preparation?.invocationId === runningInvocationId && preparing;
  const loadEventDetails = useCallback(
    (id: string) => {
      if (!selectedSessionId || !client.loadEventDetails)
        throw new Error('Action details unavailable');
      return client.loadEventDetails(selectedSessionId, id);
    },
    [client, selectedSessionId],
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
            contextKey: quickFeatureKey,
            catalogue: quickCatalogue,
            load: loadQuickFeatures,
            loadSkillDetails: quickFeaturesClient.loadSkillDetails
              ? (skillId) =>
                  quickFeaturesClient.loadSkillDetails!(
                    {
                      sessionId: selectedSessionId,
                      workingDirectory: quickContext || null,
                      ...(quickFolderTarget ? { folderTarget: quickFolderTarget } : {}),
                      ...(quickExecutionTarget ? { executionTarget: quickExecutionTarget } : {}),
                      ...(options.executionSelection
                        ? {
                            configuration: {
                              provider: options.executionSelection.execution.provider,
                              configurationId:
                                options.executionSelection.execution.configurationRef,
                            },
                          }
                        : {}),
                    },
                    skillId,
                  )
              : undefined,
            refresh: async () => {
              const catalogue = await refreshQuickFeatures();
              setQuickCatalogue(catalogue);
              return catalogue;
            },
            selection: options.execution.selection,
            setSelection: options.execution.setSelection,
            compact:
              selectedSessionId &&
              options.execution.client.enqueueCompaction &&
              quickCatalogue?.configuration?.provider === 'codex'
                ? compact
                : undefined,
          }
        : undefined,
    quickCatalogue,
    respondToRequest,
    steeringAvailable:
      Boolean(client.steerSession) && Boolean(runningInvocationId) && !runningTurnPreparing,
    steer,
    queue,
    queuePresentation,
    ...queueControls,
    steeringQueue,
    editSteering: client.editSteering
      ? async (interaction, action, text) => {
          const sessionId = selectedIdRef.current;
          if (!sessionId) throw new Error('No conversation selected');
          const result = await client.editSteering!({
            sessionId,
            invocationId: interaction.invocationId,
            inputId: interaction.id,
            action,
            ...(text !== undefined ? { text } : {}),
          });
          if (selectedIdRef.current === sessionId) {
            setOptimisticSteering((current) => [
              ...current.filter((i) => i.id !== result.id),
              result,
            ]);
            void loadSelected(sessionId, true);
          }
          return result;
        }
      : undefined,
    selectedSessionId,
    details,
    loadEventDetails: selectedSessionId && client.loadEventDetails ? loadEventDetails : undefined,
    transcript,
    get draft() {
      return draftSource.getSnapshot();
    },
    draftSource,
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
