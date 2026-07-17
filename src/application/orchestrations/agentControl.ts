/**
 * Provider-neutral Agent Control commands. These describe requests and recorded results only;
 * they do not deliver prompts, execute transitions, or turn requests into observed effects.
 */
export const AGENT_CONTROL_CONTRACTS_V1 = 'orchestration-agent-control/v1' as const;

export type PromptSourceKind =
  | 'user_authored'
  | 'agent_session_derived'
  | 'application_produced'
  | 'repository_or_system_derived'
  | 'other';

export interface PromptProvenanceV1 {
  readonly promptProvenanceId: string;
  readonly sourceKind: PromptSourceKind;
  /** Required only for an intentionally extensible source classification. */
  readonly otherSourceType?: string;
  readonly sourceReference: string;
  readonly causalInputReferences: readonly string[];
}

export type AgentControlTargetV1 =
  | { readonly kind: 'next_ready_work_unit_planner'; readonly sprintId: string }
  | { readonly kind: 'next_sprint_planner'; readonly epicId: string }
  | { readonly kind: 'agent_session'; readonly agentSessionRefId: string };

export type AgentControlCommandKind =
  | 'request_next_ready_work_unit_planner'
  | 'request_next_sprint_planner'
  | 'request_agent_session_prompt';

export interface AgentControlCommandV1 {
  readonly agentControlCommandId: string;
  readonly commandKind: AgentControlCommandKind;
  /** The neutral Agent Session that receives this command and may act through MCP. */
  readonly recipientAgentSessionRefId: string;
  readonly target: AgentControlTargetV1;
  readonly idempotency: {
    readonly key: string;
    readonly scopeKind: 'sprint' | 'epic' | 'agent_session';
    readonly scopeId: string;
  };
  readonly initiatedBy: {
    readonly sourceKind: PromptSourceKind;
    readonly sourceReference: string;
    readonly otherSourceType?: string;
  };
  /** Every Agent Control command is backed by a durable prompt artifact or source record. */
  readonly promptProvenanceId: string;
  readonly recordedAt: string;
  /** Evidence is an explicit reference; the envelope does not infer it from policy state. */
  readonly preconditionEvidenceReference: string;
  /** Required for continuation requests and forbidden on generic Agent Session prompt requests. */
  readonly continuation?: {
    readonly policyId: string;
    readonly eligibilityEvaluationId: string;
  };
}

export type ContinuationPolicyV1 =
  | {
      readonly continuationPolicyId: string;
      readonly level: 'sprint';
      readonly sprintId: string;
      readonly autoFlowEnabled: boolean;
    }
  | {
      readonly continuationPolicyId: string;
      readonly level: 'epic';
      readonly epicId: string;
      readonly autoFlowEnabled: boolean;
    };

export type ContinuationEligibilityStatus = 'eligible' | 'ineligible' | 'feedback_required';
export type FeedbackBoundary =
  'auto_flow_off' | 'designed_feedback_flow' | 'all_pending_work_blocked';

export interface ContinuationEligibilityEvaluationV1 {
  readonly continuationEligibilityEvaluationId: string;
  readonly continuationPolicyId: string;
  readonly level: 'sprint' | 'epic';
  readonly target: Extract<
    AgentControlTargetV1,
    { readonly kind: 'next_ready_work_unit_planner' | 'next_sprint_planner' }
  >;
  readonly requiredConditionsSatisfied: boolean;
  readonly designedForFeedback: boolean;
  readonly allPendingDevelopmentTechnicallyBlocked: boolean;
  readonly recordedAt: string;
  readonly result: {
    readonly status: ContinuationEligibilityStatus;
    readonly feedbackBoundary?: FeedbackBoundary;
  };
}

export interface AgentControlResultV1 {
  readonly agentControlResultId: string;
  readonly agentControlCommandId: string;
  readonly state:
    | 'requested'
    | 'acknowledged'
    | 'unsupported'
    | 'denied_ineligible'
    | 'failed'
    | 'orchestration_event_recorded';
  readonly recordedAt: string;
  /** Present only when handling the command resulted in an Orchestration Event. */
  readonly orchestrationEventReference?: string;
}

export interface AgentControlContractsV1 {
  readonly version: typeof AGENT_CONTROL_CONTRACTS_V1;
  readonly promptProvenance: readonly PromptProvenanceV1[];
  readonly continuationPolicies: readonly ContinuationPolicyV1[];
  readonly continuationEligibilityEvaluations: readonly ContinuationEligibilityEvaluationV1[];
  readonly commands: readonly AgentControlCommandV1[];
  readonly results: readonly AgentControlResultV1[];
}

export interface ContinuationEligibilityProjection {
  readonly status: ContinuationEligibilityStatus;
  readonly feedbackBoundary?: FeedbackBoundary;
}

/** Pure policy projection. Eligibility never proves that a request or continuation occurred. */
export function projectContinuationEligibility(
  policy: ContinuationPolicyV1,
  input: Pick<
    ContinuationEligibilityEvaluationV1,
    | 'requiredConditionsSatisfied'
    | 'designedForFeedback'
    | 'allPendingDevelopmentTechnicallyBlocked'
  >,
): ContinuationEligibilityProjection {
  if (!policy.autoFlowEnabled)
    return { status: 'feedback_required', feedbackBoundary: 'auto_flow_off' };
  if (input.designedForFeedback)
    return { status: 'feedback_required', feedbackBoundary: 'designed_feedback_flow' };
  if (input.allPendingDevelopmentTechnicallyBlocked)
    return { status: 'feedback_required', feedbackBoundary: 'all_pending_work_blocked' };
  return { status: input.requiredConditionsSatisfied ? 'eligible' : 'ineligible' };
}

/** Pure read model: only an explicit event-recorded result proves an Orchestration Event. */
export function projectAgentControlOutcome(
  contracts: AgentControlContractsV1,
  agentControlCommandId: string,
) {
  const results = contracts.results.filter(
    (result) => result.agentControlCommandId === agentControlCommandId,
  );
  return {
    requested: contracts.commands.some(
      (command) => command.agentControlCommandId === agentControlCommandId,
    ),
    acknowledged: results.some((result) => result.state === 'acknowledged'),
    orchestrationEventRecorded: results.some(
      (result) => result.state === 'orchestration_event_recorded',
    ),
  };
}

/** Deterministic duplicate grouping; collisions are rejected by the decoder. */
export function projectIdempotency(
  contracts: AgentControlContractsV1,
  agentControlCommandId: string,
) {
  const command = contracts.commands.find(
    (candidate) => candidate.agentControlCommandId === agentControlCommandId,
  );
  if (!command) return { recognized: false as const, duplicateCommandIds: [] as readonly string[] };
  const duplicateCommandIds = contracts.commands
    .filter(
      (candidate) =>
        candidate.idempotency.key === command.idempotency.key &&
        candidate.idempotency.scopeKind === command.idempotency.scopeKind &&
        candidate.idempotency.scopeId === command.idempotency.scopeId,
    )
    .map((candidate) => candidate.agentControlCommandId);
  return { recognized: duplicateCommandIds.length > 1, duplicateCommandIds };
}
