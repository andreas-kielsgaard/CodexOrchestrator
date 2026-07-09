import type { EntityId } from './model';
import {
  canShowRuntimeCompletion,
  canShowRuntimeProgress,
  getOrchestrationStatusDescription,
  getOrchestrationStatusLabel,
  type OrchestrationTruthState,
} from './orchestrationState';

export type AgentConversationMode = 'interactive' | 'read_only';

export type AgentConversationTurnRole = 'user' | 'assistant' | 'runtime' | 'system';

export type AgentConversationStreamStatus =
  | 'idle'
  | 'input_ready'
  | 'starting'
  | 'waiting_for_event'
  | 'running'
  | 'completed'
  | 'failed'
  | 'unsupported'
  | 'unavailable';

export type AgentConversationEvidenceKind =
  | 'user_input'
  | 'local_optimistic_ui'
  | 'backend_acknowledgement'
  | 'first_runtime_event'
  | 'streamed_runtime_output'
  | 'completed_runtime_output'
  | 'failed_runtime_output'
  | 'unsupported_integration'
  | 'persisted_snapshot'
  | 'mock_demo_fixture';

export interface AgentConversationEvidence {
  kind: AgentConversationEvidenceKind;
  label: string;
  truth: OrchestrationTruthState;
  recordedAt?: string;
  detail?: string;
}

export interface AgentConversationRuntimeIdentity {
  providerId: string;
  providerLabel: string;
  runtimeId?: string;
  runtimeLabel?: string;
  runtimeRoute?: string;
  unsupportedReason?: string;
}

export interface AgentConversationAttachment {
  id: EntityId;
  name: string;
  kind: 'uploaded' | 'local_draft' | 'backend_evidence' | 'runtime_evidence';
  truth: OrchestrationTruthState;
  detail?: string;
  evidence?: AgentConversationEvidence;
}

export interface AgentConversationArtifact {
  id: EntityId;
  name: string;
  kind: 'draft' | 'backend_evidence' | 'runtime_evidence';
  truth: OrchestrationTruthState;
  detail?: string;
  evidence?: AgentConversationEvidence;
}

export interface AgentConversationTurn {
  id: EntityId;
  role: AgentConversationTurnRole;
  body: string;
  truth: OrchestrationTruthState;
  evidence: AgentConversationEvidence;
  createdAt?: string;
  title?: string;
  isCurrent?: boolean;
  attachments?: AgentConversationAttachment[];
  artifacts?: AgentConversationArtifact[];
}

export interface AgentConversationCurrentTurn {
  turnId?: EntityId;
  title: string;
  summary: string;
  truth: OrchestrationTruthState;
  evidence: AgentConversationEvidence;
  startedAt?: string;
}

export interface AgentConversationInputState {
  enabled: boolean;
  placeholder?: string;
  disabledReason?: string;
}

export interface AgentConversationUnavailableState {
  kind: 'unsupported' | 'unavailable';
  title: string;
  detail: string;
  truth: OrchestrationTruthState;
  evidence: AgentConversationEvidence;
}

export interface AgentConversationState {
  streamStatus: AgentConversationStreamStatus;
  truth: OrchestrationTruthState;
  evidence: AgentConversationEvidence;
  latestActivity?: string;
  currentTurn?: AgentConversationCurrentTurn;
  unavailable?: AgentConversationUnavailableState;
}

export interface AgentConversation {
  id: EntityId;
  title: string;
  role: string;
  mode: AgentConversationMode;
  runtime: AgentConversationRuntimeIdentity;
  state: AgentConversationState;
  turns: AgentConversationTurn[];
  attachments: AgentConversationAttachment[];
  artifacts: AgentConversationArtifact[];
  externalThreadId?: string;
  input?: AgentConversationInputState;
}

export function canClaimAgentConversationRunning(conversation: AgentConversation): boolean {
  return (
    conversation.state.streamStatus === 'running' &&
    canShowRuntimeProgress(conversation.state.truth)
  );
}

export function canClaimAgentConversationCompleted(conversation: AgentConversation): boolean {
  return (
    conversation.state.streamStatus === 'completed' &&
    canShowRuntimeCompletion(conversation.state.truth)
  );
}

export function canClaimAgentConversationActiveWork(conversation: AgentConversation): boolean {
  return canClaimAgentConversationRunning(conversation);
}

export function getAgentConversationStatusLabel(conversation: AgentConversation): string {
  const { streamStatus, truth } = conversation.state;

  if (streamStatus === 'running') {
    return canClaimAgentConversationRunning(conversation)
      ? 'Running'
      : 'Runtime status unconfirmed';
  }

  if (streamStatus === 'completed') {
    return canClaimAgentConversationCompleted(conversation) ? 'Completed' : 'Completion unverified';
  }

  if (streamStatus === 'waiting_for_event') {
    return 'Waiting for first runtime event';
  }

  if (streamStatus === 'input_ready') {
    return 'Input ready';
  }

  if (streamStatus === 'unsupported') {
    return 'Unsupported';
  }

  if (streamStatus === 'unavailable') {
    return 'Unavailable';
  }

  if (streamStatus === 'idle') {
    return 'Not started';
  }

  return getOrchestrationStatusLabel(truth);
}

export function getAgentConversationStatusDescription(conversation: AgentConversation): string {
  const { streamStatus, truth } = conversation.state;

  if (streamStatus === 'running' && !canClaimAgentConversationRunning(conversation)) {
    return 'Running cannot be shown until backend or runtime evidence confirms active work.';
  }

  if (streamStatus === 'completed' && !canClaimAgentConversationCompleted(conversation)) {
    return 'Completion cannot be shown until backend or runtime evidence confirms the result.';
  }

  if (streamStatus === 'waiting_for_event') {
    return 'The backend has acknowledged the request, but no runtime event has arrived yet.';
  }

  if (streamStatus === 'input_ready') {
    return 'User input is ready locally; no backend command or runtime thread is implied.';
  }

  if (streamStatus === 'unsupported') {
    return conversation.state.unavailable?.detail ?? 'This runtime route is not supported yet.';
  }

  if (streamStatus === 'unavailable') {
    return conversation.state.unavailable?.detail ?? 'This runtime route is unavailable.';
  }

  if (streamStatus === 'idle') {
    return 'No prompt has been submitted and no runtime conversation exists yet.';
  }

  return getOrchestrationStatusDescription(truth);
}

export function getAgentConversationCurrentActionLabel(conversation: AgentConversation): string {
  if (conversation.state.currentTurn) {
    return conversation.state.currentTurn.title;
  }

  return getAgentConversationStatusLabel(conversation);
}

export function getAgentConversationLatestSummary(conversation: AgentConversation): string {
  const currentTurn = conversation.state.currentTurn;

  if (currentTurn) {
    return currentTurn.summary;
  }

  const latestTurn = conversation.turns.at(-1);

  if (latestTurn) {
    return latestTurn.body;
  }

  return getAgentConversationStatusDescription(conversation);
}

export function getAgentConversationEvidenceLabel(evidence: AgentConversationEvidence): string {
  return evidence.label;
}

