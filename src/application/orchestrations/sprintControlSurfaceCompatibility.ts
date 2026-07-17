import type {
  ConcernState,
  DependencyKind,
  SprintReadModel,
  SprintRelationshipGraph,
  WorkUnitExecutionState,
  WorkUnitPresentationState,
} from './sprintReadModels';

/** Provisional discovery input. This is not durable product schema. */
export const SPRINT_PLANNER_OUTPUT_V1 = 'sprint-planner-output/v1' as const;
/** Provisional recorded execution input. This is not provider authority. */
export const SPRINT_EXECUTION_SNAPSHOT_V1 = 'sprint-execution-snapshot/v1' as const;

export interface SprintPlannerOutputV1 {
  readonly version: typeof SPRINT_PLANNER_OUTPUT_V1;
  readonly epicId: string;
  readonly sprint: {
    readonly id: string;
    readonly title: string;
    readonly summary: string;
    readonly details: string;
  };
  readonly sprintPlan: { readonly id: string; readonly sprintId: string };
  readonly concerns: readonly {
    readonly id: string;
    readonly title: string;
    readonly summary: string;
    readonly details: string;
    readonly requiredWorkUnitIds: readonly string[];
  }[];
  readonly planRevisions: readonly {
    readonly id: string;
    readonly revision: number;
    readonly summary: string;
    readonly supersedesPlanRevisionId?: string;
    readonly workUnitIds: readonly string[];
  }[];
  readonly sprintPlannerActivities: readonly {
    readonly id: string;
    readonly title: string;
    readonly purpose: string;
    readonly planRevisionId: string;
    readonly workUnitIds: readonly string[];
    readonly userReviewGateIds: readonly string[];
  }[];
  readonly planChanges: readonly {
    readonly id: string;
    readonly source: 'sprint_conversation';
    readonly summary: string;
    readonly priorPlanRevisionId: string;
    readonly resultingPlanRevisionId: string;
    readonly priorSprintPlannerActivityId: string;
    readonly resultingSprintPlannerActivityId: string;
  }[];
  readonly parallelGroups: readonly {
    readonly id: string;
    readonly rationale: string;
    readonly planRevisionId: string;
    readonly workUnitIds: readonly string[];
  }[];
  readonly workUnits: readonly {
    readonly id: string;
    readonly shortTitle: string;
    readonly summary: string;
    readonly details: string;
    readonly concernIds: readonly string[];
    readonly parallelGroupId?: string;
    readonly dependencies: readonly {
      readonly workUnitId?: string;
      readonly kind: DependencyKind;
      readonly gateId?: string;
    }[];
    readonly specRevisions: readonly {
      readonly id: string;
      readonly revision: number;
      readonly planRevisionId: string;
      readonly summary: string;
      readonly details: string;
    }[];
  }[];
  readonly gates: readonly {
    readonly id: string;
    readonly kind: 'user' | 'planner' | 'replan' | 'convergence';
    readonly specRevisions: readonly {
      readonly id: string;
      readonly revision: number;
      readonly planRevisionId: string;
      readonly summary: string;
      readonly requiresWorkUnitIds: readonly string[];
      readonly requiresGateIds: readonly string[];
    }[];
  }[];
  readonly documents: readonly {
    readonly id: string;
    readonly title: string;
    readonly kind: 'plan' | 'brief' | 'decision' | 'handoff';
    readonly sprintPlannerActivityId: string;
    readonly planRevisionId: string;
    readonly recordedAt: string;
  }[];
}

export interface SprintExecutionSnapshotV1 {
  readonly version: typeof SPRINT_EXECUTION_SNAPSHOT_V1;
  readonly sprintId: string;
  readonly activePlanRevisionId: string;
  readonly workUnits: readonly {
    readonly workUnitId: string;
    readonly state: WorkUnitExecutionState;
    readonly projectedAt: string;
    readonly actualLaunch?: { readonly launchedAt: string; readonly agentSessionId: string };
    readonly deferredByEventId?: string;
    readonly attempts: readonly {
      readonly id: string;
      readonly specRevisionId: string;
      readonly outcome: 'working' | 'returned' | 'accepted' | 'corrected' | 'blocked';
      readonly recordedAt: string;
      readonly workerFeedback?: string;
    }[];
  }[];
  readonly events: readonly {
    readonly id: string;
    readonly kind:
      'review' | 'correction' | 'acceptance' | 'replan' | 'blocker' | 'deferred_decision';
    readonly workUnitId?: string;
    readonly gateId?: string;
    readonly summary: string;
    readonly recordedAt: string;
  }[];
  readonly concernDecisions: readonly {
    readonly concernId: string;
    readonly kind: 'deferred' | 'accepted';
    readonly summary: string;
  }[];
  readonly generatedDocuments: readonly {
    readonly id: string;
    readonly title: string;
    readonly sourceDocumentId?: string;
    readonly workUnitId?: string;
    readonly kind: 'outcome' | 'review' | 'handoff';
    readonly recordedAt: string;
  }[];
  readonly agentSessions: readonly {
    readonly id: string;
    readonly title: string;
    readonly role: 'sprint' | 'work_unit_handler' | 'work_unit_worker';
    readonly workUnitId?: string;
  }[];
  readonly continuation: {
    readonly sprint: {
      readonly automaticEnabled: boolean;
      readonly status: 'not_ready' | 'ready_for_manual' | 'continuation_requested';
      readonly initiationObserved: boolean;
    };
    readonly epic: {
      readonly automaticEnabled: boolean;
      readonly status: 'not_ready' | 'ready_for_manual' | 'continuation_requested';
      readonly initiationObserved: boolean;
    };
  };
}

/** Recorded presentation compatibility. Product consumers should use `readModel`. */
export interface SprintControlSurfaceProjection {
  readonly sourceAuthority: 'recorded_compatibility';
  readonly readModel: SprintReadModel;
  readonly sprint: SprintPlannerOutputV1['sprint'];
  readonly activePlanRevisionId: string;
  readonly selectedPlanRevisionId: string;
  readonly revisionGraph: readonly {
    readonly id: string;
    readonly revision: number;
    readonly summary: string;
    readonly supersedesPlanRevisionId?: string;
    readonly workUnitIds: readonly string[];
    readonly isActive: boolean;
    readonly isSelected: boolean;
  }[];
  readonly workUnits: readonly {
    readonly id: string;
    readonly shortTitle: string;
    readonly summary: string;
    readonly details: string;
    readonly concernIds: readonly string[];
    readonly specRevision: {
      readonly id: string;
      readonly revision: number;
      readonly summary: string;
      readonly details: string;
    };
    readonly executionState: WorkUnitExecutionState;
    readonly presentationState: WorkUnitPresentationState;
    readonly journey: {
      readonly specRevisions: readonly {
        readonly id: string;
        readonly revision: number;
        readonly planRevisionId: string;
        readonly summary: string;
      }[];
      readonly attemptDetails: SprintExecutionSnapshotV1['workUnits'][number]['attempts'];
      readonly events: SprintExecutionSnapshotV1['events'];
      readonly attempts: number;
      readonly hasWorkerFeedback: boolean;
      readonly accepted: boolean;
      readonly launched: boolean;
    };
    readonly dependencies: SprintPlannerOutputV1['workUnits'][number]['dependencies'];
    readonly parallelGroupId?: string;
  }[];
  readonly concerns: readonly {
    readonly id: string;
    readonly title: string;
    readonly summary: string;
    readonly details: string;
    readonly requiredWorkUnitIds: readonly string[];
    readonly state: ConcernState;
  }[];
  readonly documents: readonly {
    readonly id: string;
    readonly title: string;
    readonly kind: string;
    readonly recordedAt: string;
    readonly provenance: 'planner' | 'execution';
    readonly sprintPlannerActivityId?: string;
    readonly planRevisionId?: string;
    readonly workUnitId?: string;
    readonly sourceDocumentId?: string;
  }[];
  readonly mapLayout: SprintRelationshipGraph;
  readonly sprintPlannerActivities: SprintPlannerOutputV1['sprintPlannerActivities'];
  /** Recorded UI grouping keyed by Sprint Planner Activity identity. */
  readonly sprintPlannerActivityGroups: SprintPlannerOutputV1['sprintPlannerActivities'];
  readonly gates: readonly {
    readonly id: string;
    readonly kind: SprintPlannerOutputV1['gates'][number]['kind'];
    readonly summary: string;
  }[];
  readonly parallelGroups: SprintPlannerOutputV1['parallelGroups'];
  readonly planChanges: SprintPlannerOutputV1['planChanges'];
  readonly revisionViews: readonly {
    readonly planRevisionId: string;
    readonly workUnits: SprintControlSurfaceProjection['workUnits'];
    readonly sprintPlannerActivities: SprintControlSurfaceProjection['sprintPlannerActivities'];
    readonly sprintPlannerActivityGroups: SprintControlSurfaceProjection['sprintPlannerActivityGroups'];
    readonly gates: SprintControlSurfaceProjection['gates'];
    readonly parallelGroups: SprintControlSurfaceProjection['parallelGroups'];
    readonly planChanges: SprintControlSurfaceProjection['planChanges'];
    readonly mapLayout: SprintRelationshipGraph;
  }[];
  readonly agentSessions: SprintExecutionSnapshotV1['agentSessions'];
  readonly continuation: SprintExecutionSnapshotV1['continuation'];
}
