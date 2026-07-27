export interface AgentReviewViewport {
  readonly width: number;
  readonly height: number;
}

export interface AgentReviewEnvironment {
  readonly platform: string;
  readonly viewport: AgentReviewViewport | null;
}

export interface AgentReviewSurface {
  readonly id: string;
  readonly name: string;
}

export interface AgentReviewScenario {
  readonly id: string;
  readonly name: string;
  readonly startingState: string;
  readonly actions: readonly string[];
}

export type AgentReviewClaimKind = 'behavior' | 'visual' | 'native-shell' | 'security';

export interface AgentReviewClaim {
  readonly id: string;
  readonly kind: AgentReviewClaimKind;
  readonly statement: string;
}

/**
 * A semantic capability requested from an adapter. Driver commands and protocols stay outside
 * this application boundary.
 */
export interface AgentReviewCapability {
  readonly id: string;
  readonly purpose: string;
}

export type AgentReviewEvidenceKind =
  | 'action-log'
  | 'assertion'
  | 'screenshot'
  | 'trace'
  | 'application-log'
  | 'native-observation'
  | 'artifact';

export interface AgentReviewEvidenceRequirement {
  readonly claimId: string;
  readonly evidenceKinds: readonly AgentReviewEvidenceKind[];
}

export interface AgentReviewRequest {
  readonly id: string;
  readonly revision: string;
  readonly worktree: string;
  readonly surface: AgentReviewSurface;
  readonly scenario: AgentReviewScenario;
  readonly environment: AgentReviewEnvironment;
  readonly claims: readonly AgentReviewClaim[];
  readonly capabilities: readonly AgentReviewCapability[];
  readonly evidenceRequirements: readonly AgentReviewEvidenceRequirement[];
}

/**
 * Exploration may discover facts; deterministic verification may support claims. Neither is the
 * review judgement itself.
 */
export type AgentReviewEvidenceLane = 'exploratory-control' | 'deterministic-verification';
export type AgentReviewApplicationMode = 'development' | 'test' | 'production';

export interface AgentReviewDriver {
  readonly name: string;
  readonly version: string | null;
}

export type AgentReviewActionOutcome = 'completed' | 'failed' | 'not-run';

export interface AgentReviewActionEvidence {
  readonly sequence: number;
  readonly description: string;
  /** Adapter execution only; an application effect needs separate assertion evidence. */
  readonly driverOutcome: AgentReviewActionOutcome;
}

export type AgentReviewAssertionOutcome = 'passed' | 'failed' | 'not-run';

export interface AgentReviewAssertionEvidence {
  readonly claimId: string;
  readonly description: string;
  readonly outcome: AgentReviewAssertionOutcome;
}

export interface AgentReviewProducedFile {
  readonly path: string;
  readonly kind: AgentReviewEvidenceKind;
}

export interface AgentReviewRuntimeEvidenceReference {
  readonly instanceId: string;
  readonly runtimeManifestPath: string;
}

export interface AgentReviewEvidenceBundle {
  readonly id: string;
  readonly requestId: string;
  readonly lane: AgentReviewEvidenceLane;
  readonly applicationMode: AgentReviewApplicationMode;
  readonly driver: AgentReviewDriver;
  readonly environment: AgentReviewEnvironment;
  readonly recordedAt: string;
  readonly startingState: string;
  readonly actions: readonly AgentReviewActionEvidence[];
  readonly assertions: readonly AgentReviewAssertionEvidence[];
  readonly producedFiles: readonly AgentReviewProducedFile[];
  /** Null only when evidence predates the worktree-runtime handoff. */
  readonly runtimeEvidence: AgentReviewRuntimeEvidenceReference | null;
  readonly observations: readonly string[];
  readonly unverifiedClaims: readonly string[];
}

export const agentReviewDispositions = [
  'accepted',
  'changes-required',
  'user-review-required',
  'blocked',
  'inconclusive',
] as const;

export type AgentReviewDisposition = (typeof agentReviewDispositions)[number];

export type AgentReviewFindingKind =
  'observation' | 'change-required' | 'user-decision' | 'blocker' | 'unverified';

export interface AgentReviewFinding {
  readonly kind: AgentReviewFindingKind;
  readonly summary: string;
  readonly claimIds: readonly string[];
}

/** A judgement over retained evidence; it does not imply that requested actions were applied. */
export interface AgentReviewResult {
  readonly kind: 'agent-judgement';
  readonly id: string;
  readonly requestId: string;
  readonly evidenceBundleIds: readonly string[];
  readonly disposition: AgentReviewDisposition;
  readonly summary: string;
  readonly findings: readonly AgentReviewFinding[];
}
