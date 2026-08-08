export type ProductDecisionEvidenceKind =
  | 'human_interaction'
  | 'agent_session_completed'
  | 'work_unit_approved'
  | 'sprint_completed'
  | 'epic_completed';

/** An immutable, typed link to the originating application record. Its identity stays opaque. */
export type ProductDecisionEvidenceOriginReference =
  | { readonly kind: 'human_interaction'; readonly opaqueId: string }
  | { readonly kind: 'agent_session_completed'; readonly opaqueId: string }
  | { readonly kind: 'work_unit_approved'; readonly opaqueId: string }
  | { readonly kind: 'sprint_completed'; readonly opaqueId: string }
  | { readonly kind: 'epic_completed'; readonly opaqueId: string };

export type ProductDecisionConversationPassageReference = Readonly<{
  kind: 'agent_session_passage';
  sessionId: string;
  invocationId: string;
  passage:
    | Readonly<{ kind: 'submitted_input' | 'outcome' }>
    | Readonly<{ kind: 'activity' | 'final_response'; runtimeEventId: string }>;
}>;

/** A destination is available only when the source can match this full typed citation. */
export type ProductDecisionEvidenceDestination = Readonly<{
  kind: 'agent_session_passage';
  sessionId: string;
  invocationId: string;
  passage: ProductDecisionConversationPassageReference['passage'];
}>;

export type ProductDecisionEvidenceNavigationRequest = Readonly<{
  epicId: string;
  evidenceId: string;
  originReference: ProductDecisionEvidenceOriginReference;
  conversationCitation: ProductDecisionConversationPassageReference;
}>;

export type ProductDecisionEvidenceNavigationResolution =
  | { readonly kind: 'available'; readonly destination: ProductDecisionEvidenceDestination }
  | { readonly kind: 'unavailable' };

/** Provenance identifies eligible context; it does not claim that context caused a decision. */
export interface ProductDecisionEvidence {
  readonly evidenceId: string;
  readonly originReference: ProductDecisionEvidenceOriginReference;
  readonly conversationCitation?: ProductDecisionConversationPassageReference;
  readonly label: string;
  readonly occurredAt: string;
}

export interface ProductDecisionLineage {
  readonly kind: 'introduced' | 'refined' | 'combined';
  readonly supersedesDecisionIds: readonly string[];
}

export interface ProductDecisionHierarchyRelationship {
  readonly kind: 'derives_from' | 'expands' | 'contradicts';
  readonly targetDecisionId: string;
}

/** A current reasoning-level policy, distinct from observations and enforceable rules. */
export interface ProductDecision {
  readonly decisionId: string;
  readonly hierarchyRelationship?: ProductDecisionHierarchyRelationship;
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
  /** Resolves provenance only. It never derives a destination from display text or transcript prose. */
  resolveEvidenceNavigation(request: unknown): ProductDecisionEvidenceNavigationResolution;
}

/**
 * Resolves only an exact, current evidence record. Callers supply the source-owned snapshot;
 * malformed, cross-Epic, stale, or mismatched requests intentionally have no destination.
 */
export function resolveProductDecisionEvidenceNavigation(
  snapshot: EpicProductDecisionSnapshot,
  request: unknown,
): ProductDecisionEvidenceNavigationResolution {
  if (!isEvidenceNavigationRequest(request) || request.epicId !== snapshot.epicId)
    return { kind: 'unavailable' };
  const evidence = snapshot.evidence.find(({ evidenceId }) => evidenceId === request.evidenceId);
  if (
    !evidence ||
    !evidence.conversationCitation ||
    !sameOriginReference(evidence.originReference, request.originReference) ||
    !sameConversationCitation(evidence.conversationCitation, request.conversationCitation)
  )
    return { kind: 'unavailable' };
  return {
    kind: 'available',
    destination: {
      kind: 'agent_session_passage',
      sessionId: evidence.conversationCitation.sessionId,
      invocationId: evidence.conversationCitation.invocationId,
      passage: evidence.conversationCitation.passage,
    },
  };
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
    requireOriginReference(item.originReference);
    if (item.conversationCitation) requireConversationCitation(item.conversationCitation);
  });
  snapshot.decisions.forEach((decision) => {
    requireText(decision.title, 'Decision title');
    requireText(decision.statement, 'Decision statement');
    requireText(decision.intent, 'Decision intent');
    requireKnown(decision.evidenceIds, evidence, 'Decision evidence');
    requireRelationReferences(
      decision.lineage.kind,
      decision.lineage.supersedesDecisionIds,
      decisions,
      'Decision lineage',
      decision.decisionId,
    );
    if (decision.hierarchyRelationship) {
      if (
        !new Set(['derives_from', 'expands', 'contradicts']).has(
          decision.hierarchyRelationship.kind,
        )
      )
        fail('Decision hierarchy relationship kind is invalid');
      requireText(decision.hierarchyRelationship.targetDecisionId, 'Decision hierarchy reference');
      if (decision.hierarchyRelationship.targetDecisionId === decision.decisionId)
        fail('Decision hierarchy cannot reference its own identity');
      if (!decisions.has(decision.hierarchyRelationship.targetDecisionId))
        fail('Decision hierarchy references an unknown decision');
    }
  });
  assertHierarchyAcyclic(snapshot.decisions, decisions);
  assertLineageAcyclic(snapshot.decisions, decisions);
  snapshot.candidates.forEach((candidate) => {
    requireText(candidate.title, 'Candidate title');
    requireText(candidate.proposedStatement, 'Candidate statement');
    requireText(candidate.rationale, 'Candidate rationale');
    requireKnown(candidate.evidenceIds, evidence, 'Candidate evidence');
    requireRelationReferences(
      candidate.relation,
      candidate.targetDecisionIds,
      decisions,
      'Candidate target',
      candidate.candidateId,
    );
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
  const distinct = new Set<string>();
  ids.forEach((id) => {
    requireText(id, `${label} reference`);
    if (distinct.has(id)) fail(`${label} references must be distinct`);
    distinct.add(id);
    if (!values.has(id)) fail(`${label} references an unknown identity`);
  });
}

function requireRelationReferences<T>(
  relation: 'introduced' | 'refined' | 'combined' | 'introduce' | 'refine' | 'combine',
  ids: readonly string[],
  values: ReadonlyMap<string, T>,
  label: string,
  selfId: string,
) {
  const normalized =
    relation === 'introduce'
      ? 'introduced'
      : relation === 'refine'
        ? 'refined'
        : relation === 'combine'
          ? 'combined'
          : relation;
  if (normalized === 'introduced' && ids.length)
    fail(`${label} introduction cannot contain references`);
  if (normalized === 'refined' && ids.length !== 1)
    fail(`${label} refinement must identify exactly one reference`);
  if (normalized === 'combined' && ids.length < 2)
    fail(`${label} combination must identify at least two references`);
  const distinct = new Set<string>();
  ids.forEach((id) => {
    requireText(id, `${label} reference`);
    if (id === selfId) fail(`${label} cannot reference its own identity`);
    if (distinct.has(id)) fail(`${label} references must be distinct`);
    distinct.add(id);
    if (!values.has(id)) fail(`${label} references an unknown decision`);
  });
}

function assertHierarchyAcyclic(
  values: readonly ProductDecision[],
  decisions: ReadonlyMap<string, ProductDecision>,
) {
  values.forEach((value) => {
    const visited = new Set([value.decisionId]);
    let targetId = value.hierarchyRelationship?.targetDecisionId;
    while (targetId) {
      if (visited.has(targetId)) fail('Decision hierarchy cannot contain a cycle');
      visited.add(targetId);
      targetId = decisions.get(targetId)?.hierarchyRelationship?.targetDecisionId;
    }
  });
}

function assertLineageAcyclic(
  values: readonly ProductDecision[],
  decisions: ReadonlyMap<string, ProductDecision>,
) {
  const visit = (decisionId: string, path: ReadonlySet<string>) => {
    if (path.has(decisionId)) fail('Decision lineage cannot contain a cycle');
    const nextPath = new Set(path).add(decisionId);
    decisions
      .get(decisionId)
      ?.lineage.supersedesDecisionIds.forEach((targetId) => visit(targetId, nextPath));
  };
  values.forEach(({ decisionId }) => visit(decisionId, new Set()));
}

const evidenceKinds = new Set<ProductDecisionEvidenceKind>([
  'human_interaction',
  'agent_session_completed',
  'work_unit_approved',
  'sprint_completed',
  'epic_completed',
]);

function requireOriginReference(reference: unknown) {
  if (
    !isRecord(reference) ||
    !hasOnlyKeys(reference, ['kind', 'opaqueId']) ||
    !evidenceKinds.has(reference.kind as ProductDecisionEvidenceKind)
  )
    fail('Evidence origin reference kind is not eligible');
  requireText(reference.opaqueId as string, 'Evidence origin reference identity');
}

function requireConversationCitation(reference: unknown) {
  if (
    !isRecord(reference) ||
    !hasOnlyKeys(reference, ['kind', 'sessionId', 'invocationId', 'passage']) ||
    reference.kind !== 'agent_session_passage' ||
    !isRecord(reference.passage) ||
    !hasOnlyKeys(reference.passage, ['kind', 'runtimeEventId'])
  )
    fail('Conversation citation kind is invalid');
  requireText(reference.sessionId as string, 'Conversation citation Session identity');
  requireText(reference.invocationId as string, 'Conversation citation invocation identity');
  if (
    !reference.passage ||
    !new Set(['submitted_input', 'activity', 'final_response', 'outcome']).has(
      reference.passage.kind as string,
    )
  )
    fail('Conversation citation passage kind is invalid');
  if (
    (reference.passage.kind === 'activity' || reference.passage.kind === 'final_response') &&
    !(reference.passage.runtimeEventId as string).trim()
  )
    fail('Conversation citation runtime event identity cannot be empty');
}

function isEvidenceNavigationRequest(
  value: unknown,
): value is ProductDecisionEvidenceNavigationRequest {
  if (
    !isRecord(value) ||
    !hasOnlyKeys(value, ['epicId', 'evidenceId', 'originReference', 'conversationCitation'])
  )
    return false;
  if (!isNonEmptyText(value.epicId) || !isNonEmptyText(value.evidenceId)) return false;
  try {
    requireOriginReference(value.originReference);
    requireConversationCitation(value.conversationCitation);
    return true;
  } catch {
    return false;
  }
}

function sameOriginReference(
  left: ProductDecisionEvidenceOriginReference,
  right: ProductDecisionEvidenceOriginReference,
) {
  return left.kind === right.kind && left.opaqueId === right.opaqueId;
}

function sameConversationCitation(
  left: ProductDecisionConversationPassageReference,
  right: ProductDecisionConversationPassageReference,
) {
  return (
    left.kind === right.kind &&
    left.sessionId === right.sessionId &&
    left.invocationId === right.invocationId &&
    left.passage.kind === right.passage.kind &&
    ('runtimeEventId' in left.passage
      ? 'runtimeEventId' in right.passage &&
        left.passage.runtimeEventId === right.passage.runtimeEventId
      : !('runtimeEventId' in right.passage))
  );
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function hasOnlyKeys(value: Record<string, unknown>, allowed: readonly string[]) {
  return Object.keys(value).every((key) => allowed.includes(key));
}

function isNonEmptyText(value: unknown): value is string {
  return typeof value === 'string' && value.trim().length > 0;
}

function requireText(value: string, label: string) {
  if (!value.trim()) fail(`${label} cannot be empty`);
}

function fail(message: string): never {
  throw new Error(`Invalid Epic product decisions: ${message}`);
}
