import type {
  ProductDecisionConversationPassageReference,
  ProductDecisionEvidenceDestination,
  ProductDecisionEvidenceOriginReference,
} from './epicProductDecisions';

/** Product-owned durable boundary. Recorded development decisions use the same typed origins and destinations. */
type ProductDecisionHumanAcceptanceOrigin = Extract<
  ProductDecisionEvidenceOriginReference,
  { readonly kind: 'human_interaction' }
>;

export type ProductDecisionAcceptanceProvenance =
  | Readonly<{
      kind: 'manual_human_application';
      humanInteractionOrigin: ProductDecisionHumanAcceptanceOrigin;
    }>
  | Readonly<{
      kind: 'agent_assisted';
      humanInteractionOrigin: ProductDecisionHumanAcceptanceOrigin;
      proposalPassage: ProductDecisionConversationPassageReference;
    }>;

export type ProductDecisionCurrentActionableEvidence = Readonly<{
  evidenceId: string;
  originReference: ProductDecisionEvidenceOriginReference;
  /** Exact, application-recognized destination for the established Product Decision navigation seam. */
  destination: ProductDecisionEvidenceDestination;
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
