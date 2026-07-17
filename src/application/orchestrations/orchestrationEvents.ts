/**
 * Provider-neutral identities and durable Orchestration Events.
 *
 * This module deliberately contains neither presentation state nor transition behavior.
 * Read models may derive convenience states from these events, but must never write those
 * derivations back as events or infer observed work from a request.
 */
export const ORCHESTRATION_EVENTS_V1 = 'orchestration-events/v1' as const;

export type AgentSessionAssociationTargetKind =
  'epic' | 'sprint' | 'sprint_planner_activity' | 'work_unit_execution' | 'other';

export type AgentSessionSemanticRole =
  | 'epic_runner'
  | 'epic_plan_builder'
  | 'sprint_runner'
  | 'sprint_planner'
  | 'work_unit_planner'
  | 'work_unit_handler'
  | 'work_unit_worker'
  | 'reviewer'
  | 'other';

export interface OrchestrationEventsV1 {
  readonly version: typeof ORCHESTRATION_EVENTS_V1;
  readonly epics: readonly { readonly epicId: string }[];
  readonly sprints: readonly { readonly sprintId: string; readonly epicId: string }[];
  readonly sprintPlans: readonly { readonly sprintPlanId: string; readonly sprintId: string }[];
  readonly sprintPlanRevisions: readonly {
    readonly sprintPlanRevisionId: string;
    readonly sprintPlanId: string;
    readonly revision: number;
    readonly supersedesSprintPlanRevisionId?: string;
  }[];
  /** A Work Unit is the only durable planned responsibility. */
  readonly workUnits: readonly { readonly workUnitId: string }[];
  /** Revision-specific membership and ready-scope definition, separate from a Work Unit identity. */
  readonly workUnitScopes: readonly {
    readonly workUnitScopeId: string;
    readonly sprintPlanRevisionId: string;
    readonly workUnitId: string;
    readonly dependsOnWorkUnitScopeIds: readonly string[];
    readonly gateIds: readonly string[];
  }[];
  /** Planner activity is distinct from both the Sprint Plan and Agent Session. */
  readonly sprintPlannerActivities: readonly {
    readonly sprintPlannerActivityId: string;
    readonly sprintPlanId: string;
    readonly assessedSprintPlanRevisionIds: readonly string[];
  }[];
  /**
   * This execution record fixes scope for a requested or later observed planner/execution. Its
   * presence does not prove planner instantiation or launch; attempts only repeat the same scope.
   */
  readonly workUnitExecutions: readonly {
    readonly workUnitExecutionId: string;
    readonly workUnitId: string;
    readonly fixedWorkUnitScopeId: string;
  }[];
  readonly attempts: readonly {
    readonly attemptId: string;
    readonly workUnitExecutionId: string;
    readonly fixedWorkUnitScopeId: string;
  }[];
  readonly agentSessions: readonly { readonly agentSessionId: string }[];
  readonly agentSessionReferences: readonly {
    readonly agentSessionRefId: string;
    readonly agentSessionId: string;
    readonly targetKind: AgentSessionAssociationTargetKind;
    readonly targetId: string;
    readonly semanticRole: AgentSessionSemanticRole;
    /** Required only when the association target is not yet a first-class product entity. */
    readonly otherTargetType?: string;
    /** Required only when the role is not yet first-class product vocabulary. */
    readonly otherSemanticRole?: string;
  }[];
  readonly gates: readonly { readonly gateId: string; readonly sprintPlanRevisionId: string }[];
  readonly gateCriteriaRevisions: readonly {
    readonly gateCriteriaRevisionId: string;
    readonly gateId: string;
    readonly revision: number;
  }[];
  readonly feedbackRecords: readonly {
    readonly feedbackRecordId: string;
    readonly gateId: string;
    readonly boundary: 'auto_flow_off' | 'designed_feedback_flow' | 'all_pending_work_blocked';
    readonly provenanceId: string;
  }[];
  /** Eligibility is recorded separately from any requested or observed continuation. */
  readonly policyEligibilityFacts: readonly {
    readonly policyEligibilityFactId: string;
    readonly level: 'sprint' | 'epic';
    readonly targetId: string;
    readonly autoFlowEnabled: boolean;
    readonly eligible: boolean;
    readonly provenanceId: string;
  }[];
  /** Requested intent only; a request does not establish an observed planner launch. */
  readonly executionRequests: readonly {
    readonly executionRequestId: string;
    readonly workUnitExecutionId: string;
    readonly provenanceId: string;
  }[];
  readonly observedLaunches: readonly {
    readonly observedLaunchId: string;
    readonly executionRequestId: string;
    readonly workUnitExecutionId: string;
    readonly attemptId: string;
    readonly provenanceId: string;
  }[];
  readonly observedReturns: readonly {
    readonly observedReturnId: string;
    readonly observedLaunchId: string;
    readonly attemptId: string;
    readonly provenanceId: string;
  }[];
  readonly reviews: readonly {
    readonly reviewId: string;
    readonly subjectKind:
      'work_unit_execution' | 'attempt' | 'sprint_plan_revision' | 'document_reference';
    readonly subjectId: string;
    readonly outcome?: 'accepted' | 'needs_correction' | 'blocked';
    readonly rationaleArtifactId?: string;
    readonly provenanceId: string;
  }[];
  readonly observedIntegrations: readonly {
    readonly observedIntegrationId: string;
    readonly workUnitExecutionId: string;
    readonly provenanceId: string;
  }[];
  /** Completion records accepted responsibility; it never implies integration or continuation. */
  readonly observedCompletions: readonly {
    readonly observedCompletionId: string;
    readonly subjectKind: 'work_unit_execution' | 'sprint';
    readonly subjectId: string;
    readonly responsibilityAccepted: boolean;
    readonly provenanceId: string;
  }[];
  readonly continuationRequests: readonly {
    readonly continuationRequestId: string;
    readonly policyEligibilityFactId: string;
    readonly targetKind: 'next_work_unit' | 'next_sprint_planner';
    readonly targetId: string;
    readonly provenanceId: string;
  }[];
  readonly observedContinuations: readonly {
    readonly observedContinuationId: string;
    readonly continuationRequestId: string;
    readonly provenanceId: string;
  }[];
  /** A handoff is observed only when this fact is recorded; an idle Sprint is not a handoff. */
  readonly observedHandoffs: readonly {
    readonly observedHandoffId: string;
    readonly sprintId: string;
    readonly provenanceId: string;
  }[];
  readonly internalArtifacts: readonly {
    readonly artifactId: string;
    readonly provenanceId: string;
  }[];
  readonly documentReferences: readonly {
    readonly documentRefId: string;
    readonly artifactIds: readonly string[];
    readonly provenanceId: string;
  }[];
  /** Immutable source and causal links for a recorded fact. */
  readonly provenance: readonly {
    readonly provenanceId: string;
    readonly sourceKind:
      'user' | 'agent_session' | 'application' | 'repository' | 'system' | 'other';
    readonly recordedAt: string;
    readonly causalFactIds: readonly string[];
    readonly actorAgentSessionRefId?: string;
  }[];
}

export interface WorkUnitObservedEvidence {
  readonly workUnitId: string;
  readonly plannedScopeIds: readonly string[];
  readonly executionRequested: boolean;
  readonly observedLaunched: boolean;
  readonly observedReturned: boolean;
  readonly observedReviewed: boolean;
  readonly observedIntegrated: boolean;
  readonly responsibilityAccepted: boolean;
}

/** Pure read-model primitive: only explicit Orchestration Events produce observed outcomes. */
export function deriveWorkUnitObservedEvidence(
  events: OrchestrationEventsV1,
  workUnitId: string,
): WorkUnitObservedEvidence {
  const plannedScopeIds = events.workUnitScopes
    .filter((scope) => scope.workUnitId === workUnitId)
    .map((scope) => scope.workUnitScopeId);
  const executions = events.workUnitExecutions.filter(
    (execution) => execution.workUnitId === workUnitId,
  );
  const executionIds = new Set(executions.map((execution) => execution.workUnitExecutionId));
  const attemptIds = new Set(
    events.attempts
      .filter((attempt) => executionIds.has(attempt.workUnitExecutionId))
      .map((attempt) => attempt.attemptId),
  );
  const launchIds = new Set(
    events.observedLaunches
      .filter((launch) => executionIds.has(launch.workUnitExecutionId))
      .map((launch) => launch.observedLaunchId),
  );
  return {
    workUnitId,
    plannedScopeIds,
    executionRequested: events.executionRequests.some((request) =>
      executionIds.has(request.workUnitExecutionId),
    ),
    observedLaunched: launchIds.size > 0,
    observedReturned: events.observedReturns.some((returned) =>
      launchIds.has(returned.observedLaunchId),
    ),
    observedReviewed: events.reviews.some(
      (review) =>
        (review.subjectKind === 'work_unit_execution' && executionIds.has(review.subjectId)) ||
        (review.subjectKind === 'attempt' && attemptIds.has(review.subjectId)),
    ),
    observedIntegrated: events.observedIntegrations.some((integration) =>
      executionIds.has(integration.workUnitExecutionId),
    ),
    responsibilityAccepted: events.observedCompletions.some(
      (completion) =>
        completion.subjectKind === 'work_unit_execution' &&
        executionIds.has(completion.subjectId) &&
        completion.responsibilityAccepted,
    ),
  };
}
