import { projectedQueueSource } from '../../application/agentSessions/sessionQueue';
import type { AgentSessionImportClient } from '../../application/agentSessions/importContracts';
import { capabilityProfileDeviceModels, withCapabilityProfileDeviceModels } from './routeModels';
import { ImportCodexSessionDialog } from './ImportCodexSessionDialog';
import type { SessionWorkflowTarget } from '../../application/agentSessions/workflowNavigation';
import type { RepositoryBranchSource } from '../../application/branches';
import type { ExecutionConfigurationClient } from '../../application/executionConfiguration';
import {
  localExecutionBinding,
  executionRouteKey,
  executionRouteRef,
  type ExecutionTargetClient,
  type SessionExecutionTargetDto,
} from '../../application/executionTargets/contracts';
import type { ProfileModelCatalogueDto } from '../../application/executionConfiguration';
import { selectedTargetQuickFeatures } from './selectedTargetQuickFeatures';
import { useSessionTarget } from './useSessionTarget';
import { SessionComposerToolbar } from './SessionComposerToolbar';
import { SessionPreparationPanel } from './SessionPreparationPanel';
import { preparationPreview } from './preparationPreview';
import { SessionQueuePanel } from './SessionQueuePanel';
import { SessionDeviceActivityList } from './SessionDeviceActivityList';
import { SessionTargetDialog } from './SessionTargetDialog';
import { DeviceContinuationDialog } from './DeviceContinuationDialog';
import { legacyHarnessRoleLabel } from '../../application/identities/legacyAgentIdentityAdapter';
import type {
  SessionNavigationCommandRequest,
  SessionNavigationState,
} from '../../application/agentSessions/agentAccess';
import { useSessionNavigation } from './useSessionNavigation';
import { useSessionNavigationCommands } from './useSessionNavigationCommands';
import { AlertCircle, X } from 'lucide-react';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import type {
  AgentIdentity,
  AgentSessionClient,
  AgentSessionProfileClient,
} from '../../application/agentSessions';
import type { SessionQueueEntryViewDto } from '../../application/agentSessions/sessionQueue';
import type { ConversationHarnessManagementSource } from '../../application/conversationHarnesses';
import type { SessionEventQueryClient } from '../../application/sessionEvents';
import type { ProductDecisionEvidenceDestination } from '../../application/productDecisions';
import {
  buildSessionNavigation,
  selectionKey,
  sessionIdOf,
  type SessionNavigationSelection,
} from '../../application/agentSessions/navigation';
import type {
  SessionNavigationClient,
  SessionFolderTarget,
} from '../../application/agentSessions/organization';
import type { TranscriptAnchorRange } from './transcriptProjector';
import { AgentSessionHeaderActionsProvider, AgentSessionWorkspace } from './AgentSessionWorkspace';
import { AgentSessionExecutionSettings } from './AgentSessionExecutionSettings';
import { effectiveSessionOptions } from './effectiveSessionOptions';
import { HarnessAwareAgentSessionPane } from '../conversationHarnesses/HarnessAwareAgentSessionPane';
import { SessionSelector } from './SessionSelector';
import { useAgentSessionCollection } from './useAgentSessionCollection';
import { useProfiledAgentSession } from './useProfiledAgentSession';
import { ResizableSplitSurface } from '../orchestrations/components/ResizableSplitSurface';
import './agentSession.css';
import { ModalDialog } from '../../components/ModalDialog';
import { IdentityPickerDialog } from '../identities';
import { browserAgentSessionClipboard, formatAgentSessionContext } from './sessionClipboard';
import { projectAgentSessionTranscript } from './transcriptProjector';

type RouteModelCatalogueCacheEntry =
  | { readonly state: 'loading'; readonly promise: Promise<ProfileModelCatalogueDto> }
  | { readonly state: 'ready'; readonly value: ProfileModelCatalogueDto };

const routeModelCatalogueCaches = new WeakMap<
  ExecutionConfigurationClient,
  Map<string, RouteModelCatalogueCacheEntry>
>();

function cachedRouteModelCatalogue(
  client: ExecutionConfigurationClient,
  reference: Parameters<NonNullable<ExecutionConfigurationClient['loadProfileModelCatalogue']>>[0],
): Promise<ProfileModelCatalogueDto> {
  let cache = routeModelCatalogueCaches.get(client);
  if (!cache) {
    cache = new Map();
    routeModelCatalogueCaches.set(client, cache);
  }
  const key = executionRouteKey(reference);
  const existing = cache.get(key);
  if (existing?.state === 'ready') return Promise.resolve(existing.value);
  if (existing?.state === 'loading') return existing.promise;
  const promise = client.loadProfileModelCatalogue!(reference).then(
    (value) => {
      cache!.set(key, { state: 'ready', value });
      return value;
    },
    (cause) => {
      cache!.delete(key);
      throw cause;
    },
  );
  cache.set(key, { state: 'loading', promise });
  return promise;
}

function readyRouteModelCatalogues(
  client: ExecutionConfigurationClient | undefined,
  references: readonly Parameters<
    NonNullable<ExecutionConfigurationClient['loadProfileModelCatalogue']>
  >[0][],
): Readonly<Record<string, ProfileModelCatalogueDto>> {
  if (!client) return {};
  const cache = routeModelCatalogueCaches.get(client);
  return Object.fromEntries(
    references.flatMap((reference) => {
      const key = executionRouteKey(reference);
      const entry = cache?.get(key);
      return entry?.state === 'ready' ? [[key, entry.value] as const] : [];
    }),
  );
}
export interface AgentSessionScreenProps {
  readonly importClient?: AgentSessionImportClient;
  readonly executionTargetClient?: ExecutionTargetClient;
  readonly executionConfigurationClient?: ExecutionConfigurationClient;
  readonly branchSource?: RepositoryBranchSource;
  readonly client: AgentSessionClient;
  readonly agentRequest?: SessionNavigationCommandRequest | null;
  readonly onAgentComplete?: (
    id: string,
    state: SessionNavigationState | null,
    error: string | null,
  ) => Promise<void>;
  readonly navigationClient?: SessionNavigationClient;
  readonly selection?: SessionNavigationSelection;
  readonly onSelectionChange?: (selection: SessionNavigationSelection) => void;
  readonly harnessManagementSource?: ConversationHarnessManagementSource;
  readonly profileClient?: AgentSessionProfileClient;
  readonly sessionEventQueryClient?: SessionEventQueryClient;
  readonly agentIdentityForSession?: (sessionId: string) => AgentIdentity | undefined;
  readonly focusInvocationId?: string;
  readonly focusEvidence?: ProductDecisionEvidenceDestination;
  readonly onOpenWorkflow?: (target: SessionWorkflowTarget) => void;
  readonly onConfigureCapabilities?: () => void;
}
export function StandaloneAgentSessionScreen({
  importClient,
  executionTargetClient,
  executionConfigurationClient,
  branchSource,
  client,
  agentRequest,
  onAgentComplete,
  navigationClient,
  selection: controlled,
  onSelectionChange,
  harnessManagementSource,
  profileClient,
  sessionEventQueryClient,
  agentIdentityForSession,
  focusInvocationId,
  focusEvidence,
  onConfigureCapabilities,
  onOpenWorkflow,
}: AgentSessionScreenProps) {
  const [importOpen, setImportOpen] = useState(false);
  const [sessionDialog, setSessionDialog] = useState<{
    id: string;
    kind: 'identity' | 'configuration';
  } | null>(null);
  const [identityError, setIdentityError] = useState<string | null>(null);
  const copyEntireSession = useCallback(
    async (sessionId: string) => {
      const details = await client.loadSession({ sessionId });
      await browserAgentSessionClipboard.writeText(
        formatAgentSessionContext(details, projectAgentSessionTranscript(details)),
      );
    },
    [client],
  );
  const [targetPicker, setTargetPicker] = useState<{
    deviceId?: string;
    target?: SessionExecutionTargetDto;
    queueEntry?: SessionQueueEntryViewDto;
  } | null>(null);
  const [deviceContinuationOpen, setDeviceContinuationOpen] = useState(false);
  const [previewOpen, setPreviewOpen] = useState(false);
  const [localSelection, setLocalSelection] = useState<SessionNavigationSelection>({
    kind: 'initial',
  });
  const selection = controlled ?? localSelection;
  const previousSelection = useRef(selection);
  const revealRevision = useRef(0);
  if (previousSelection.current !== selection) {
    previousSelection.current = selection;
    revealRevision.current++;
  }
  const select = useCallback(
    (value: SessionNavigationSelection) => {
      if (onSelectionChange) onSelectionChange(value);
      else setLocalSelection(value);
    },
    [onSelectionChange],
  );
  const collection = useAgentSessionCollection(client, navigationClient);
  const selectedSessionId = sessionIdOf(selection);
  const draftId = selection.kind === 'draft' ? selection.draftId : undefined;
  const draftFolderTarget = selection.kind === 'draft' ? selection.folderTarget : null;
  const draftRepositoryId =
    draftFolderTarget?.kind === 'repository'
      ? draftFolderTarget.repositoryId
      : draftFolderTarget?.kind === 'workflow_instance'
        ? collection.data.instances.find((instance) => instance.id === draftFolderTarget.instanceId)
            ?.repositoryId
        : null;
  const targetDraft = useSessionTarget(
    executionTargetClient,
    executionConfigurationClient,
    selectedSessionId,
    draftId,
    draftFolderTarget,
    branchSource,
    draftRepositoryId,
  );
  const { preserveOnAcknowledgement, adoptCurrent, acceptReady } = targetDraft;
  useEffect(() => {
    if (selection.kind !== 'initial' || collection.loading || collection.error) return;
    const first = collection.summaries[0];
    select(
      first
        ? { kind: 'session', sessionId: first.id }
        : { kind: 'draft', draftId: crypto.randomUUID(), folderTarget: null },
    );
  }, [selection, collection.loading, collection.error, collection.summaries, select]);
  const reloadCollection = collection.reload;
  const onCreated = useCallback(
    (id: string) => {
      preserveOnAcknowledgement(id);
      select({ kind: 'session', sessionId: id });
      void reloadCollection();
    },
    [select, reloadCollection, preserveOnAcknowledgement],
  );
  const view = useProfiledAgentSession(client, profileClient, sessionEventQueryClient, {
    selectedSessionId,
    onSessionCreated: onCreated,
    draftId,
    executionTarget: targetDraft.target,
    executionSelection: targetDraft.selection,
    executionQuickFeatures: selectedTargetQuickFeatures(targetDraft.runtime, targetDraft.profile),
    preparedExecution: Boolean(executionTargetClient && profileClient?.sendPreparedMessage),
    folderTarget: selection.kind === 'draft' ? selection.folderTarget : undefined,
  });
  const session = view.session;
  const currentTarget = session.details?.session.executionTarget ?? null;
  useEffect(() => {
    if (session.details?.session.id === selectedSessionId)
      adoptCurrent(
        currentTarget,
        session.preparation?.selection ??
          (() => {
            const configured = targetDraft.availableProfiles.find(
              (profile) =>
                profile.capabilityProfileId ===
                (session.currentProfile ?? view.profile)?.creationResolution.sessionProfile
                  .capabilityProfileId,
            );
            return configured
              ? {
                  capabilityProfileId: configured.capabilityProfileId,
                  capabilityProfileRevision: configured.revision,
                  execution: configured.execution ?? localExecutionBinding,
                  workspace: { kind: 'auxiliary' as const },
                }
              : null;
          })(),
        (() => {
          const operation = session.queue?.entries
            .map((item) => item.targetOperation)
            .find(
              (operation) =>
                operation?.receipt &&
                operation.stagedTarget?.worktreeId === currentTarget?.worktreeId &&
                operation.request.destination.workspace.kind === 'create',
            );
          return operation?.stagedTarget
            ? { selection: operation.request.destination, target: operation.stagedTarget }
            : null;
        })(),
      );
  }, [
    session.details?.session.id,
    selectedSessionId,
    currentTarget,
    session.preparation?.selection,
    adoptCurrent,
    targetDraft.availableProfiles,
    view.profile,
    session.currentProfile,
    session.queue,
  ]);
  useEffect(() => {
    if (session.preparation?.phase === 'ready')
      acceptReady(session.preparation.selection, session.preparation.resolvedTarget ?? null);
  }, [session.preparation, acceptReady]);
  useEffect(() => {
    if (session.preparing || session.sending) setPreviewOpen(false);
  }, [session.preparing, session.sending]);

  const model = useMemo(() => buildSessionNavigation(collection.data), [collection.data]);
  const [openRevision, setOpenRevision] = useState(0);
  const onSelect = useCallback(
    (sessionId: string) => {
      select({ kind: 'session', sessionId });
      setOpenRevision((value) => value + 1);
    },
    [select],
  );
  const onNew = useCallback(
    (folderTarget: SessionFolderTarget | null) =>
      select({ kind: 'draft', draftId: crypto.randomUUID(), folderTarget }),
    [select],
  );
  const onRename = useCallback(
    async (sessionId: string, title: string) => {
      await collection.rename(sessionId, title);
      if (sessionId === selectedSessionId) await session.reload();
    },
    [collection.rename, selectedSessionId, session.reload],
  );
  const tree = useSessionNavigation(
    model,
    selectedSessionId,
    `${selectionKey(selection)}:${openRevision}:${revealRevision.current}`,
  );
  useSessionNavigationCommands({
    request: agentRequest,
    complete: onAgentComplete,
    model,
    tree,
    selection,
    collectionLoading: collection.loading,
    loading: collection.loading || session.loading,
    error: collection.error,
    onSelect,
    onNew,
    onMove: collection.move,
    onPin: collection.pin,
    onReorder: collection.reorder,
    onOpenWorkflow,
  });
  const selectedIdentity = selectedSessionId
    ? agentIdentityForSession?.(selectedSessionId)
    : undefined;
  const folderTarget = draftFolderTarget;
  const folderLabel =
    folderTarget?.kind === 'repository'
      ? collection.data.repositories.find((repo) => repo.id === folderTarget.repositoryId)?.name
      : folderTarget?.kind === 'workflow_instance'
        ? collection.data.instances.find((instance) => instance.id === folderTarget.instanceId)
            ?.name
        : undefined;
  const emptyState = {
    heading: 'Start with a message',
    guidance: targetDraft.target
      ? `This session will run in ${targetDraft.target.path} on ${targetDraft.target.execution.deviceName}.`
      : folderLabel
        ? `New session in ${folderLabel}. It starts in the repository’s main working tree unless you select a target worktree.`
        : 'Choose a target worktree on this laptop or a remote device. To start in an empty local workspace, choose a local default in Capability Profiles.',
  };
  const evidenceRange =
    focusEvidence &&
    focusEvidence.sessionId === selectedSessionId &&
    focusEvidence.invocationId === focusInvocationId
      ? evidenceTranscriptRange(focusEvidence)
      : undefined;
  useEffect(() => {
    if (!focusInvocationId || !selectedSessionId) return;
    const element = document.querySelector<HTMLElement>(
      `[data-invocation-id="${CSS.escape(focusInvocationId)}"]`,
    );
    element?.focus();
    element?.scrollIntoView({ block: 'center' });
  }, [focusInvocationId, selectedSessionId, session.transcript]);
  const settings =
    profileClient && selectedSessionId ? (
      <AgentSessionExecutionSettings
        modalContent
        profile={view.profile}
        profileError={view.profileError}
        deliveries={view.deliveries.deliveries}
        deliveryError={view.deliveries.error}
        onReloadDeliveries={view.deliveries.reload}
        selection={view.selection}
        onSelectionChange={view.setSelection}
        showMessageControls={!executionTargetClient || !branchSource}
      />
    ) : undefined;
  const resolvedProfile =
    session.currentProfile?.creationResolution.sessionProfile ??
    view.profile?.creationResolution.sessionProfile;
  const routeCapabilities =
    session.quickCatalogue ??
    selectedTargetQuickFeatures(targetDraft.runtime, targetDraft.profile) ??
    (resolvedProfile
      ? {
          configuration: resolvedProfile.configuration,
          defaults: {
            model: resolvedProfile.pinnedDefaults.model,
            reasoningMode: resolvedProfile.pinnedDefaults.reasoningMode,
          },
          models: resolvedProfile.attachedRuntimeCapabilities.models.map((id) => ({
            id,
            label: id,
            description: '',
            defaultReasoningMode: resolvedProfile.pinnedDefaults.reasoningMode,
            reasoningModes: resolvedProfile.attachedRuntimeCapabilities.reasoningModes.map(
              (id) => ({ id, description: '' }),
            ),
          })),
          skills: [],
          limitations: [],
        }
      : undefined);
  const [routeModelCatalogueState, setRouteModelCatalogueState] = useState<{
    readonly key: string;
    readonly client?: ExecutionConfigurationClient;
    readonly values: Readonly<Record<string, ProfileModelCatalogueDto>>;
    readonly settled: boolean;
  }>({ key: '[]', values: {}, settled: true });
  const modelCatalogueRoutes = useMemo(
    () =>
      (targetDraft.profile?.routePolicies ?? [])
        .filter(
          (route) =>
            route.execution.deviceId ===
            (targetDraft.selection?.execution ?? currentTarget?.execution)?.deviceId,
        )
        .map((route) => executionRouteRef(route.execution)),
    [
      targetDraft.profile?.routePolicies,
      targetDraft.selection?.execution,
      currentTarget?.execution,
    ],
  );
  const profileRouteKey = JSON.stringify(modelCatalogueRoutes);
  const modelCatalogueRoutesRef = useRef(modelCatalogueRoutes);
  modelCatalogueRoutesRef.current = modelCatalogueRoutes;
  useEffect(() => {
    let active = true;
    const routes = modelCatalogueRoutesRef.current;
    const client = executionConfigurationClient;
    const load = client?.loadProfileModelCatalogue;
    if (!client || !load || routes.length === 0) {
      setRouteModelCatalogueState({ key: profileRouteKey, client, values: {}, settled: true });
      return;
    }
    const cached = readyRouteModelCatalogues(client, routes);
    const settled = Object.keys(cached).length === routes.length;
    setRouteModelCatalogueState({ key: profileRouteKey, client, values: cached, settled });
    if (settled) return;
    void Promise.all(
      routes.map(async (reference) => {
        return [
          executionRouteKey(reference),
          await cachedRouteModelCatalogue(client, reference),
        ] as const;
      }),
    ).then(
      (entries) =>
        active &&
        setRouteModelCatalogueState({
          key: profileRouteKey,
          client,
          values: Object.fromEntries(entries),
          settled: true,
        }),
      () =>
        active &&
        setRouteModelCatalogueState({
          key: profileRouteKey,
          client,
          values: cached,
          settled: false,
        }),
    );
    return () => {
      active = false;
    };
  }, [executionConfigurationClient, profileRouteKey]);
  const routeModelCatalogues =
    routeModelCatalogueState.key === profileRouteKey &&
    routeModelCatalogueState.client === executionConfigurationClient
      ? routeModelCatalogueState.values
      : readyRouteModelCatalogues(executionConfigurationClient, modelCatalogueRoutes);
  const routeModelsLoading =
    modelCatalogueRoutes.length > 0 &&
    !(
      routeModelCatalogueState.key === profileRouteKey &&
      routeModelCatalogueState.client === executionConfigurationClient &&
      routeModelCatalogueState.settled
    );
  const capabilities = withCapabilityProfileDeviceModels(
    routeCapabilities,
    capabilityProfileDeviceModels(
      targetDraft.profile,
      targetDraft.selection?.execution ?? currentTarget?.execution,
      routeModelCatalogues,
    ),
  );
  const messageSelection = view.selection;
  const setMessageSelection = view.setSelection;
  useEffect(() => {
    if (!capabilities || targetDraft.loading || routeModelsLoading) return;
    if (!messageSelection.model && !messageSelection.reasoningMode) return;
    if (
      messageSelection.model &&
      !capabilities.models.some((model) => model.id === messageSelection.model)
    ) {
      setMessageSelection({ model: null, reasoningMode: null });
      return;
    }
    const effective = effectiveSessionOptions(capabilities, messageSelection);
    if (messageSelection.reasoningMode !== effective.reasoningMode)
      setMessageSelection({
        model: messageSelection.model,
        reasoningMode: effective.reasoningMode,
      });
  }, [
    capabilities,
    targetDraft.loading,
    routeModelsLoading,
    messageSelection,
    setMessageSelection,
  ]);
  const models = capabilities?.models.map((model) => model.id) ?? [];
  const effectiveOptions = effectiveSessionOptions(capabilities, messageSelection);
  const hasRuntimeFacts = Boolean(capabilities);
  const selectionError = targetDraft.branchChoice?.requiresWorktree
    ? 'Choose or create a worktree for this branch.'
    : targetDraft.deviceId && !targetDraft.selection
      ? 'Choose a Capability Profile for this device.'
      : targetDraft.selection?.execution.connection.kind === 'ssh' &&
          targetDraft.selection.workspace.kind === 'auxiliary'
        ? 'Choose a worktree on this device.'
        : hasRuntimeFacts &&
            view.selection.model &&
            !targetDraft.loading &&
            !models.includes(view.selection.model)
          ? `Model ${view.selection.model} is unavailable in this Capability Profile on the selected device.`
          : hasRuntimeFacts &&
              view.selection.reasoningMode &&
              !targetDraft.loading &&
              !effectiveOptions.model?.reasoningModes.some(
                (mode) => mode.id === view.selection.reasoningMode,
              )
            ? `Reasoning ${view.selection.reasoningMode} is unavailable for the selected model.`
            : undefined;
  const sendUnavailableReason = selectionError ?? view.sendUnavailableReason;
  const workspaceChoice = targetDraft.selection?.workspace;
  const pending = Boolean(
    targetDraft.selection &&
    (workspaceChoice?.kind === 'create' ||
      (currentTarget
        ? targetDraft.selection.capabilityProfileId !== currentTarget.capabilityProfileId ||
          targetDraft.selection.capabilityProfileRevision !==
            currentTarget.capabilityProfileRevision ||
          JSON.stringify(targetDraft.selection.execution) !==
            JSON.stringify(currentTarget.execution) ||
          targetDraft.selection.execution.deviceId !== currentTarget.execution.deviceId ||
          workspaceChoice?.kind !== 'existing' ||
          workspaceChoice.target.worktreeId !== currentTarget.worktreeId
        : session.preparation?.phase !== 'ready' ||
          JSON.stringify(targetDraft.selection) !== JSON.stringify(session.preparation.selection))),
  );
  const preview = useMemo(
    () => (previewOpen ? preparationPreview(targetDraft.selection, currentTarget) : null),
    [previewOpen, targetDraft.selection, currentTarget],
  );
  const toolbar =
    executionTargetClient && profileClient && branchSource ? (
      <SessionComposerToolbar
        profiles={targetDraft.availableProfiles}
        selection={targetDraft.selection}
        deviceId={targetDraft.deviceId}
        options={view.selection}
        capabilities={capabilities}
        workspaceLabel={targetDraft.workspaceLabel}
        pending={pending}
        onProfile={targetDraft.chooseProfile}
        onDevice={() => setDeviceContinuationOpen(true)}
        onWorktree={() => setTargetPicker({ deviceId: targetDraft.deviceId ?? undefined })}
        onOptions={view.setSelection}
        onPreview={() => setPreviewOpen(!previewOpen)}
      />
    ) : undefined;
  const queuePanel = session.queue ? (
    <SessionQueuePanel
      queue={session.queue}
      steeringEntries={session.steeringQueue}
      controller={session}
      onChooseTarget={(queueEntry) =>
        setTargetPicker({
          deviceId: queueEntry.targetOperation?.request.destination.execution.deviceId,
          queueEntry,
        })
      }
    />
  ) : null;
  const deviceActivityPanel = (
    <SessionDeviceActivityList activities={session.queuePresentation?.deviceActivities ?? []} />
  );
  const composerDialog = preview ? (
    <SessionPreparationPanel
      preparation={null}
      preview={preview}
      deviceName={targetDraft.selection?.execution.deviceName ?? 'this device'}
      onClosePreview={() => setPreviewOpen(false)}
      onCancel={() => undefined}
    />
  ) : null;
  const workspace =
    selection.kind === 'initial' ? (
      <p role="status">Loading sessions…</p>
    ) : (
      <AgentSessionHeaderActionsProvider actions={null}>
        <AgentSessionWorkspace
          controller={session}
          composerToolbar={toolbar}
          deviceActivityPanel={deviceActivityPanel}
          queuePanel={queuePanel}
          composerDialog={composerDialog}
          targetSource={
            executionTargetClient && branchSource && profileClient
              ? {
                  contextKey: selectionKey(selection),
                  sessionId: selectedSessionId ?? undefined,
                  client: executionTargetClient,
                  target: targetDraft.target,
                  onSelectTarget: (target) => {
                    if (selectedSessionId && target)
                      setTargetPicker({ deviceId: target.execution.deviceId, target });
                    else targetDraft.setTarget(target);
                  },
                  onBrowseBranches: (deviceId) => setTargetPicker({ deviceId }),
                }
              : undefined
          }
          sendUnavailableReason={sendUnavailableReason}
          transcriptRange={evidenceRange}
          inspection={
            focusEvidence && focusEvidence.sessionId === selectedSessionId
              ? { sessionId: focusEvidence.sessionId, invocationId: focusEvidence.invocationId }
              : undefined
          }
          presentation={{
            compactHeader: true,
            showCopyAction: false,
            emptyState,
            ...(selectedIdentity
              ? {
                  identityHeader: {
                    agentIdentity: selectedIdentity,
                    title:
                      session.details?.session.title ??
                      legacyHarnessRoleLabel(selectedIdentity.harnessRole),
                  },
                }
              : {}),
          }}
        />
      </AgentSessionHeaderActionsProvider>
    );
  return (
    <>
      {targetPicker && executionTargetClient && branchSource && (
        <SessionTargetDialog
          client={executionTargetClient}
          source={branchSource}
          selected={
            targetPicker.queueEntry?.targetOperation?.request.destination.workspace.kind ===
            'existing'
              ? targetPicker.queueEntry.targetOperation.request.destination.workspace.target
              : targetPicker.queueEntry?.message
                ? null
                : (targetPicker.target ?? targetDraft.target)
          }
          initialRepositoryId={
            targetPicker.queueEntry?.targetOperation?.request.destination.workspace.kind ===
            'create'
              ? targetPicker.queueEntry.targetOperation.request.destination.workspace.repositoryId
              : (draftRepositoryId ?? undefined)
          }
          initialBranchRef={
            targetPicker.queueEntry?.targetOperation?.request.destination.workspace.kind ===
            'create'
              ? targetPicker.queueEntry.targetOperation.request.destination.workspace.branchRef
              : targetDraft.branchChoice?.branchRef
          }
          deviceId={targetPicker.deviceId}
          capabilityProfileId={
            targetPicker.deviceId === targetDraft.deviceId
              ? targetDraft.selection?.capabilityProfileId
              : undefined
          }
          sessionId={selectedSessionId ?? undefined}
          onClose={() => setTargetPicker(null)}
          sourceTarget={
            session.queue
              ? projectedQueueSource(session.queue, targetPicker.queueEntry?.entry.id)
              : currentTarget
          }
          initialRequest={targetPicker.queueEntry?.targetOperation?.request}
          onSelect={(target) => {
            targetDraft.setTarget(target);
            setTargetPicker(null);
          }}
          onSelectRequest={(request) => {
            if (targetPicker.queueEntry?.targetOperation)
              void session.updateTargetOperation?.({
                entryId: targetPicker.queueEntry.entry.id,
                request,
              });
            else if (selectedSessionId)
              void session.enqueueTargetOperation?.(request, targetPicker.queueEntry?.entry.id);
            targetDraft.setSelection(request.destination);
            setTargetPicker(null);
          }}
        />
      )}
      {deviceContinuationOpen && executionTargetClient && branchSource && (
        <DeviceContinuationDialog
          client={executionTargetClient}
          sessionId={selectedSessionId ?? undefined}
          source={branchSource}
          sourceTarget={
            session.queue
              ? projectedQueueSource(session.queue)
              : (currentTarget ?? targetDraft.target)
          }
          onChooseDevice={targetDraft.chooseDevice}
          onQueueSelection={
            selectedSessionId
              ? async (request) => {
                  targetDraft.setSelection(request.destination);
                  await session.enqueueTargetOperation?.(request);
                }
              : undefined
          }
          onClose={() => setDeviceContinuationOpen(false)}
        />
      )}
      {importOpen && importClient && (
        <ImportCodexSessionDialog
          client={importClient}
          onClose={() => setImportOpen(false)}
          onImported={(id) => {
            setImportOpen(false);
            onCreated(id);
          }}
        />
      )}
      <main className="agent-session-screen">
        <ResizableSplitSurface
          axis="horizontal"
          primaryLabel="Agent Session navigation"
          secondaryLabel="Selected Agent Session"
          initialPrimaryPercent={25}
          minimumPrimaryPixels={240}
          minimumSecondaryPixels={480}
          compactBreakpoint={860}
          primary={
            <SessionSelector
              onImport={importClient ? () => setImportOpen(true) : undefined}
              onOpenWorkflow={onOpenWorkflow}
              model={model}
              selectedSessionId={selectedSessionId}
              tree={tree}
              loading={collection.loading}
              onSelect={onSelect}
              onNew={onNew}
              onMove={collection.move}
              onPin={collection.pin}
              onRename={onRename}
              onCopySession={copyEntireSession}
              onSetIdentity={
                profileClient && client.updateIdentity
                  ? (id) => {
                      onSelect(id);
                      setIdentityError(null);
                      setSessionDialog({ id, kind: 'identity' });
                    }
                  : undefined
              }
              onConfigure={
                profileClient || harnessManagementSource
                  ? (id) => {
                      onSelect(id);
                      setSessionDialog({ id, kind: 'configuration' });
                    }
                  : undefined
              }
              onReorder={collection.reorder}
              organizing={Boolean(navigationClient)}
              onReload={() => {
                void reloadCollection();
                void session.reload();
                view.deliveries.reload();
              }}
            />
          }
          secondary={
            <div className="agent-session-content">
              {session.error?.includes('Choose a default Capability Profile') &&
                onConfigureCapabilities && (
                  <button onClick={onConfigureCapabilities}>
                    Choose default Capability Profile
                  </button>
                )}
              {collection.error && (
                <section className="agent-session-error" role="alert">
                  <AlertCircle size={17} />
                  <span>{collection.error}</span>
                  <button
                    className="icon-button"
                    onClick={collection.clearError}
                    aria-label="Dismiss error"
                  >
                    <X size={15} />
                  </button>
                </section>
              )}
              {workspace}
            </div>
          }
        />
      </main>
      {sessionDialog?.kind === 'configuration' && selectedSessionId === sessionDialog.id && (
        <ModalDialog
          labelledBy="session-configuration-title"
          onClose={() => setSessionDialog(null)}
          className="session-configuration-dialog"
        >
          <header>
            <h2 id="session-configuration-title">Session configuration</h2>
            <button
              type="button"
              aria-label="Close session configuration"
              onClick={() => setSessionDialog(null)}
            >
              <X size={18} />
            </button>
          </header>
          {harnessManagementSource ? (
            <HarnessAwareAgentSessionPane
              sessionId={sessionDialog.id}
              source={harnessManagementSource}
            >
              {settings}
            </HarnessAwareAgentSessionPane>
          ) : (
            settings
          )}
        </ModalDialog>
      )}
      {sessionDialog?.kind === 'identity' &&
        selectedSessionId === sessionDialog.id &&
        session.details?.session.id === sessionDialog.id && (
          <>
            <IdentityPickerDialog
              identity={
                session.details.session.assignedIdentity ?? {
                  originIdentityId: null,
                  displayName: 'Agent',
                  color: '#39745a',
                  shape: 'circle',
                }
              }
              title="Set identity"
              onClose={() => setSessionDialog(null)}
              onSave={(identity) => {
                if (!view.updateIdentity) return;
                void view.updateIdentity(identity).then(
                  () => setSessionDialog(null),
                  (cause) => setIdentityError(String(cause)),
                );
              }}
            />
            {identityError && (
              <p className="session-identity-error" role="alert">
                {identityError}
              </p>
            )}
          </>
        )}
    </>
  );
}
function evidenceTranscriptRange(
  destination: ProductDecisionEvidenceDestination,
): TranscriptAnchorRange {
  const anchor = {
    sessionId: destination.sessionId,
    invocationId: destination.invocationId,
    kind: destination.passage.kind,
    ...('runtimeEventId' in destination.passage
      ? { runtimeEventId: destination.passage.runtimeEventId }
      : {}),
  } as TranscriptAnchorRange['start'];
  return { start: anchor, end: anchor };
}

export const AgentSessionScreen = StandaloneAgentSessionScreen;
