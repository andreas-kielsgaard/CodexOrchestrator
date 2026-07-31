/** Provider-neutral application read models. They are neither persistence nor transition contracts. */
export type DependencyKind = 'hard' | 'preferred' | 'gated';
export type WorkUnitExecutionState =
  'projected' | 'launched' | 'working' | 'under_review' | 'accepted' | 'blocked' | 'deferred';
export type WorkUnitPresentationState =
  | 'not_started'
  | 'waiting_for_dependencies'
  | 'working'
  | 'under_review'
  | 'completed'
  | 'blocked'
  | 'deferred';
export type ConcernState = WorkUnitPresentationState;

export interface ContinuationReadModel {
  readonly automaticEnabled: boolean;
  readonly status: 'not_ready' | 'ready_for_manual' | 'continuation_requested';
  /** True only when initiation was observed, never from policy or eligibility alone. */
  readonly initiationObserved: boolean;
}

export interface SprintReadModel {
  readonly epic: { readonly epicId: string };
  readonly sprint: {
    readonly sprintId: string;
    readonly epicId: string;
    readonly title: string;
    readonly summary: string;
    readonly details: string;
  };
  readonly sprintPlan: { readonly sprintPlanId: string; readonly sprintId: string };
  readonly activeSprintPlanRevisionId: string;
  readonly selectedSprintPlanRevisionId: string;
  readonly sprintPlanRevisions: readonly {
    readonly sprintPlanRevisionId: string;
    readonly sprintPlanId: string;
    readonly revision: number;
    readonly summary: string;
    readonly supersedesSprintPlanRevisionId?: string;
    readonly workUnitIds: readonly string[];
    readonly isActive: boolean;
    readonly isSelected: boolean;
  }[];
  readonly workSlicePlanningPoints: readonly {
    readonly workSlicePlanningPointId: string;
    readonly sprintPlanRevisionId: string;
    readonly title: string;
    readonly purpose: string;
    readonly workUnitIds: readonly string[];
    readonly userReviewGateIds: readonly string[];
  }[];
  readonly workUnits: readonly {
    readonly workUnitId: string;
    readonly title: string;
    readonly summary: string;
    readonly details: string;
    readonly concernIds: readonly string[];
    readonly sprintPlanRevisionId: string;
    readonly selectedScopeDefinitionId: string;
    /** Absent when compatibility input cannot establish the fixed instantiated scope. */
    readonly fixedExecutionScopeId?: string;
    readonly executionState: WorkUnitExecutionState;
    readonly presentationState: WorkUnitPresentationState;
    readonly executionRequestObserved: boolean;
    readonly launchObserved: boolean;
    readonly responsibilityAccepted: boolean;
    readonly attempts: number;
    readonly dependencies: readonly {
      readonly workUnitId?: string;
      readonly kind: DependencyKind;
      readonly gateId?: string;
    }[];
  }[];
  readonly agentSessionReferences: readonly {
    readonly agentSessionRefId: string;
    readonly title: string;
    readonly role: 'sprint' | 'work_unit_handler' | 'work_unit_implementer';
    readonly workUnitId?: string;
  }[];
  readonly continuation: {
    readonly sprint: ContinuationReadModel;
    readonly epic: ContinuationReadModel;
  };
}

export interface SprintRelationshipGraph {
  readonly nodes: readonly {
    readonly id: string;
    readonly type:
      'sprint_plan' | 'plan_revision' | 'work_slice_planning_point' | 'work_unit' | 'gate';
    readonly semanticId: string;
    readonly parallelGroupId?: string;
  }[];
  readonly edges: readonly {
    readonly id: string;
    readonly from: string;
    readonly to: string;
    readonly kind: 'revision' | 'assessment' | 'plan' | 'dependency' | 'gate';
    readonly dependencyKind?: DependencyKind;
    readonly gateId?: string;
  }[];
}
