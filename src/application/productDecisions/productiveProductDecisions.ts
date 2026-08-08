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

export type ProductDecisionPublishTarget = Readonly<{
  epicId: string;
  decisionId: string;
  versionId: string;
  version: number;
}>;

export type ProductDecisionCommandErrorCode =
  'invalid_input' | 'revision_conflict' | 'idempotency_conflict' | 'not_found' | 'unavailable';

export function productDecisionCommandErrorCode(
  error: unknown,
): ProductDecisionCommandErrorCode | undefined {
  if (
    typeof error === 'object' &&
    error !== null &&
    'code' in error &&
    typeof error.code === 'string' &&
    new Set<ProductDecisionCommandErrorCode>([
      'invalid_input',
      'revision_conflict',
      'idempotency_conflict',
      'not_found',
      'unavailable',
    ]).has(error.code as ProductDecisionCommandErrorCode)
  )
    return error.code as ProductDecisionCommandErrorCode;
  return undefined;
}

export interface ProductDecisionClient {
  loadCurrent(epicId: string): Promise<readonly ProductDecisionCurrent[]>;
  loadHistory(epicId: string, decisionId: string): Promise<readonly ProductDecisionVersion[]>;
  /** This is the explicit human acceptance boundary for both manual and agent-assisted material. */
  acceptVersion(input: AcceptProductDecisionVersionInput): Promise<ProductDecisionVersion>;
}

/** A decision-owned conversation can discuss and retain proposals, but only this explicit
 * acceptance command creates a new official immutable version. */
export interface ProductDecisionCorrectionConversation {
  readonly correctionId: string;
  readonly epicId: string;
  readonly decisionId: string;
  readonly baseVersion: number;
  readonly sessionId: string;
  readonly latestProposal?: ProductDecisionCorrectionProposal;
}

export interface ProductDecisionCorrectionProposal {
  readonly proposalId: string;
  readonly correctionId: string;
  readonly title: string;
  readonly statement: string;
  readonly intent: string;
  readonly proposalPassage: ProductDecisionConversationPassageReference;
}

export interface ProductDecisionCorrectionClient {
  startConversation(
    input: Readonly<{
      epicId: string;
      decisionId: string;
      baseVersion: number;
    }>,
  ): Promise<ProductDecisionCorrectionConversation>;
  sendMessage(
    input: Readonly<{
      correctionId: string;
      submittedText: string;
    }>,
  ): Promise<Readonly<{ sessionId: string; invocationId: string }>>;
  saveProposal(
    input: Readonly<{
      correctionId: string;
      title: string;
      statement: string;
      intent: string;
      proposalPassage: ProductDecisionConversationPassageReference;
    }>,
  ): Promise<ProductDecisionCorrectionProposal>;
  acceptProposal(input: Readonly<{ proposalId: string }>): Promise<ProductDecisionVersion>;
}
