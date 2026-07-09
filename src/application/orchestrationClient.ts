import type { EntityId } from '../domain/model';
import type {
  AgentConversation,
  AgentConversationEvidence,
  AgentConversationEvidenceKind,
  AgentConversationStreamStatus,
  AgentConversationTurnRole,
} from '../domain/agentConversation';
import type {
  OrchestrationProvenance,
  OrchestrationStatus,
  OrchestrationTruthState,
} from '../domain/orchestrationState';

export type OrchestrationLifecycleState =
  | OrchestrationStatus
  | 'idle'
  | 'queued'
  | 'planning'
  | 'delegated'
  | 'delegating'
  | 'working'
  | 'waiting'
  | 'reviewing'
  | 'merging'
  | 'reporting'
  | 'recording'
  | 'merged'
  | 'recorded';

export type OrchestrationBuildStageId =
  'plan-builder' | 'plan-review' | 'instantiator' | 'root-startup';

export const localDraftTruthState: OrchestrationTruthState = {
  status: 'draft',
  provenance: 'local_draft',
};

export const integrationPendingTruthState: OrchestrationTruthState = {
  status: 'integration_pending',
  provenance: 'unsupported',
};

export interface OrchestrationClientNotice {
  id: EntityId;
  kind: 'error' | 'blocker' | 'missing_capability';
  title: string;
  message: string;
  truth: OrchestrationTruthState;
}

export interface OrchestrationClientAction {
  id:
    | 'create-draft'
    | 'request-build-stage'
    | 'start-instantiation'
    | 'start-orchestration'
    | 'add-local-note'
    | 'attach-local-files';
  label: string;
  enabled: boolean;
  reason?: string;
}

export interface OrchestrationClientState {
  id?: EntityId;
  status: OrchestrationStatus;
  provenance: OrchestrationProvenance;
  currentAction: string;
  updatedAt?: string;
  persisted: boolean;
  runtimeSupported: boolean;
  notices: OrchestrationClientNotice[];
  primaryAction?: OrchestrationClientAction;
}

export interface OrchestrationBuildStage {
  id: OrchestrationBuildStageId;
  title: string;
  state: OrchestrationTruthState;
  summary: string;
  detail: string;
}

export interface OrchestrationStageRunEvidence {
  id: EntityId;
  buildPackageId: EntityId;
  stageId: OrchestrationBuildStageId;
  state: OrchestrationTruthState;
  statusReason?: string;
  promptArtifactId?: EntityId;
  outputArtifactId?: EntityId;
  rawEventArtifactId?: EntityId;
  taskId?: EntityId;
  taskRunId?: EntityId;
  conversationId?: EntityId;
  eventIds: EntityId[];
  evidence: Record<string, unknown>;
  startedAt?: string;
  completedAt?: string;
  createdAt: string;
  updatedAt: string;
}

export type OrchestrationRuntimeRouteStatus =
  'supported' | 'blocked' | 'integration_pending' | 'unsupported';

export interface OrchestrationRuntimeRoute {
  stageId: OrchestrationBuildStageId;
  status: OrchestrationRuntimeRouteStatus;
  truth: OrchestrationTruthState;
  reason: string;
  taskId?: EntityId;
  worktreeId?: EntityId;
  cwd?: string;
  runtimeCommand?: 'startCodexTaskRun' | 'startOrchestrationPlanBuilderRun';
  updatedAt?: string;
}

export interface OrchestrationStep {
  id: EntityId;
  title: string;
  role: string;
  state: OrchestrationLifecycleState;
  timestamp: string;
  prompt: string;
  output: string;
}

export interface OrchestrationAgentWindow {
  id: EntityId;
  title: string;
  role: string;
  threadId: string;
  state: OrchestrationLifecycleState;
  currentAction: string;
  lastOutput: string;
  planNodeId?: EntityId;
  blockerIds: EntityId[];
  lastUpdatedAt: string;
}

export interface OrchestrationWorkSlice {
  id: EntityId;
  title: string;
  repo: string;
  state: OrchestrationLifecycleState;
  lifecycleStage: string;
  currentTurn: string;
  summary: string;
  blockerIds: EntityId[];
  delegationThread: OrchestrationAgentWindow;
  workerThread: OrchestrationAgentWindow;
  steps: OrchestrationStep[];
  recordNote: string;
}

export interface OrchestrationPlannerTurn {
  id: EntityId;
  title: string;
  state: OrchestrationLifecycleState;
  startedAt: string;
  reasoningSummary: string;
  explicitPlan: string[];
  planNodeIds: EntityId[];
  blockerIds: EntityId[];
  workSlices: OrchestrationWorkSlice[];
}

export interface OrchestrationRecordEntry {
  id: EntityId;
  title: string;
  timestamp: string;
  summary: string;
}

export type OrchestrationBlockerState = 'open' | 'addressed' | 'deferred';
export type OrchestrationBlockerKind = 'technical' | 'decision' | 'dependency';

export interface OrchestrationBlocker {
  id: EntityId;
  title: string;
  state: OrchestrationBlockerState;
  kind: OrchestrationBlockerKind;
  severity: 'low' | 'medium' | 'high';
  summary: string;
  detail: string;
  resolutionQuestion: string;
  nextPlannerContext: string;
  associatedPlanNodeIds: EntityId[];
  associatedTaskIds: EntityId[];
  createdByRole: string;
  createdAt: string;
}

export interface OrchestrationRootTurn {
  id: EntityId;
  title: string;
  state: OrchestrationLifecycleState;
  lastUpdatedAt: string;
  currentAction: string;
  lastOutput: string;
  instantiatedPlannerIds: EntityId[];
  blockerIds: EntityId[];
}

export interface BlockerConclusion {
  state: Extract<OrchestrationBlockerState, 'addressed' | 'deferred'>;
  conclusion: string;
  updatedAt: string;
}

export interface OrchestrationPlanNode {
  id: EntityId;
  title: string;
  state: OrchestrationLifecycleState;
  summary: string;
  statusDetail: string;
  blockerIds: EntityId[];
  activeRefs: string[];
  children: OrchestrationPlanNode[];
}

export interface OrchestrationSnapshot {
  id: EntityId;
  title: string;
  objective: string;
  anchor: string;
  state: OrchestrationLifecycleState;
  clientState: OrchestrationClientState;
  currentPosition: string;
  plan: OrchestrationPlanNode;
  rootTurns: OrchestrationRootTurn[];
  activeRuns: Array<{
    id: EntityId;
    title: string;
    role: string;
    state: OrchestrationLifecycleState;
    planNodeId: EntityId;
    detail: string;
  }>;
  agentWindows: OrchestrationAgentWindow[];
  blockers: OrchestrationBlocker[];
  planners: OrchestrationPlannerTurn[];
  recordEntries: OrchestrationRecordEntry[];
}

export interface OrchestrationConversationMessage {
  id: EntityId;
  role: 'user' | 'assistant' | 'system';
  body: string;
  createdAt: string;
  state?: 'completed' | 'processing';
  truth?: OrchestrationTruthState;
}

export interface UploadedConversationFile {
  id: EntityId;
  name: string;
  size: number;
  lastModified?: number;
}

export interface OrchestrationBuildPackage {
  id: EntityId;
  title: string;
  folderPath: string;
  sourcePrompt: string;
  createdAt: string;
  updatedAt: string;
  clientState: OrchestrationClientState;
  messages: OrchestrationConversationMessage[];
  files: UploadedConversationFile[];
  stages: OrchestrationBuildStage[];
  stageRuns?: OrchestrationStageRunEvidence[];
  runtimeRoutes?: OrchestrationRuntimeRoute[];
  generatedFiles: Array<{
    name: string;
    purpose: string;
    state: OrchestrationTruthState;
  }>;
  planPreview: string[];
}

export interface OrchestrationRegistrySnapshot {
  orchestrations: OrchestrationSnapshot[];
  buildPackages: OrchestrationBuildPackage[];
  clientState: OrchestrationClientState;
}

export interface CreateOrchestrationDraftInput {
  title: string;
  folderPath: string;
  prompt: string;
  files: UploadedConversationFile[];
}

export interface AddOrchestrationDraftNoteInput {
  buildPackageId: EntityId;
  body: string;
}

export interface AttachOrchestrationDraftFilesInput {
  buildPackageId: EntityId;
  files: UploadedConversationFile[];
}

export interface RequestOrchestrationBuildStageInput {
  buildPackageId: EntityId;
  stageId: OrchestrationBuildStageId;
}

export interface StartOrchestrationPlanBuilderRunInput {
  buildPackageId: EntityId;
}

export interface StartOrchestrationInput {
  buildPackageId: EntityId;
}

export interface StartOrchestrationResult {
  buildPackage?: OrchestrationBuildPackage;
  orchestration?: OrchestrationSnapshot;
  clientState: OrchestrationClientState;
}

export interface OrchestrationClient {
  loadOrchestrations(): Promise<OrchestrationRegistrySnapshot>;
  createDraft(input: CreateOrchestrationDraftInput): Promise<OrchestrationBuildPackage>;
  addDraftNote(input: AddOrchestrationDraftNoteInput): Promise<OrchestrationBuildPackage>;
  attachDraftFiles(input: AttachOrchestrationDraftFilesInput): Promise<OrchestrationBuildPackage>;
  requestBuildStage(input: RequestOrchestrationBuildStageInput): Promise<OrchestrationBuildPackage>;
  startPlanBuilderRun(input: StartOrchestrationPlanBuilderRunInput): Promise<OrchestrationBuildPackage>;
  startOrchestration(input: StartOrchestrationInput): Promise<StartOrchestrationResult>;
  loadOrchestration(id: EntityId): Promise<OrchestrationSnapshot | null>;
  cancelDraft(buildPackageId: EntityId): Promise<OrchestrationRegistrySnapshot>;
}

export function mapBuildPackageToAgentConversation(
  buildPackage: OrchestrationBuildPackage,
): AgentConversation {
  const truth: OrchestrationTruthState = {
    provenance: buildPackage.clientState.provenance,
    status: buildPackage.clientState.status,
  };
  const evidence = toAgentConversationEvidence(truth);
  const unsupportedNotice = buildPackage.clientState.notices.find(
    (notice) => notice.kind === 'missing_capability' || notice.truth.provenance === 'unsupported',
  );
  const streamStatus = toAgentConversationStreamStatus(truth);
  const latestStageRun = (buildPackage.stageRuns ?? []).at(-1);
  const externalThreadId =
    typeof latestStageRun?.evidence.externalThreadId === 'string'
      ? latestStageRun.evidence.externalThreadId
      : undefined;

  return {
    artifacts: [
      ...stageRunArtifacts(buildPackage),
      ...buildPackage.generatedFiles.map((file) => ({
        detail: file.purpose,
        evidence: toAgentConversationEvidence(file.state),
        id: file.name,
        kind: generatedFileArtifactKind(file.state),
        name: file.name,
        truth: file.state,
      })),
    ],
    attachments: buildPackage.files.map((file) => ({
      detail: `${file.size} bytes`,
      evidence: toAgentConversationEvidence(localDraftTruthState),
      id: file.id,
      kind: 'uploaded',
      name: file.name,
      truth: localDraftTruthState,
    })),
    id: buildPackage.id,
    input: {
      disabledReason: buildPackage.clientState.runtimeSupported
        ? undefined
        : 'Runtime continuation is not supported for this draft package yet.',
      enabled: buildPackage.clientState.runtimeSupported && truth.status === 'ready',
      placeholder: 'Continue the agent conversation',
    },
    mode: buildPackage.clientState.runtimeSupported ? 'interactive' : 'read_only',
    role: 'Plan Builder',
    runtime: {
      providerId: 'codex-orchestrator',
      providerLabel: 'Codex Orchestrator',
      runtimeId: latestStageRun?.conversationId,
      runtimeLabel: buildPackage.clientState.runtimeSupported
        ? 'Runtime support reported by backend'
        : 'No supported runtime route',
      unsupportedReason: unsupportedNotice?.message,
    },
    state: {
      currentTurn: buildPackage.clientState.currentAction
        ? {
            evidence,
            summary: buildPackage.clientState.currentAction,
            title: buildPackage.clientState.currentAction,
            truth,
          }
        : undefined,
      evidence,
      latestActivity: buildPackage.updatedAt,
      streamStatus,
      truth,
      unavailable:
        streamStatus === 'unsupported' || streamStatus === 'unavailable'
          ? {
              detail:
                unsupportedNotice?.message ??
                'The draft exists, but no supported runtime conversation route exists yet.',
              evidence,
              kind: streamStatus,
              title: unsupportedNotice?.title ?? 'Runtime route unavailable',
              truth,
            }
          : undefined,
    },
    title: buildPackage.title,
    turns: buildPackage.messages.map((message) => {
      const messageTruth = message.truth ?? localDraftTruthState;
      return {
        body: message.body,
        createdAt: message.createdAt,
        evidence: toAgentConversationEvidence(messageTruth),
        id: message.id,
        isCurrent: message.state === 'processing',
        role: toAgentConversationTurnRole(message),
        title: message.role === 'system' ? 'System' : undefined,
        truth: messageTruth,
      };
    }),
    externalThreadId,
  };
}

function stageRunArtifacts(buildPackage: OrchestrationBuildPackage): AgentConversation['artifacts'] {
  return (buildPackage.stageRuns ?? []).flatMap((stageRun) => {
    const artifacts: AgentConversation['artifacts'] = [];
    const detail = stageRun.statusReason;

    if (stageRun.promptArtifactId) {
      artifacts.push({
        detail,
        evidence: toAgentConversationEvidence(stageRun.state),
        id: stageRun.promptArtifactId,
        kind: 'backend_evidence',
        name: 'Submitted Plan Builder prompt',
        truth: stageRun.state,
      });
    }

    if (stageRun.rawEventArtifactId) {
      artifacts.push({
        detail,
        evidence: toAgentConversationEvidence(stageRun.state),
        id: stageRun.rawEventArtifactId,
        kind: 'runtime_evidence',
        name: 'Raw Codex JSONL',
        truth: stageRun.state,
      });
    }

    if (stageRun.outputArtifactId) {
      artifacts.push({
        detail,
        evidence: toAgentConversationEvidence(stageRun.state),
        id: stageRun.outputArtifactId,
        kind: 'backend_evidence',
        name: 'Final Plan Builder response',
        truth: stageRun.state,
      });
    }

    return artifacts;
  });
}

function generatedFileArtifactKind(
  state: OrchestrationTruthState,
): AgentConversation['artifacts'][number]['kind'] {
  if (state.provenance === 'runtime_event') {
    return 'runtime_evidence';
  }

  if (state.provenance === 'backend_response') {
    return 'backend_evidence';
  }

  return 'draft';
}

function toAgentConversationStreamStatus(
  truth: OrchestrationTruthState,
): AgentConversationStreamStatus {
  if (truth.status === 'draft') {
    return 'idle';
  }

  if (truth.status === 'ready') {
    return 'input_ready';
  }

  if (truth.status === 'starting') {
    return 'starting';
  }

  if (truth.status === 'waiting_for_event') {
    return 'waiting_for_event';
  }

  if (truth.status === 'running') {
    return 'running';
  }

  if (truth.status === 'completed') {
    return 'completed';
  }

  if (truth.status === 'failed') {
    return 'failed';
  }

  if (truth.provenance === 'unsupported' || truth.status === 'integration_pending') {
    return 'unsupported';
  }

  return 'unavailable';
}

function toAgentConversationTurnRole(
  message: OrchestrationConversationMessage,
): AgentConversationTurnRole {
  if (message.truth?.provenance === 'runtime_event') {
    return 'runtime';
  }

  return message.role;
}

function toAgentConversationEvidence(
  truth: OrchestrationTruthState,
): AgentConversationEvidence {
  return {
    kind: toAgentConversationEvidenceKind(truth),
    label: toAgentConversationEvidenceLabel(truth),
    truth,
  };
}

function toAgentConversationEvidenceKind(
  truth: OrchestrationTruthState,
): AgentConversationEvidenceKind {
  if (truth.provenance === 'user_input') {
    return 'user_input';
  }

  if (truth.provenance === 'local_draft') {
    return 'local_optimistic_ui';
  }

  if (truth.provenance === 'backend_response') {
    if (truth.status === 'waiting_for_event') {
      return 'backend_acknowledgement';
    }

    if (truth.status === 'completed') {
      return 'completed_runtime_output';
    }

    if (truth.status === 'failed') {
      return 'failed_runtime_output';
    }

    return 'backend_acknowledgement';
  }

  if (truth.provenance === 'runtime_event') {
    if (truth.status === 'completed') {
      return 'completed_runtime_output';
    }

    if (truth.status === 'failed') {
      return 'failed_runtime_output';
    }

    return truth.status === 'running' ? 'streamed_runtime_output' : 'first_runtime_event';
  }

  if (truth.provenance === 'unsupported') {
    return 'unsupported_integration';
  }

  if (truth.provenance === 'persisted_snapshot') {
    return 'persisted_snapshot';
  }

  return 'mock_demo_fixture';
}

function toAgentConversationEvidenceLabel(truth: OrchestrationTruthState): string {
  if (truth.provenance === 'runtime_event') {
    return 'Runtime event evidence';
  }

  if (truth.provenance === 'backend_response') {
    return 'Backend response evidence';
  }

  if (truth.provenance === 'user_input') {
    return 'User input held locally';
  }

  if (truth.provenance === 'local_draft') {
    return truth.status === 'starting' ? 'Local optimistic UI state' : 'Local draft snapshot';
  }

  if (truth.provenance === 'persisted_snapshot') {
    return 'Persisted snapshot';
  }

  if (truth.provenance === 'unsupported') {
    return 'Unsupported integration';
  }

  return 'Mock/demo fixture';
}
