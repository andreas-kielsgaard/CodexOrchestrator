import type { AgentSessionImportClient } from '../../application/agentSessions/importContracts';
import { ImportCodexSessionDialog } from './ImportCodexSessionDialog';
import type { SessionWorkflowTarget } from '../../application/agentSessions/workflowNavigation';
import type { RepositoryBranchSource } from '../../application/branches';
import type { ExecutionConfigurationClient } from '../../application/executionConfiguration';
import type { ExecutionTargetClient } from '../../application/executionTargets/contracts';
import { useSessionTarget } from './useSessionTarget';
import { PerMessageRuntimeControls } from './PerMessageRuntimeControls';
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
  );
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
      select({ kind: 'session', sessionId: id });
      void reloadCollection();
    },
    [select, reloadCollection],
  );
  const view = useProfiledAgentSession(client, profileClient, sessionEventQueryClient, {
    selectedSessionId,
    onSessionCreated: onCreated,
    draftId,
    executionTarget: targetDraft.target,
    folderTarget: selection.kind === 'draft' ? selection.folderTarget : undefined,
  });
  const session = view.session;
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
        disabled={session.sending}
        identity={session.details?.session.assignedIdentity}
        onIdentityChange={view.updateIdentity}
      />
    ) : targetDraft.target ? (
      <div className="session-target-draft-options">
        <p>
          Capability Profile:{' '}
          <strong>{targetDraft.profile?.name ?? targetDraft.target.capabilityProfileId}</strong>
        </p>
        {targetDraft.error && <p role="alert">{targetDraft.error}</p>}
        <PerMessageRuntimeControls
          value={view.selection}
          onChange={view.setSelection}
          disabled={session.sending || targetDraft.loading}
          models={(targetDraft.runtime?.exposure.models ?? [])
            .filter((model) => targetDraft.profile?.allowedCapabilities.models.includes(model))
            .map((value) => ({ value, label: value }))}
          reasoningModes={(targetDraft.runtime?.exposure.reasoningModes ?? [])
            .filter((mode) =>
              targetDraft.profile?.allowedCapabilities.reasoningModes.includes(mode),
            )
            .map((value) => ({ value, label: value }))}
          defaultModelLabel={targetDraft.profile?.defaults?.model ?? 'Use profile default'}
          defaultReasoningLabel={
            targetDraft.profile?.defaults?.reasoningMode ?? 'Use profile default'
          }
        />
      </div>
    ) : undefined;
  const sendUnavailableReason =
    !selectedSessionId &&
    targetDraft.target &&
    (targetDraft.loading || targetDraft.error || !targetDraft.runtime)
      ? (targetDraft.error ?? 'Loading target capabilities…')
      : view.sendUnavailableReason;
  const targetControl =
    executionTargetClient && branchSource && profileClient
      ? {
          client: executionTargetClient,
          branches: branchSource,
          target: selectedSessionId
            ? (session.details?.session.executionTarget ?? null)
            : targetDraft.target,
          fixed: Boolean(selectedSessionId),
          disabled: session.sending,
          onChange: targetDraft.setTarget,
        }
      : undefined;
  const workspace =
    selection.kind === 'initial' ? (
      <p role="status">Loading sessions…</p>
    ) : (
      <AgentSessionHeaderActionsProvider actions={null} settings={settings}>
        <AgentSessionWorkspace
          controller={session}
          targetControl={targetControl}
          targetSource={
            targetControl
              ? {
                  contextKey: selectionKey(selection),
                  client: targetControl.client,
                  target: targetControl.target,
                  disabledReason: selectedSessionId
                    ? 'This Session target is fixed after its first prompt.'
                    : session.sending
                      ? 'Target selection is unavailable while sending.'
                      : undefined,
                  onSelectTarget: targetDraft.setTarget,
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
