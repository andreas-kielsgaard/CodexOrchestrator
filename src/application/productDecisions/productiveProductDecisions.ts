/** Product-owned durable boundary. Recorded development decisions use a separate read contract. */
export type ProductDecisionAgentPassage = Readonly<{
  sessionId: string;
  invocationId: string;
  passage:
    | Readonly<{ kind: 'submitted_input' }>
    | Readonly<{ kind: 'runtime_event'; runtimeEventId: string }>;
}>;

export type ProductDecisionAcceptanceProvenance =
  | Readonly<{ kind: 'manual_human_application' }>
  | Readonly<{ kind: 'agent_assisted'; passage: ProductDecisionAgentPassage }>;

export type ProductDecisionCurrentActionableEvidence = Readonly<{
  evidenceId: string;
  passage: ProductDecisionAgentPassage;
}>;

/** Retained audit context only: it has no current actionable destination until relinked. */
export type ProductDecisionHistoricalUnresolvedEvidence = Readonly<{
  evidenceId: string;
  legacyReference: string;
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
  loadCurrent(): Promise<readonly ProductDecisionCurrent[]>;
  loadHistory(epicId: string, decisionId: string): Promise<readonly ProductDecisionVersion[]>;
  /** This is the explicit human acceptance boundary for both manual and agent-assisted material. */
  acceptVersion(input: AcceptProductDecisionVersionInput): Promise<ProductDecisionVersion>;
}
