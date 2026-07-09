import type { OrchestrationTruthState } from '../../domain/orchestrationState';

export interface OrchestrationStageItem {
  description?: string;
  evidenceLabel?: string;
  id: string;
  isCurrent?: boolean;
  state: OrchestrationTruthState;
  title: string;
}

export type ConversationMessageRole = 'user' | 'assistant' | 'system' | 'runtime' | 'mock';

export interface ConversationMessageItem {
  author?: string;
  body: string;
  id: string;
  role: ConversationMessageRole;
  sourceLabel?: string;
  state?: OrchestrationTruthState;
  timestampLabel?: string;
}

export type OrchestrationFileKind = 'uploaded' | 'draft' | 'backend_evidence' | 'runtime_evidence';

export interface OrchestrationFileItem {
  detailLabel?: string;
  evidenceLabel?: string;
  id: string;
  kind: OrchestrationFileKind;
  name: string;
  state?: OrchestrationTruthState;
}

export interface ActivityTimelineItem {
  description?: string;
  id: string;
  sourceLabel: string;
  state: OrchestrationTruthState;
  timestampLabel?: string;
  title: string;
}
