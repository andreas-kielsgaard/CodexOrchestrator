import type { ProjectedTranscript, TranscriptAnchorRange } from '../agentSessions';
import type { SprintWorkspacePresentationV1 } from '../../application/orchestrations';
import type { RecordedPlanWorkflowV1 } from '../../application/orchestrations/recordedPlanWorkflow';
import type { AutomaticContinuationPolicyUpdateIntent } from '../../application/orchestrations';

/**
 * Disposable, fixture-driven presentation data for orientation discovery. These types describe
 * what the current UI needs; they are not persistence records, orchestration language, or a
 * transition engine contract.
 */
export type EpicMovement =
  | { readonly kind: 'preparing_next_sprint' }
  | { readonly kind: 'reviewing_sprint_completion' }
  | { readonly kind: 'planning_next_work' }
  | { readonly kind: 'starting_work_units'; readonly count: number }
  | {
      readonly kind: 'executing_work';
      readonly processingCount: number;
      readonly reviewingCount: number;
    }
  | { readonly kind: 'reviewing_returned_work'; readonly count: number }
  | { readonly kind: 'integrating_accepted_work' }
  | { readonly kind: 'reevaluating_direction' };

export type UnavailablePresentation = {
  readonly kind: 'pending' | 'unavailable' | 'unsupported';
  readonly reason: string;
};

export type EpicState = 'running' | 'ready_to_continue' | 'paused' | 'blocked' | 'completed';

export type EpicStatePresentation = EpicState | UnavailablePresentation;
export type EpicMovementPresentation = EpicMovement | UnavailablePresentation;
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
  readonly role: 'handler' | 'implementer';
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
  /** Present only after an Sprint starts. */
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
  if ('reason' in movement) return `${sourceStatusLabel(movement.kind)}: ${movement.reason}`;
  switch (movement.kind) {
    case 'preparing_next_sprint':
      return 'Preparing next Sprint';
    case 'reviewing_sprint_completion':
      return 'Reviewing Sprint completion';
    case 'planning_next_work':
      return 'Planning next work';
    case 'starting_work_units':
      return `Starting ${movement.count} Work Unit${movement.count === 1 ? '' : 's'}`;
    case 'executing_work':
      return `${movement.processingCount} processing · ${movement.reviewingCount} reviewing`;
    case 'reviewing_returned_work':
      return `Reviewing ${movement.count} returned Work Unit${movement.count === 1 ? '' : 's'}`;
    case 'integrating_accepted_work':
      return 'Integrating accepted work';
    case 'reevaluating_direction':
      return 'Reevaluating direction';
  }
}

export function sourceStatusLabel(status: UnavailablePresentation['kind']): string {
  return { pending: 'Pending', unavailable: 'Unavailable', unsupported: 'Unsupported' }[status];
}
