/** Product-owned durable boundary. Recorded development decisions use a separate read contract. */
export type ProductDecisionAgentPassage = Readonly<{
  sessionId: string;
  invocationId: string;
  passage:
    | Readonly<{ kind: 'submitted_input' }>
    | Readonly<{ kind: 'outcome' }>
    | Readonly<{ kind: 'activity'; runtimeEventId: string }>
    | Readonly<{ kind: 'final_response'; runtimeEventId: string }>;
}>;

export type ProductDecisionEvidenceOriginReference =
  | Readonly<{ kind: 'human_interaction'; opaqueId: string }>
  | Readonly<{ kind: 'agent_session_completed'; opaqueId: string }>
  | Readonly<{ kind: 'work_unit_approved'; opaqueId: string }>
  | Readonly<{ kind: 'sprint_completed'; opaqueId: string }>
  | Readonly<{ kind: 'epic_completed'; opaqueId: string }>;

export type ProductDecisionHumanAcceptanceOrigin = Readonly<{
  kind: 'human_interaction';
  opaqueId: string;
}>;

export type ProductDecisionAcceptanceProvenance =
  | Readonly<{
      kind: 'manual_human_application';
      humanInteractionOrigin: ProductDecisionHumanAcceptanceOrigin;
    }>
  | Readonly<{
      kind: 'agent_assisted';
      humanInteractionOrigin: ProductDecisionHumanAcceptanceOrigin;
      proposalPassage: ProductDecisionAgentPassage;
    }>;

export type ProductDecisionCurrentActionableEvidence = Readonly<{
  evidenceId: string;
  originReference: ProductDecisionEvidenceOriginReference;
  /** Exact, application-recognized destination for the established Product Decision navigation seam. */
  destination: ProductDecisionAgentPassage;
}>;

/** Retained audit context only: it has no current actionable destination until relinked. */
export type ProductDecisionHistoricalUnresolvedEvidence = Readonly<{
  evidenceId: string;
  originReference: ProductDecisionEvidenceOriginReference;
  label: string;
}>;

export interface AcceptProductDecisionVersionInput {
  readonly decisionId: string;
  readonly epicId: string;
  readonly expectedCurrentVersion?: number;
  readonly idempotencyKey: string;
  readonly title: string;
  readonly statement: string;
  readonly intent: string;
  readonly acceptanceProvenance: ProductDecisionAcceptanceProvenance;
  readonly currentActionableEvidence?: readonly ProductDecisionCurrentActionableEvidence[];
  readonly historicalUnresolvedEvidence?: readonly ProductDecisionHistoricalUnresolvedEvidence[];
}

export interface ProductDecisionVersion {
  readonly versionId: string;
  readonly decisionId: string;
  readonly epicId: string;
  readonly version: number;
  readonly title: string;
  readonly statement: string;
  readonly intent: string;
  readonly acceptanceProvenance: ProductDecisionAcceptanceProvenance;
  readonly currentActionableEvidence: readonly ProductDecisionCurrentActionableEvidence[];
  readonly historicalUnresolvedEvidence: readonly ProductDecisionHistoricalUnresolvedEvidence[];
  readonly acceptedAt: string;
}

export interface ProductDecisionCurrent {
  readonly decisionId: string;
  readonly epicId: string;
  readonly currentVersion: ProductDecisionVersion;
  /** No scope or publication is inferred from an accepted current version. */
  readonly applicationState: 'not_applied';
}

export interface ProductDecisionClient {
  loadCurrent(epicId: string): Promise<readonly ProductDecisionCurrent[]>;
  loadHistory(epicId: string, decisionId: string): Promise<readonly ProductDecisionVersion[]>;
  /** This is the explicit human acceptance boundary for both manual and agent-assisted material. */
  acceptVersion(input: AcceptProductDecisionVersionInput): Promise<ProductDecisionVersion>;
}
