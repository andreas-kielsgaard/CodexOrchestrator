/** Product-only Orchestration read contracts. These are presentation inputs, never persistence or transitions. */
import type {
  AgentSessionAssociationTargetKind,
  AgentSessionSemanticRole,
  OrchestrationEventsV1,
} from './orchestrationEvents';

export type ReadSourceAuthorityV1 =
  | {
      readonly status: 'available';
      readonly sourceKind:
        'orchestration_event' | 'application_interpretation' | 'user' | 'repository';
      readonly sourceReferences: readonly string[];
    }
  | { readonly status: 'pending' | 'unavailable' | 'unsupported'; readonly reason: string };

export type ProductEpicMovementV1 =
  | { readonly kind: 'preparing_next_sprint' }
  | { readonly kind: 'reviewing_sprint_completion' }
  | { readonly kind: 'planning_next_work' }
  | { readonly kind: 'initiating_work_units'; readonly count: number }
  | {
      readonly kind: 'executing_work';
      readonly processingCount: number;
      readonly reviewingCount: number;
    }
  | { readonly kind: 'reviewing_returned_work_units'; readonly count: number }
  | { readonly kind: 'integrating_accepted_work' }
  | { readonly kind: 'reevaluating_direction' };
export type ProductEpicStateV1 =
  'running' | 'ready_to_continue' | 'paused' | 'blocked' | 'completed';
export type ProductSourcedReadValueV1<T> =
  | {
      readonly source: Extract<ReadSourceAuthorityV1, { readonly status: 'available' }>;
      readonly value: T;
    }
  | {
      readonly source: Exclude<ReadSourceAuthorityV1, { readonly status: 'available' }>;
      readonly value?: never;
    };

export type ProductSprintPlanningStateV1 =
  | { readonly kind: 'pre_start_forecast' }
  | {
      readonly kind: 'started_plan';
      readonly currentWorkSlicePlanningPointId: string;
      readonly repositoryAssessmentSummary: string;
      readonly reevaluatedAt: string;
    };

export type ProductGatePresentationRoleV1 =
  | { readonly kind: 'accepted_review_marker' }
  | { readonly kind: 'other'; readonly fallbackLabel: string };

export interface ProductSprintWorkspaceNarrativesV1 {
  readonly direction?: ProductSourcedReadValueV1<string>;
  readonly progress?: ProductSourcedReadValueV1<string>;
  readonly attention?: ProductSourcedReadValueV1<string>;
}

export interface ProductSprintWorkspacePresentationMetadataV1 {
  readonly workSlicePlanningPointMembership: readonly {
    readonly workSlicePlanningPointId: string;
    readonly sprintPlanRevisionId: string;
    readonly workUnitScopeIds: readonly string[];
    readonly source: ReadSourceAuthorityV1;
  }[];
  readonly gates: readonly {
    readonly gateId: string;
    readonly role: ProductGatePresentationRoleV1;
    readonly source: ReadSourceAuthorityV1;
  }[];
  readonly documents: readonly {
    readonly documentRefId: string;
    readonly displayOrder: number;
    readonly recordedAt: ProductSourcedReadValueV1<string>;
    readonly displayCategory: ProductSourcedReadValueV1<string>;
    readonly sprintPlanRevisionIds: readonly string[];
    readonly workSlicePlanningPointIds: readonly string[];
    readonly workUnitScopeIds: readonly string[];
  }[];
  /** Epic Runner-authored Sprint objectives. Global Epic goals are not substituted here. */
  readonly epicRunnerObjectives?: readonly {
    readonly objectiveId: string;
    readonly sprintId: string;
    readonly title: string;
    readonly source: ReadSourceAuthorityV1;
  }[];
  /** Explicit Sprint Runner concern-to-graph links. Transcript prose is never parsed for these. */
  readonly sprintRunnerConcerns?: readonly {
    readonly sprintRunnerConcernId: string;
    readonly sprintId: string;
    readonly title: string;
    readonly source: ReadSourceAuthorityV1;
    readonly graphElementRefs: readonly {
      readonly kind: 'work_slice_planning_point' | 'work_unit' | 'gate';
      readonly id: string;
    }[];
  }[];
  /** Recorded navigation metadata only; runtime lifecycle support is not implied. */
  readonly workUnitLifecycle?: readonly {
    readonly entryId: string;
    readonly sprintId: string;
    readonly workUnitId: string;
    readonly sequence: number;
    readonly kind:
      | 'planning'
      | 'launch'
      | 'work'
      | 'review'
      | 'reprompt'
      | 'renewed_work'
      | 'merge'
      | 'completion';
    readonly title: string;
    readonly summary: string;
    readonly agentSessionId: string;
    readonly agentRole: AgentSessionSemanticRole;
    readonly invocationId: string;
    readonly source: ReadSourceAuthorityV1;
  }[];
  readonly narratives?: readonly (Readonly<{ readonly sprintId: string }> &
    ProductSprintWorkspaceNarrativesV1)[];
}

/**
 * The minimal display/reference index supplied beside durable facts. Every entry is keyed by an
 * existing product identity; available entries name their authority rather than implying it.
 */
export interface ProductReadReferenceIndexV1 {
  readonly epics: readonly {
    readonly epicId: string;
    readonly title: string;
    readonly goal: string;
    readonly source: ReadSourceAuthorityV1;
  }[];
  readonly epicOverviews: readonly {
    readonly epicId: string;
    readonly currentMovement: ProductSourcedReadValueV1<ProductEpicMovementV1>;
    readonly state: ProductSourcedReadValueV1<ProductEpicStateV1>;
  }[];
  readonly sprints: readonly {
    readonly sprintId: string;
    readonly title: string;
    readonly summary: string;
    readonly details: string;
    readonly source: ReadSourceAuthorityV1;
    readonly lifecycle?: ProductSourcedReadValueV1<'completed' | 'in_progress' | 'not_started'>;
    /** Typed planning authority. Missing production support projects as unavailable. */
    readonly planningState?: ProductSourcedReadValueV1<ProductSprintPlanningStateV1>;
  }[];
  readonly sprintPlanRevisions: readonly {
    readonly sprintPlanRevisionId: string;
    readonly summary: string;
    readonly source: ReadSourceAuthorityV1;
  }[];
  readonly workSlicePlanningPoints: readonly {
    readonly workSlicePlanningPointId: string;
    readonly title: string;
    readonly purpose: string;
    readonly source: ReadSourceAuthorityV1;
  }[];
  readonly workUnits: readonly {
    readonly workUnitId: string;
    readonly title: string;
    readonly summary: string;
    readonly details: string;
    readonly source: ReadSourceAuthorityV1;
    readonly handlerActivation?: ProductWorkUnitHandlerActivationV1;
    readonly actionContinuation?: ProductWorkUnitActionContinuationV1;
    readonly implementerActivation?: ProductWorkUnitImplementerActivationV1;
  }[];
  readonly gates: readonly {
    readonly gateId: string;
    readonly title: string;
    readonly summary: string;
    readonly source: ReadSourceAuthorityV1;
  }[];
  readonly concerns: readonly {
    readonly concernId: string;
    readonly sprintId: string;
    readonly title: string;
    readonly summary: string;
    readonly details: string;
    readonly requiredWorkUnitIds: readonly string[];
    readonly stateAuthority:
      | { readonly kind: 'derived_from_required_work_units' }
      | {
          readonly kind: 'explicit_decision';
          readonly decision: 'accepted' | 'deferred';
          readonly provenanceId: string;
        };
    readonly source: ReadSourceAuthorityV1;
  }[];
  readonly agentSessions: readonly {
    readonly agentSessionId: string;
    readonly title: string;
    readonly source: ReadSourceAuthorityV1;
  }[];
  /** Explicit product association; events alone intentionally do not infer artifact ownership. */
  readonly artifactOwnership: readonly {
    readonly artifactId: string;
    readonly sprintId: string;
    readonly source: ReadSourceAuthorityV1;
  }[];
  readonly documentOwnership: readonly {
    readonly documentRefId: string;
    readonly sprintId: string;
    readonly source: ReadSourceAuthorityV1;
  }[];
  /** Explicit display-only relationships; they do not change durable Event semantics. */
  readonly sprintWorkspacePresentation?: ProductSprintWorkspacePresentationMetadataV1;
}

/** Optional presentation selectors are validated against the ordered revision chain and are not facts. */
export interface ProductReadSelectionV1 {
  readonly selectedSprintPlanRevisionIds?: Readonly<Record<string, string>>;
}

export interface ProductReadCompositionInputV1 {
  readonly events: OrchestrationEventsV1;
  readonly agentControl: import('./agentControl').AgentControlContractsV1;
  readonly artifactAccess: import('./artifactAccess').ArtifactAccessContractsV1;
  readonly referenceIndex: ProductReadReferenceIndexV1;
  readonly workUnitMaterializations?: readonly {
    readonly materializationId: string;
    readonly planningPointId: string;
    readonly acceptedRevisionId: string;
    readonly sprintId: string;
    readonly stage:
      | 'authorized'
      | 'attempt_recorded'
      | 'work_units_created'
      | 'relationships_complete'
      | 'settled';
    readonly source: ReadSourceAuthorityV1;
  }[];
  readonly selection?: ProductReadSelectionV1;
  readonly bootstrapTransition?: Readonly<{
    readonly query: import('./epicBootstrapTransition').EpicBootstrapTransitionQueryV2;
    readonly initiationIdsByEpic: Readonly<Record<string, string>>;
  }>;
  readonly sprintRunnerTransition?: Readonly<{
    readonly query: import('./sprintRunnerTransition').SprintRunnerTransitionQueryV1;
  }>;
}

export type ProductWorkUnitPresentationState =
  | 'not_started'
  | 'waiting_for_dependencies'
  | 'requested'
  | 'launched'
  | 'returned'
  | 'under_review'
  | 'integrated'
  | 'responsibility_accepted'
  /** Recorded deferral remains explicit; it does not imply launch or progress. */
  | 'deferred';

/** Durable Handler activation state for an already materialized Work Unit. */
export type ProductWorkUnitHandlerActivationV1 =
  | Readonly<{
      readonly eligibilityState: 'blocked';
      readonly blockedReason: string;
    }>
  | Readonly<{
      readonly eligibilityState: 'eligible';
      readonly stage:
        | 'eligible_not_prepared'
        | 'invocation_prepared'
        | 'launch_requested'
        | 'launch_accepted'
        | 'handler_ready';
      /** Invocation-correlated observation, never provider compliance or lifecycle. */
      readonly providerActivityObserved: boolean;
    }>;

export type ProductWorkUnitActionContinuationV1 = Readonly<{
  readonly stage:
    | 'blocked'
    | 'requested'
    | 'authorized'
    | 'invocation_prepared'
    | 'harness_bound'
    | 'launch_requested'
    | 'launch_accepted'
    | 'action_ready';
  readonly blockedReason?: string;
  readonly providerActivityObserved: boolean;
}>;

export type ProductWorkUnitImplementerActivationV1 = Readonly<{
  readonly stage:
    | 'requested'
    | 'authorized'
    | 'execution_support_granted'
    | 'worktree_ready'
    | 'session_created'
    | 'invocation_prepared'
    | 'harness_bound'
    | 'launch_requested'
    | 'launch_accepted'
    | 'implementer_ready'
    | 'failed';
  readonly failureReason?: string;
  readonly providerActivityObserved: boolean;
}>;

export interface ProductContinuationReadModelV1 {
  readonly level: 'sprint' | 'epic';
  readonly policy: Readonly<{
    readonly policyId: string;
    readonly automaticEnabled: boolean;
  }> | null;
  readonly eligibility: Readonly<{
    readonly evaluationId: string;
    readonly status: 'eligible' | 'ineligible' | 'feedback_required';
    readonly feedbackBoundary?:
      'auto_flow_off' | 'designed_feedback_flow' | 'all_pending_work_blocked';
  }> | null;
  readonly commandResults: readonly {
    readonly commandId: string;
    readonly state: import('./agentControl').AgentControlResultV1['state'];
  }[];
  readonly eventEligibilityFacts: readonly {
    readonly policyEligibilityFactId: string;
    readonly automaticEnabled: boolean;
    readonly eligible: boolean;
  }[];
  readonly continuationRequests: readonly {
    readonly continuationRequestId: string;
    readonly targetKind: 'next_work_slice_planner' | 'next_sprint_runner';
  }[];
  readonly observedContinuationIds: readonly string[];
  /** True only from an observed continuation Event; policy, eligibility, and command results stay separate. */
  readonly initiationObserved: boolean;
}

export interface ProductAgentSessionReferenceReadModelV1 {
  readonly agentSessionRefId: string;
  readonly agentSessionId: string;
  readonly title: string;
  readonly source: ReadSourceAuthorityV1;
  readonly targetKind: AgentSessionAssociationTargetKind;
  readonly targetId: string;
  readonly semanticRole: AgentSessionSemanticRole;
  readonly otherTargetType?: string;
}

export interface ProductSprintRevisionViewV1 {
  readonly sprintPlanRevisionId: string;
  readonly revision: number;
  readonly summary: string;
  readonly source: ReadSourceAuthorityV1;
  readonly supersedesSprintPlanRevisionId?: string;
  readonly isCurrent: boolean;
  readonly isSelected: boolean;
  readonly workUnitScopes: readonly {
    readonly workUnitScopeId: string;
    readonly workUnitId: string;
    readonly dependsOnWorkUnitScopeIds: readonly string[];
    readonly gateIds: readonly string[];
  }[];
  readonly workSlicePlanningPointGroups: readonly {
    readonly workSlicePlanningPointId: string;
    readonly title: string;
    readonly purpose: string;
    readonly source: ReadSourceAuthorityV1;
    readonly membershipSource: ReadSourceAuthorityV1;
    readonly workUnitScopeIds: readonly string[];
  }[];
  readonly workUnits: readonly {
    readonly workUnitId: string;
    readonly title: string;
    readonly summary: string;
    readonly details: string;
    readonly source: ReadSourceAuthorityV1;
    readonly handlerActivation?: ProductWorkUnitHandlerActivationV1;
    readonly actionContinuation?: ProductWorkUnitActionContinuationV1;
    readonly implementerActivation?: ProductWorkUnitImplementerActivationV1;
    readonly workUnitScopeId: string;
    readonly sprintPlanRevisionId: string;
    readonly fixedExecutionScopeIds: readonly string[];
    readonly dependencies: readonly {
      readonly workUnitScopeId: string;
      readonly workUnitId: string;
    }[];
    readonly gateIds: readonly string[];
    readonly attempts: readonly {
      readonly attemptId: string;
      readonly workUnitExecutionId: string;
      readonly returned: boolean;
    }[];
    readonly reviews: readonly {
      readonly reviewId: string;
      readonly outcome?: 'accepted' | 'needs_correction' | 'blocked';
      readonly attemptId?: string;
    }[];
    readonly observed: Readonly<{
      readonly executionRequested: boolean;
      readonly launched: boolean;
      readonly returned: boolean;
      readonly integrated: boolean;
      readonly responsibilityAccepted: boolean;
    }>;
    readonly presentationState: ProductWorkUnitPresentationState;
  }[];
  readonly gates: readonly {
    readonly gateId: string;
    readonly title: string;
    readonly summary: string;
    readonly source: ReadSourceAuthorityV1;
    readonly criteriaRevisionIds: readonly string[];
    readonly feedback: readonly { readonly feedbackRecordId: string; readonly boundary: string }[];
    readonly presentationRole: ProductGatePresentationRoleV1;
    readonly presentationSource: ReadSourceAuthorityV1;
  }[];
  readonly reviews: readonly {
    readonly reviewId: string;
    readonly outcome?: 'accepted' | 'needs_correction' | 'blocked';
    readonly rationaleArtifactId?: string;
  }[];
}

export interface ProductSprintReadModelV1 {
  readonly sprintId: string;
  readonly epicId: string;
  readonly title: string;
  readonly summary: string;
  readonly details: string;
  readonly source: ReadSourceAuthorityV1;
  readonly lifecycle?: ProductSourcedReadValueV1<'completed' | 'in_progress' | 'not_started'>;
  readonly planningState: ProductSourcedReadValueV1<ProductSprintPlanningStateV1>;
  readonly sprintRunnerTransition?: import('./sprintRunnerTransition').ProductSprintRunnerTransitionStatusV1;
  /** Durable materialization progress; this remains separate from execution or Handler activation. */
  readonly workUnitMaterializations?: readonly {
    readonly materializationId: string;
    readonly planningPointId: string;
    readonly acceptedRevisionId: string;
    readonly stage:
      | 'authorized'
      | 'attempt_recorded'
      | 'work_units_created'
      | 'relationships_complete'
      | 'settled';
    readonly source: ReadSourceAuthorityV1;
  }[];
  readonly sprintPlan: {
    readonly sprintPlanId: string;
    readonly currentSprintPlanRevisionId: string;
    readonly selectedSprintPlanRevisionId: string;
    readonly revisions: readonly {
      readonly sprintPlanRevisionId: string;
      readonly revision: number;
      readonly summary: string;
      readonly source: ReadSourceAuthorityV1;
      readonly supersedesSprintPlanRevisionId?: string;
      readonly isCurrent: boolean;
      readonly isSelected: boolean;
      readonly workUnitScopes: readonly {
        readonly workUnitScopeId: string;
        readonly workUnitId: string;
        readonly dependsOnWorkUnitScopeIds: readonly string[];
        readonly gateIds: readonly string[];
      }[];
    }[];
  };
  readonly workSlicePlanningPoints: readonly {
    readonly workSlicePlanningPointId: string;
    readonly title: string;
    readonly purpose: string;
    readonly source: ReadSourceAuthorityV1;
    readonly assessedSprintPlanRevisionIds: readonly string[];
  }[];
  readonly revisionViews: readonly ProductSprintRevisionViewV1[];
  readonly concerns: readonly {
    readonly concernId: string;
    readonly title: string;
    readonly summary: string;
    readonly details: string;
    readonly requiredWorkUnitIds: readonly string[];
    readonly state:
      | 'not_started'
      | 'waiting_for_dependencies'
      | 'requested'
      | 'launched'
      | 'returned'
      | 'under_review'
      | 'integrated'
      | 'responsibility_accepted'
      | 'deferred';
    readonly stateAuthority: ProductReadReferenceIndexV1['concerns'][number]['stateAuthority'];
    readonly source: ReadSourceAuthorityV1;
  }[];
  readonly reviews: readonly {
    readonly reviewId: string;
    readonly subjectKind: 'sprint_plan_revision' | 'document_reference';
    readonly subjectId: string;
    readonly outcome?: 'accepted' | 'needs_correction' | 'blocked';
    readonly rationaleArtifactId?: string;
  }[];
  readonly documents: readonly {
    readonly documentRefId: string;
    readonly title: string;
    readonly summary?: string;
    readonly classification: string;
    readonly artifactIds: readonly string[];
    readonly changedFileReferenceIds: readonly string[];
    readonly provenanceReference: string;
    readonly ownershipSource: ReadSourceAuthorityV1;
  }[];
  readonly internalArtifacts: readonly {
    readonly artifactId: string;
    readonly kind: string;
    readonly provenanceReference: string;
    readonly ownershipSource: ReadSourceAuthorityV1;
  }[];
  readonly workspacePresentation: Readonly<{
    readonly workSlicePlanningPointMembership: ProductSprintWorkspacePresentationMetadataV1['workSlicePlanningPointMembership'];
    readonly gates: ProductSprintWorkspacePresentationMetadataV1['gates'];
    readonly documents: ProductSprintWorkspacePresentationMetadataV1['documents'];
    readonly epicRunnerObjectives?: ProductSprintWorkspacePresentationMetadataV1['epicRunnerObjectives'];
    readonly sprintRunnerConcerns?: ProductSprintWorkspacePresentationMetadataV1['sprintRunnerConcerns'];
    readonly workUnitLifecycle?: ProductSprintWorkspacePresentationMetadataV1['workUnitLifecycle'];
    readonly narratives?: ProductSprintWorkspaceNarrativesV1;
  }>;
  readonly agentSessionReferences: readonly ProductAgentSessionReferenceReadModelV1[];
  readonly continuation: ProductContinuationReadModelV1;
}

export interface ProductEpicReadModelV1 {
  readonly epicId: string;
  readonly title: string;
  readonly goal: string;
  readonly source: ReadSourceAuthorityV1;
  readonly overview: {
    readonly currentMovement: ProductSourcedReadValueV1<ProductEpicMovementV1>;
    readonly state: ProductSourcedReadValueV1<ProductEpicStateV1>;
  };
  readonly sprints: readonly ProductSprintReadModelV1[];
  readonly agentSessionReferences: readonly ProductAgentSessionReferenceReadModelV1[];
  readonly continuation: ProductContinuationReadModelV1;
  readonly bootstrapTransition?: import('./epicBootstrapTransition').ProductBootstrapTransitionStatusV2;
}

export interface ProductReadModelsV1 {
  readonly epics: readonly ProductEpicReadModelV1[];
  /** Extensible targets have no inferred owner and therefore remain explicitly unassociated. */
  readonly unassociatedAgentSessionReferences: readonly ProductAgentSessionReferenceReadModelV1[];
}
