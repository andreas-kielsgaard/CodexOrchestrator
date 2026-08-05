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
    readonly attemptHistory: readonly ProductWorkUnitAttemptHistoryV1[];
    readonly retryAttempts: readonly ProductWorkUnitRetryAttemptV1[];
    readonly integration?: ProductWorkUnitIntegrationV1;
    readonly executionState?: ProductWorkUnitExecutionStateV1;
    readonly dependencyActivationIntent?: {
      readonly eligibilityState: 'blocked' | 'eligible';
      readonly blockedReason?: string;
      readonly eligibilityRecordedAt: string;
      readonly activationIntendedAt?: string;
    };
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
    readonly execution?: Readonly<{
      readonly graphCompletion?: Readonly<{ readonly completedAt: string }>;
      readonly settlement?: Readonly<{ readonly settledAt: string }>;
      readonly planningPointSettlement?: Readonly<{ readonly settledAt: string }>;
      readonly attention?: Readonly<{ readonly recordedAt: string }>;
    }>;
  }[];
  readonly sprintContinuation?: Readonly<{
    readonly decisions: readonly {
      readonly decisionId: string;
      readonly sprintId: string;
      readonly decisionSequence: number;
      readonly state: 'continuing' | 'attention' | 'settled';
      readonly reason: string;
      readonly acceptedMaterializationCount: number;
      readonly recordedAt: string;
      readonly attention?: Readonly<{
        readonly attentionId: string;
        readonly code: string;
        readonly structuredAttention?: Readonly<{
          readonly reason: string;
          readonly authorityNeeded: string;
          readonly evidenceContext: string;
          readonly resumptionPath: string;
        }>;
      }>;
    }[];
    readonly currentDecisions: readonly {
      readonly sprintId: string;
      readonly decisionId: string;
      readonly state: 'continuing' | 'attention' | 'settled';
      readonly updatedAt: string;
    }[];
    readonly upwardResults: readonly {
      readonly resultId: string;
      readonly decisionId: string;
      readonly sprintId: string;
      readonly resultKind: 'continuing' | 'attention' | 'settled';
      readonly recordedAt: string;
    }[];
  }>;
  readonly sprintResultProjections?: readonly ProductSprintResultProjectionV1[];
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
    | 'failed'
    | 'requested'
    | 'authorized'
    | 'invocation_prepared'
    | 'harness_bound'
    | 'launch_requested'
    | 'launch_accepted'
    | 'action_ready';
  readonly blockedReason?: string;
  readonly failureReason?: string;
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

export type ProductWorkUnitHandlerReviewV1 = Readonly<{
  readonly attemptId: string;
  readonly reportingInvocationId: string;
  readonly handlerSessionId: string;
  readonly originalHandlerInvocationId: string;
  readonly actionHandlerInvocationId: string;
  readonly reviewInvocationId: string;
  readonly reviewHarnessRevisionId: string;
  readonly deliveryRequestedAt: string;
  readonly deliveryPersistedAt?: string;
  readonly harnessBoundAt?: string;
  readonly launchRequestedAt?: string;
  readonly launchAcceptedAt?: string;
  readonly reviewReadyAt?: string;
  readonly delivered: Readonly<{
    readonly summaryClaim: string;
    readonly validationStatementClaim: string;
    readonly changedFiles: readonly Readonly<{
      readonly evidenceRef: string;
      readonly displayName: string;
      readonly changeKind: 'added' | 'modified' | 'deleted' | 'renamed';
      readonly contentFingerprint: string;
    }>[];
    readonly comparisonFingerprint: string;
    readonly deliveredPayloadFingerprint: string;
  }>;
  readonly semanticJudgment?: Readonly<{
    readonly variant: 'accept' | 'return';
    readonly reason?: Readonly<{ readonly code: string; readonly explanation: string }>;
    readonly fingerprint: string;
    readonly recordedAt: string;
  }>;
  readonly lifecycle?: Readonly<{
    readonly status: 'completed' | 'failed' | 'canceled' | 'interrupted';
    readonly observedAt: string;
  }>;
  readonly conflict?: Readonly<{ readonly occurredAt: string; readonly reason: string }>;
}>;

export type ProductWorkUnitHandlerDecisionV1 = Readonly<{
  readonly attemptId: string;
  readonly reviewInvocationId: string;
  readonly variant: 'accepted' | 'returned';
  readonly fingerprint: string;
  readonly returnReason?: Readonly<{ readonly code: string; readonly explanation: string }>;
  readonly recordedAt: string;
  readonly implementationAcceptedAt?: string;
  readonly implementationReturnedAt?: string;
  readonly retryRequiredAt?: string;
  readonly settlementReadyAt?: string;
}>;

export type ProductWorkUnitIntegrationV1 = Readonly<{
  readonly requestedAt: string;
  readonly authorizedAt: string;
  readonly progress?: Readonly<{
    readonly phase: 'preparing' | 'applying' | 'recording';
    readonly recordedAt: string;
  }>;
  readonly attention?: Readonly<{
    readonly kind: 'conflict' | 'failure';
    readonly safeCode: 'integration_conflict' | 'integration_failure';
    readonly recordedAt: string;
  }>;
  readonly success?: Readonly<{ readonly recordedAt: string }>;
  readonly settlement?: Readonly<{ readonly settledAt: string }>;
  readonly prerequisiteContribution?: Readonly<{
    readonly recordedAt: string;
    readonly dependentCount: number;
  }>;
}>;
export type ProductWorkUnitExecutionStateV1 = Readonly<{
  readonly state:
    | 'waiting_on_prerequisites'
    | 'ready'
    | 'active'
    | 'retry_authorized'
    | 'handed_back'
    | 'settled'
    | 'attention';
  readonly recordedAt: string;
}>;

export type ProductWorkUnitIncompleteDispositionV1 = Readonly<{
  readonly attemptId: string;
  readonly reviewInvocationId: string;
  readonly decisionFingerprint: string;
  readonly classification: 'refinement_needed' | 'functional_objective_not_satisfied' | 'blocked';
  readonly meaningfulProgress: boolean;
  readonly recordedAt: string;
  readonly nextAttemptAuthorizedAt?: string;
  readonly noProgressHandback?: Readonly<{
    readonly handbackId: string;
    readonly sourceAttemptId: string;
    readonly sourceReviewInvocationId: string;
    readonly contextFingerprint: string;
    readonly persistedAt: string;
    readonly deliveryIntendedAt: string;
    readonly sprintRunnerReceiverActivatedAt?: string;
    readonly sprintRunnerReceiverDecisionAt?: string;
    readonly sprintRunnerDelivery?: ProductSprintRunnerHandbackDeliveryV1;
    readonly epicRunnerReceiver?: ProductEpicEscalationReceiverV1;
  }>;
}>;

export type ProductSprintRunnerHandbackDependencyOwnerClassificationV1 =
  'work_unit_handler' | 'work_unit_implementer' | 'work_slice_planner' | 'sprint_runner';

export type ProductSprintRunnerHandbackBoundedDetailV1 = Readonly<{
  readonly label: string;
  readonly value: string;
}>;

export type ProductSprintRunnerHandbackUnknownMovementKindV1 = string & {
  readonly __boundedUnknownHandbackMovementKind: unique symbol;
};

export type ProductSprintRunnerHandbackKnownMovementKindV1 =
  'continue_eligible_work' | 'wait_for_agent_dependency' | 'local_exhaustion_escalate';

export type ProductSprintRunnerHandbackMovementV1 = Readonly<
  | {
      readonly movementKind: 'continue_eligible_work';
      readonly rationale: string;
      readonly eligibleWorkSummary: string;
    }
  | {
      readonly movementKind: 'wait_for_agent_dependency';
      readonly rationale: string;
      readonly dependencyOwner: string;
      readonly dependencyOwnerClassification: ProductSprintRunnerHandbackDependencyOwnerClassificationV1;
      readonly enablingResult: string;
      readonly resumptionPath: string;
    }
  | {
      readonly movementKind: 'local_exhaustion_escalate';
      readonly rationale: string;
      readonly localExhaustionSummary: string;
    }
  | {
      readonly movementKind: ProductSprintRunnerHandbackUnknownMovementKindV1;
      readonly rationale: string;
      readonly boundedDetails?: readonly ProductSprintRunnerHandbackBoundedDetailV1[];
    }
>;

export type ProductSprintRunnerHandbackDeliveryV1 = Readonly<{
  readonly deliveryRequestedAt: string;
  readonly deliveryPersistedAt?: string;
  readonly harnessBoundAt?: string;
  readonly launchRequestedAt?: string;
  readonly launchAcceptedAt?: string;
  readonly providerActivationObservedAt?: string;
  readonly semanticReassessmentRecordedAt?: string;
  readonly selectedMovementKind?: string;
  readonly selectedMovement?: ProductSprintRunnerHandbackMovementV1;
  readonly escalationIntentRecordedAt?: string;
  readonly escalationDeliveryRequestedAt?: string;
}>;

export type ProductEpicEscalationDispositionV1 = Readonly<{
  readonly movementKind: string;
  readonly rationale: string;
  readonly consideredIntent?: string;
  readonly downstreamRequest?: Readonly<{
    readonly target: 'sprint_runner' | 'existing_agent_achievable_dependency';
    readonly dependency?: 'work_unit_handler';
    readonly request: string;
    readonly resumptionPath: string;
  }>;
  readonly humanExternalAttention?: Readonly<{
    readonly reason: string;
    readonly authorityNeeded: string;
    readonly evidenceContext: string;
    readonly resumptionPath: string;
  }>;
}>;

export type ProductEpicEscalationReceiverV1 = Readonly<{
  readonly sprintId: string;
  readonly epicId: string;
  readonly deliveryRequestedAt: string;
  readonly deliveryPersistedAt?: string;
  readonly harnessBoundAt?: string;
  readonly launchRequestedAt?: string;
  readonly launchAcceptedAt?: string;
  readonly providerActivationObservedAt?: string;
  readonly reassessmentLifecycleStatus?: string;
  readonly reassessmentLifecycleObservedAt?: string;
  readonly semanticReassessmentRecordedAt?: string;
  readonly disposition?: ProductEpicEscalationDispositionV1;
}>;

export type ProductSprintResultProjectionV1 = Readonly<{
  readonly resultId: string;
  readonly decisionId: string;
  readonly sprintId: string;
  readonly epicId: string;
  readonly resultKind: 'continuing' | 'attention' | 'settled';
  readonly recordedAt: string;
  readonly receiver?: Readonly<{
    readonly deliveryRequestedAt: string;
    readonly deliveryPersistedAt?: string;
    readonly harnessBoundAt?: string;
    readonly launchRequestedAt?: string;
    readonly launchAcceptedAt?: string;
    readonly providerActivationObservedAt?: string;
    readonly reassessmentLifecycleStatus?: string;
    readonly reassessmentLifecycleObservedAt?: string;
    readonly semanticReassessmentRecordedAt?: string;
  }>;
  readonly dispositionRecordedAt?: string;
  readonly disposition?: ProductEpicEscalationDispositionV1;
  readonly realization?: Readonly<{
    readonly outcomeKind: 'successor_request' | 'terminal_readiness' | 'retained_attention';
    readonly consideredAt: string;
    readonly successorSprintId?: string;
    readonly successorTransition?: Readonly<{
      readonly requestedAt: string;
      readonly authorizedAt: string;
      readonly sessionCreatedAt?: string;
      readonly harnessAppliedAt?: string;
      readonly launchAcceptedAt?: string;
      readonly preStartReady: boolean;
      readonly lifecycleObserved: boolean;
      readonly accepted: boolean;
      readonly preStartSemanticOutcomeRecordedAt?: string;
      readonly preStartLifecycleObservedAt?: string;
      readonly preStartOutcomeAcceptedAt?: string;
      readonly parentContinuationDeliveryRequestedAt?: string;
      readonly parentContinuationDeliveryPersistedAt?: string;
      readonly epicContinuationLaunchAcceptedAt?: string;
      readonly providerReceiverActivationObservedAt?: string;
      readonly sprintStartAuthorizedAt?: string;
      readonly sprintStartPersistedAt?: string;
      readonly sprintContinuationLaunchAcceptedAt?: string;
      readonly repositoryBranchReevaluationRecordedAt?: string;
      readonly startedReevaluationLifecycleObservedAt?: string;
    }>;
    readonly successorRequestRecordedAt?: string;
    readonly terminalReadinessRecordedAt?: string;
    readonly retainedAttentionCode?: string;
    readonly retainedAttentionRecordedAt?: string;
  }>;
}>;

export type ProductWorkUnitRetryAttemptV1 = Readonly<{
  readonly ordinal: number;
  readonly originAttemptId: string;
  readonly retryAttemptId: string;
  readonly implementerSessionId: string;
  readonly implementerInvocationId: string;
  readonly captureRequestedAt: string;
  readonly candidatePinnedAt?: string;
  readonly authorizedAt?: string;
  readonly executionSupportGrantedAt?: string;
  readonly isolatedWorktreeReadyAt?: string;
  readonly implementerSessionCreatedAt?: string;
  readonly implementerInvocationPreparedAt?: string;
  readonly implementerHarnessBoundAt?: string;
  readonly launchRequestedAt?: string;
  readonly launchAcceptedAt?: string;
  readonly providerActivationObservedAt?: string;
  readonly retryReadyAt?: string;
  readonly failureReason?: string;
}>;

/** Application-recorded reporting facts; submitted prose remains explicitly claim-only. */
export type ProductWorkUnitImplementerOutcomeV1 = Readonly<{
  readonly attemptId: string;
  readonly implementerSessionId: string;
  readonly originalImplementerInvocationId: string;
  readonly reportingInvocationId: string;
  readonly reportingHarnessRevisionId: string;
  readonly reportingRequestedAt: string;
  readonly reportingPreparedAt?: string;
  readonly reportingHarnessBoundAt?: string;
  readonly reportingLaunchRequestedAt?: string;
  readonly reportingLaunchAcceptedAt?: string;
  readonly reportingReadyAt?: string;
  readonly submittedOutcome?: Readonly<{
    readonly variant: 'review_pending';
    readonly summaryClaim: string;
    readonly validationStatementClaim: string;
    readonly semanticPayloadFingerprint: string;
    readonly submittedAt: string;
    readonly validationAt: string;
    readonly validationResult: 'valid';
  }>;
  readonly evidence?: Readonly<{
    readonly changedFiles: readonly Readonly<{
      readonly evidenceRef: string;
      readonly displayName: string;
      readonly changeKind: 'added' | 'modified' | 'deleted' | 'renamed';
      readonly contentFingerprint: string;
    }>[];
    readonly comparisonFingerprint: string;
    readonly readyAt: string;
  }>;
  readonly semanticCompletion?: Readonly<{
    readonly invocationId: string;
    readonly completedAt: string;
  }>;
  readonly terminalLifecycle?: Readonly<{
    readonly status: 'completed' | 'failed' | 'canceled' | 'interrupted';
    readonly observedAt: string;
  }>;
  readonly applicationAcceptedAt?: string;
  readonly handlerReviewReadyAt?: string;
  readonly failureReason?: string;
}>;

/** Ordered application-owned history; ordinals are nonnegative and do not authorize later work. */
export type ProductWorkUnitAttemptHistoryV1 = Readonly<{
  readonly ordinal: number;
  readonly attemptId: string;
  readonly implementerOutcome?: ProductWorkUnitImplementerOutcomeV1;
  readonly handlerReview?: ProductWorkUnitHandlerReviewV1;
  readonly handlerDecision?: ProductWorkUnitHandlerDecisionV1;
  readonly incompleteDisposition?: ProductWorkUnitIncompleteDispositionV1;
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

export interface ProductSprintContinuationReadModelV1 {
  readonly current: Readonly<{
    readonly decisionId: string;
    readonly state: 'continuing' | 'attention' | 'settled';
    readonly updatedAt: string;
  }> | null;
  readonly history: readonly Readonly<{
    readonly decisionId: string;
    readonly sequence: number;
    readonly state: 'continuing' | 'attention' | 'settled';
    readonly reason: string;
    readonly recordedAt: string;
    readonly attention?: Readonly<{
      readonly code: string;
      readonly structuredAttention?: Readonly<{
        readonly reason: string;
        readonly authorityNeeded: string;
        readonly evidenceContext: string;
        readonly resumptionPath: string;
      }>;
    }>;
  }>[];
  /** Local persistence only; this does not imply delivery, receipt, or higher continuation. */
  readonly upwardResults: readonly Readonly<{
    readonly resultId: string;
    readonly decisionId: string;
    readonly recordedAt: string;
    readonly resultKind: 'continuing' | 'attention' | 'settled';
  }>[];
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
    readonly attemptHistory: readonly ProductWorkUnitAttemptHistoryV1[];
    readonly retryAttempts: readonly ProductWorkUnitRetryAttemptV1[];
    readonly integration?: ProductWorkUnitIntegrationV1;
    readonly executionState?: ProductWorkUnitExecutionStateV1;
    readonly dependencyActivationIntent?: {
      readonly eligibilityState: 'blocked' | 'eligible';
      readonly blockedReason?: string;
      readonly eligibilityRecordedAt: string;
      readonly activationIntendedAt?: string;
    };
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
    readonly execution?: Readonly<{
      readonly graphCompletion?: Readonly<{ readonly completedAt: string }>;
      readonly settlement?: Readonly<{ readonly settledAt: string }>;
      readonly planningPointSettlement?: Readonly<{ readonly settledAt: string }>;
      readonly attention?: Readonly<{ readonly recordedAt: string }>;
    }>;
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
  readonly epicEscalationReceivers?: readonly ProductEpicEscalationReceiverV1[];
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
  readonly sprintContinuation?: ProductSprintContinuationReadModelV1;
  readonly sprintResultProjections?: readonly ProductSprintResultProjectionV1[];
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
  readonly epicEscalationReceivers?: readonly ProductEpicEscalationReceiverV1[];
  readonly sprintResultProjections?: readonly ProductSprintResultProjectionV1[];
  readonly agentSessionReferences: readonly ProductAgentSessionReferenceReadModelV1[];
  readonly continuation: ProductContinuationReadModelV1;
  readonly bootstrapTransition?: import('./epicBootstrapTransition').ProductBootstrapTransitionStatusV2;
}

export interface ProductReadModelsV1 {
  readonly epics: readonly ProductEpicReadModelV1[];
  /** Extensible targets have no inferred owner and therefore remain explicitly unassociated. */
  readonly unassociatedAgentSessionReferences: readonly ProductAgentSessionReferenceReadModelV1[];
}
