export type ProductDecisionEvidenceKind =
  | 'human_interaction'
  | 'agent_session_completed'
  | 'work_unit_approved'
  | 'sprint_completed'
  | 'epic_completed';

/** Provenance points to eligible work; it does not imply that work produced a decision. */
export interface ProductDecisionEvidence {
  readonly evidenceId: string;
  readonly kind: ProductDecisionEvidenceKind;
  readonly label: string;
  readonly occurredAt: string;
}

export interface ProductDecisionLineage {
  readonly kind: 'introduced' | 'refined' | 'combined';
  readonly supersedesDecisionIds: readonly string[];
}

/** A current reasoning-level policy, distinct from observations and enforceable rules. */
export interface ProductDecision {
  readonly decisionId: string;
  readonly parentDecisionId?: string;
  readonly title: string;
  readonly statement: string;
  readonly intent: string;
  readonly evidenceIds: readonly string[];
  readonly lineage: ProductDecisionLineage;
}

/** AI-generated material is non-authoritative until reconciliation accepts it. */
export interface ProductDecisionChangeCandidate {
  readonly candidateId: string;
  readonly title: string;
  readonly proposedStatement: string;
  readonly rationale: string;
  readonly relation: 'introduce' | 'refine' | 'combine';
  readonly targetDecisionIds: readonly string[];
  readonly evidenceIds: readonly string[];
}

/** A detected contradiction links a candidate to current policy for human review. */
export interface ProductDecisionConflict {
  readonly conflictId: string;
  readonly candidateId: string;
  readonly conflictsWithDecisionIds: readonly string[];
  readonly explanation: string;
  readonly status: 'pending_human_review';
}

/** This records a review request only; it does not claim that a codebase audit ran. */
export interface ProductDecisionComplianceReviewRequest {
  readonly requestId: string;
  readonly triggeredByDecisionId: string;
  readonly reason: string;
  readonly status: 'requested';
}

export interface EpicProductDecisionSnapshot {
  readonly epicId: string;
  readonly recordedAt: string;
  readonly decisions: readonly ProductDecision[];
  readonly candidates: readonly ProductDecisionChangeCandidate[];
  readonly conflicts: readonly ProductDecisionConflict[];
  readonly evidence: readonly ProductDecisionEvidence[];
  readonly complianceReviewRequests: readonly ProductDecisionComplianceReviewRequest[];
}

export type EpicProductDecisionLoadResult =
  | { readonly kind: 'available'; readonly snapshot: EpicProductDecisionSnapshot }
  | { readonly kind: 'unavailable'; readonly reason: string }
  | { readonly kind: 'invalid'; readonly reason: string };

/** Reusable Epic-scoped application read boundary; no Agent Session owns this state. */
export interface EpicProductDecisionSource {
  loadEpicProductDecisions(epicId: string): Promise<EpicProductDecisionLoadResult>;
}

export function validateEpicProductDecisionSnapshot(
  snapshot: EpicProductDecisionSnapshot,
): EpicProductDecisionSnapshot {
  requireText(snapshot.epicId, 'Epic identity');
  requireText(snapshot.recordedAt, 'Snapshot recorded time');
  const decisions = uniqueBy(snapshot.decisions, (item) => item.decisionId, 'decision');
  const candidates = uniqueBy(snapshot.candidates, (item) => item.candidateId, 'candidate');
  const evidence = uniqueBy(snapshot.evidence, (item) => item.evidenceId, 'evidence');
  uniqueBy(snapshot.conflicts, (item) => item.conflictId, 'conflict');
  uniqueBy(snapshot.complianceReviewRequests, (item) => item.requestId, 'review request');

  snapshot.evidence.forEach((item) => {
    requireText(item.label, 'Evidence label');
    requireText(item.occurredAt, 'Evidence time');
  });
  snapshot.decisions.forEach((decision) => {
    requireText(decision.title, 'Decision title');
    requireText(decision.statement, 'Decision statement');
    requireText(decision.intent, 'Decision intent');
    requireKnown(decision.evidenceIds, evidence, 'Decision evidence');
    decision.lineage.supersedesDecisionIds.forEach((decisionId) => {
      if (decisionId === decision.decisionId)
        fail('Decision lineage cannot supersede its current identity');
    });
    if (decision.parentDecisionId && !decisions.has(decision.parentDecisionId))
      fail('Decision parent must reference a current decision');
  });
  assertAcyclic(snapshot.decisions, decisions);
  snapshot.candidates.forEach((candidate) => {
    requireText(candidate.title, 'Candidate title');
    requireText(candidate.proposedStatement, 'Candidate statement');
    requireText(candidate.rationale, 'Candidate rationale');
    requireKnown(candidate.evidenceIds, evidence, 'Candidate evidence');
    candidate.targetDecisionIds.forEach((decisionId) => {
      if (!decisions.has(decisionId)) fail('Candidate targets an unknown decision');
    });
  });
  snapshot.conflicts.forEach((conflict) => {
    requireText(conflict.explanation, 'Conflict explanation');
    if (!candidates.has(conflict.candidateId)) fail('Conflict references an unknown candidate');
    requireKnown(conflict.conflictsWithDecisionIds, decisions, 'Conflict decision');
  });
  snapshot.complianceReviewRequests.forEach((request) => {
    requireText(request.reason, 'Compliance review reason');
    if (!decisions.has(request.triggeredByDecisionId))
      fail('Compliance review references an unknown decision');
  });
  return snapshot;
}

function uniqueBy<T>(
  values: readonly T[],
  identity: (value: T) => string,
  label: string,
): ReadonlyMap<string, T> {
  const result = new Map<string, T>();
  values.forEach((value) => {
    const id = identity(value);
    requireText(id, `${label} identity`);
    if (result.has(id)) fail(`${label} identities must be unique`);
    result.set(id, value);
  });
  return result;
}

function requireKnown<T>(ids: readonly string[], values: ReadonlyMap<string, T>, label: string) {
  if (!ids.length) fail(`${label} must retain at least one reference`);
  ids.forEach((id) => {
    if (!values.has(id)) fail(`${label} references an unknown identity`);
  });
}

function assertAcyclic(
  values: readonly ProductDecision[],
  decisions: ReadonlyMap<string, ProductDecision>,
) {
  values.forEach((value) => {
    const visited = new Set([value.decisionId]);
    let parentId = value.parentDecisionId;
    while (parentId) {
      if (visited.has(parentId)) fail('Decision tree cannot contain a cycle');
      visited.add(parentId);
      parentId = decisions.get(parentId)?.parentDecisionId;
    }
  });
}

function requireText(value: string, label: string) {
  if (!value.trim()) fail(`${label} cannot be empty`);
}

function fail(message: string): never {
  throw new Error(`Invalid Epic product decisions: ${message}`);
}
