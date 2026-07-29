import type { ProjectedTranscript, TranscriptAnchorRange } from '../agentSessions';
import type {
  ProductEpicMovementItemV1,
  ProductEpicOverviewActionV1,
  ProductEpicStateV1,
  ProductOverviewNavigationTargetV1,
  SprintWorkspacePresentationV1,
} from '../../application/orchestrations';
import type { RecordedPlanWorkflowV1 } from '../../application/orchestrations/recordedPlanWorkflow';
import type { AutomaticContinuationPolicyUpdateIntent } from '../../application/orchestrations';

/** Feature presentation types projected from product-owned application reads. */
export type UnavailablePresentation = {
  readonly kind: 'pending' | 'unavailable' | 'unsupported';
  readonly reason: string;
};

export type EpicState = ProductEpicStateV1;
export type EpicMovementItem = ProductEpicMovementItemV1;
export type EpicOverviewAction = ProductEpicOverviewActionV1;
export type EpicOverviewNavigationTarget = ProductOverviewNavigationTargetV1;

export type EpicStatePresentation = EpicState | UnavailablePresentation;
export type EpicMovementPresentation =
  | { readonly kind: 'available'; readonly items: readonly EpicMovementItem[] }
  | UnavailablePresentation;
export type EpicReadyWorkPresentation = readonly EpicOverviewAction[] | UnavailablePresentation;
export type EpicHumanInputPresentation = EpicOverviewAction | null | UnavailablePresentation;
export type SprintPlanItemStatus =
  'completed' | 'in_progress' | 'not_started' | UnavailablePresentation;

export type SprintWorkspaceDetailLocation =
  | { readonly kind: 'sprint' }
  | {
      readonly kind: 'sprint_planner_activity_group';
      readonly revisionId: string;
      readonly sprintPlannerActivityId: string;
    }
  | {
      readonly kind: 'work_unit';
      readonly revisionId: string;
      readonly sprintPlannerActivityId: string;
      readonly workUnitId: string;
      readonly origin: 'sprint_planner_activity_group' | 'concern';
    };

export interface EpicBlocker {
  readonly id: string;
  readonly summary: string;
  readonly detail: string;
  readonly needs: string;
}

export interface AgentSessionReferencePresentation {
  readonly sessionId: string;
  readonly title: string;
}

export interface SprintAgentSessionPresentation extends AgentSessionReferencePresentation {
  readonly transcript?: ProjectedTranscript;
}

export interface WorkUnitAgentSessionPresentation extends SprintAgentSessionPresentation {
  readonly workUnitId: string;
  readonly role: 'sprint_planner' | 'handler' | 'worker' | 'reviewer';
}

/** Deferred controller/view adjuncts. They add no Sprint plan semantics. */
export interface SprintWorkspacePresentationAdjunct {
  readonly agentSession?: SprintAgentSessionPresentation;
  readonly plannerActivitySessions: readonly SprintAgentSessionPresentation[];
  readonly workUnitSessions: readonly WorkUnitAgentSessionPresentation[];
  readonly plannerActivityWorkflows: readonly RecordedPlanWorkflowV1[];
}

export interface SprintPlanItemPresentation {
  readonly id: string;
  readonly name: string;
  readonly purpose: string;
  readonly status: SprintPlanItemStatus;
  readonly blocker?: EpicBlocker;
  /** Present only after a Sprint starts. */
  readonly agentSession?: AgentSessionReferencePresentation;
  /** Product Sprint workspace semantics, projected at the application boundary. */
  readonly workspace?: SprintWorkspacePresentationV1;
  /** Narrow deferred integrations; never a source of revision/view semantics. */
  readonly workspaceAdjunct?: SprintWorkspacePresentationAdjunct;
  readonly detail?: {
    readonly summary: string;
    readonly outcome: string;
  };
}

export interface EpicPlanPresentation {
  readonly items: readonly SprintPlanItemPresentation[];
}

export interface EpicRunnerSessionPresentation {
  readonly sessionId: string;
  readonly title: string;
  readonly transcript: ProjectedTranscript;
  /** Structured excerpt pointer; no orchestration-specific transcript text is duplicated. */
  readonly latestAgentTurnRange: TranscriptAnchorRange;
}

export interface ContinuationPresentation {
  readonly automaticEnabled: boolean;
  readonly eligible: boolean;
  readonly status: 'not_ready' | 'ready_for_manual' | 'continuation_requested';
  readonly policyUpdateIntent?: Extract<
    AutomaticContinuationPolicyUpdateIntent,
    { readonly level: 'epic' }
  >;
}

export interface EpicPresentation {
  readonly id: string;
  readonly name: string;
  readonly goal: string;
  readonly movement: EpicMovementPresentation;
  readonly state: EpicStatePresentation;
  readonly readyWork: EpicReadyWorkPresentation;
  readonly humanInput: EpicHumanInputPresentation;
  readonly plan: EpicPlanPresentation;
  /** Supplied by the later embedded Agent Session controller when a product session is available. */
  readonly epicRunnerSession?: EpicRunnerSessionPresentation;
  readonly continuation?: ContinuationPresentation;
  readonly bootstrapTransition?: import('../../application/orchestrations').ProductBootstrapTransitionStatusV2;
}

export interface OrchestrationSectionView {
  readonly epics: readonly EpicPresentation[];
}

export function movementLabel(movement: EpicMovementPresentation): string {
  if (movement.kind !== 'available')
    return `${sourceStatusLabel(movement.kind)}: ${movement.reason}`;
  if (movement.items.length === 0) return 'No work in motion';
  const processing = movement.items.filter(({ state }) => state === 'processing').length;
  const reviewing = movement.items.length - processing;
  return `${processing} processing \u00b7 ${reviewing} reviewing`;
}

export function isUnavailablePresentation(
  value: EpicReadyWorkPresentation | EpicHumanInputPresentation,
): value is UnavailablePresentation {
  return value !== null && !Array.isArray(value) && 'kind' in value;
}

export function sourceStatusLabel(status: UnavailablePresentation['kind']): string {
  return { pending: 'Pending', unavailable: 'Unavailable', unsupported: 'Unsupported' }[status];
}
