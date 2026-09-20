import type { AgentSessionImportClient } from '../../application/agentSessions/importContracts';
import { ImportCodexSessionDialog } from './ImportCodexSessionDialog';
import type { SessionWorkflowTarget } from '../../application/agentSessions/workflowNavigation';
import type { RepositoryBranchSource } from '../../application/branches';
import type { ExecutionConfigurationClient } from '../../application/executionConfiguration';
import {
  localExecutionBinding,
  type ExecutionTargetClient,
} from '../../application/executionTargets/contracts';
import { selectedTargetQuickFeatures } from './selectedTargetQuickFeatures';
import { useSessionTarget } from './useSessionTarget';
import { SessionComposerToolbar } from './SessionComposerToolbar';
import { SessionPreparationPanel } from './SessionPreparationPanel';
import { preparationPreview } from './preparationPreview';
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
import { HarnessAwareAgentSessionPane } from '../conversationHarnesses/HarnessAwareAgentSessionPane';
import { SessionSelector } from './SessionSelector';
import { useAgentSessionCollection } from './useAgentSessionCollection';
import { useProfiledAgentSession } from './useProfiledAgentSession';
import { ResizableSplitSurface } from '../orchestrations/components/ResizableSplitSurface';
import './agentSession.css';
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
  const [targetPicker, setTargetPicker] = useState<{ deviceId?: string } | null>(null);
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
  const targetDraft = useSessionTarget(
    executionTargetClient,
    executionConfigurationClient,
    selectedSessionId,
    draftId,
    selection.kind === 'draft' ? selection.folderTarget : null,
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
  const onSelect = (sessionId: string) => {
    select({ kind: 'session', sessionId });
    setOpenRevision((value) => value + 1);
  };
  const onNew = (folderTarget: SessionFolderTarget | null) =>
    select({ kind: 'draft', draftId: crypto.randomUUID(), folderTarget });
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
  const folderTarget = selection.kind === 'draft' ? selection.folderTarget : null;
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
        profile={view.profile}
        profileError={view.profileError}
        deliveries={view.deliveries.deliveries}
        deliveryError={view.deliveries.error}
        onReloadDeliveries={view.deliveries.reload}
        selection={view.selection}
        onSelectionChange={view.setSelection}
        identity={session.details?.session.assignedIdentity}
        onIdentityChange={view.updateIdentity}
        showMessageControls={!executionTargetClient || !branchSource}
      />
    ) : undefined;
  const resolvedProfile =
    session.currentProfile?.creationResolution.sessionProfile ??
    view.profile?.creationResolution.sessionProfile;
  const models = targetDraft.profile
    ? (targetDraft.runtime?.exposure.models ?? []).filter((value) =>
        targetDraft.profile!.allowedCapabilities.models.includes(value),
      )
    : (resolvedProfile?.attachedRuntimeCapabilities.models ?? []);
  const reasoningModes = targetDraft.profile
    ? (targetDraft.runtime?.exposure.reasoningModes ?? []).filter((value) =>
        targetDraft.profile!.allowedCapabilities.reasoningModes.includes(value),
      )
    : (resolvedProfile?.attachedRuntimeCapabilities.reasoningModes ?? []);
  const hasRuntimeFacts = Boolean(targetDraft.profile ? targetDraft.runtime : resolvedProfile);
  const selectionError =
    targetDraft.deviceId && !targetDraft.selection
      ? 'Choose a Capability Profile for this device.'
      : targetDraft.selection?.execution.connection.kind === 'ssh' &&
          targetDraft.selection.workspace.kind === 'auxiliary'
        ? 'Choose a worktree on this device.'
        : hasRuntimeFacts &&
            view.selection.model &&
            !targetDraft.loading &&
            !models.includes(view.selection.model)
          ? `Model ${view.selection.model} is unavailable for the selected profile.`
          : hasRuntimeFacts &&
              view.selection.reasoningMode &&
              !targetDraft.loading &&
              !reasoningModes.includes(view.selection.reasoningMode)
            ? `Reasoning ${view.selection.reasoningMode} is unavailable for the selected profile.`
            : undefined;
  const sendUnavailableReason =
    selectionError ??
    (targetDraft.selection && (targetDraft.loading || targetDraft.error || !targetDraft.runtime)
      ? (targetDraft.error ?? 'Loading target capabilities…')
      : view.sendUnavailableReason);
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
        models={models}
        reasoningModes={reasoningModes}
        defaultModel={targetDraft.profile?.defaults?.model ?? resolvedProfile?.pinnedDefaults.model}
        defaultReasoning={
          targetDraft.profile?.defaults?.reasoningMode ??
          resolvedProfile?.pinnedDefaults.reasoningMode
        }
        pending={pending}
        onProfile={targetDraft.chooseProfile}
        onDevice={() => setDeviceContinuationOpen(true)}
        onWorktree={() => setTargetPicker({ deviceId: targetDraft.deviceId ?? undefined })}
        onOptions={view.setSelection}
        onPreview={() => setPreviewOpen(!previewOpen)}
      />
    ) : undefined;
  const preparationPanel = (
    <SessionPreparationPanel
      preparation={session.preparation}
      preview={preview}
      deviceName={
        (previewOpen ? targetDraft.selection : session.preparation?.selection)?.execution
          .deviceName ?? 'this device'
      }
      onClosePreview={() => setPreviewOpen(false)}
      onCancel={() => void session.cancel()}
      onRetry={() => void session.retryPreparation?.()}
    />
  );
  const workspace =
    selection.kind === 'initial' ? (
      <p role="status">Loading sessions…</p>
    ) : (
      <AgentSessionHeaderActionsProvider actions={null} settings={settings}>
        <AgentSessionWorkspace
          controller={session}
          composerToolbar={toolbar}
          preparationPanel={preparationPanel}
          targetSource={
            executionTargetClient && branchSource && profileClient
              ? {
                  contextKey: selectionKey(selection),
                  sessionId: selectedSessionId ?? undefined,
                  client: executionTargetClient,
                  target: targetDraft.target,
                  onSelectTarget: targetDraft.setTarget,
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
            emptyState,
            ...(selectedIdentity
              ? {
                  identityHeader: {
                    agentIdentity: selectedIdentity,
                    title: legacyHarnessRoleLabel(selectedIdentity.harnessRole),
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
          selected={targetDraft.target}
          deviceId={targetPicker.deviceId}
          capabilityProfileId={
            targetPicker.deviceId === targetDraft.deviceId
              ? targetDraft.selection?.capabilityProfileId
              : undefined
          }
          sessionId={selectedSessionId ?? undefined}
          onClose={() => setTargetPicker(null)}
          onSelect={(target) => {
            targetDraft.setTarget(target);
            setTargetPicker(null);
          }}
          onSelectSelection={(next) => {
            targetDraft.setSelection(next);
            setTargetPicker(null);
          }}
        />
      )}
      {deviceContinuationOpen && executionTargetClient && branchSource && (
        <DeviceContinuationDialog
          client={executionTargetClient}
          profileClient={profileClient}
          sessionId={selectedSessionId ?? undefined}
          source={branchSource}
          sourceTarget={currentTarget ?? targetDraft.target}
          onChooseDevice={targetDraft.chooseDevice}
          onTransitionChanged={() => {
            void session.reload();
            void reloadCollection();
          }}
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
              {selectedSessionId && harnessManagementSource ? (
                <HarnessAwareAgentSessionPane
                  sessionId={selectedSessionId}
                  source={harnessManagementSource}
                >
                  {workspace}
                </HarnessAwareAgentSessionPane>
              ) : (
                workspace
              )}
            </div>
          }
        />
      </main>
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
