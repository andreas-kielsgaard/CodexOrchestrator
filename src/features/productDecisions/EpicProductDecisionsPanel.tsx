import { AlertTriangle, GitMerge, ShieldCheck } from 'lucide-react';
import { useEffect, useMemo, useState } from 'react';
import type {
  EpicProductDecisionLoadResult,
  EpicProductDecisionSource,
  ProductDecision,
  ProductDecisionEvidence,
  ProductDecisionEvidenceNavigationRequest,
  ProductDecisionClient,
  ProductDecisionCorrectionClient,
  ProductDecisionEvidenceDestination,
  ProductDecisionPublishTarget,
} from '../../application/productDecisions';
import type { AgentSessionClient } from '../../application/agentSessions';
import { ProductiveProductDecisionsPanel } from './ProductiveProductDecisionsPanel';
import './epicProductDecisions.css';

export interface EpicProductDecisionsPanelProps {
  readonly epicId: string;
  readonly source?: EpicProductDecisionSource;
  readonly productiveClient?: ProductDecisionClient;
  readonly correctionClient?: ProductDecisionCorrectionClient;
  readonly agentSessionClient?: AgentSessionClient;
  /** The application owns cross-surface navigation after the source resolves an exact record. */
  readonly onOpenEvidence?: (request: ProductDecisionEvidenceNavigationRequest) => void;
  readonly onOpenProductiveEvidence?: (destination: ProductDecisionEvidenceDestination) => void;
  readonly onPublish?: (target: ProductDecisionPublishTarget) => void;
}

export function EpicProductDecisionsPanel({
  epicId,
  source,
  productiveClient,
  correctionClient,
  agentSessionClient,
  onOpenEvidence,
  onOpenProductiveEvidence,
  onPublish,
}: EpicProductDecisionsPanelProps) {
  const [load, setLoad] = useState<EpicProductDecisionLoadResult | { kind: 'loading' }>({
    kind: 'loading',
  });
  const [reviewExpanded, setReviewExpanded] = useState(false);

  useEffect(() => {
    let active = true;
    if (!source) return undefined;
    setLoad({ kind: 'loading' });
    void source.loadEpicProductDecisions(epicId).then(
      (result) => {
        if (active) setLoad(result);
      },
      () => {
        if (active) setLoad({ kind: 'invalid', reason: 'Product decisions could not be loaded.' });
      },
    );
    return () => {
      active = false;
    };
  }, [epicId, source]);

  const productivePanel = productiveClient ? (
    <ProductiveProductDecisionsPanel
      epicId={epicId}
      client={productiveClient}
      correctionClient={correctionClient}
      agentSessionClient={agentSessionClient}
      onOpenEvidence={onOpenProductiveEvidence}
      onPublish={onPublish}
    />
  ) : null;

  const snapshot = load.kind === 'available' ? load.snapshot : undefined;
  const decisionById = useMemo(
    () => new Map(snapshot?.decisions.map((decision) => [decision.decisionId, decision]) ?? []),
    [snapshot],
  );
  const evidenceById = useMemo(
    () => new Map(snapshot?.evidence.map((item) => [item.evidenceId, item]) ?? []),
    [snapshot],
  );

  if (!source)
    return (
      productivePanel ?? (
        <section
          className="product-decisions product-decisions--message"
          aria-label="Product decisions"
        >
          <p className="eyebrow">Product decisions</p>
          <h2>Product decisions unavailable</h2>
          <p role="alert">No Product Decision read boundary is available.</p>
        </section>
      )
    );

  if (load.kind !== 'available')
    return (
      <>
        {productivePanel}
        <section
          className="product-decisions product-decisions--message"
          aria-label="Recorded Product decisions"
        >
          <p className="eyebrow">Recorded development preview</p>
          <h2>
            {load.kind === 'loading'
              ? 'Loading recorded product decisions'
              : 'Recorded preview unavailable'}
          </h2>
          {load.kind !== 'loading' && <p role="alert">{load.reason}</p>}
        </section>
      </>
    );
  const available = load.snapshot;

  const evidenceRequest = (evidence: ProductDecisionEvidence) =>
    evidence.conversationCitation
      ? {
          epicId,
          evidenceId: evidence.evidenceId,
          originReference: evidence.originReference,
          conversationCitation: evidence.conversationCitation,
        }
      : null;

  return (
    <>
      {productivePanel}
      <section className="product-decisions" aria-label="Recorded Product decisions">
        <header className="product-decisions__header">
          <div>
            <p className="eyebrow">Epic product identity</p>
            <h2>Product decisions</h2>
            <p>Recorded development preview only. It is not productive durable authority.</p>
          </div>
          <button
            className="product-decisions__review-toggle"
            type="button"
            aria-expanded={reviewExpanded}
            aria-controls="product-decisions-review"
            onClick={() => setReviewExpanded((current) => !current)}
          >
            Review recorded changes
            {available.conflicts.length > 0 && <span>{available.conflicts.length}</span>}
          </button>
        </header>

        <div className="product-decisions__content">
          <section
            className="product-decisions__current"
            aria-labelledby="current-decisions-heading"
          >
            <h3 id="current-decisions-heading">Current decisions</h3>
            <p>Relationships appear only when they are explicitly recorded.</p>
            <DecisionHierarchy
              decisions={available.decisions}
              decisionById={decisionById}
              evidenceById={evidenceById}
              complianceReviewRequests={available.complianceReviewRequests}
              resolveEvidence={(evidence) => {
                const request = evidenceRequest(evidence);
                return request
                  ? { request, resolution: source.resolveEvidenceNavigation(request) }
                  : null;
              }}
              onOpenEvidence={onOpenEvidence}
            />
          </section>

          {reviewExpanded && (
            <section
              className="product-decisions__reviews"
              id="product-decisions-review"
              aria-label="Recorded Product Decision changes"
            >
              <header>
                <AlertTriangle size={19} aria-hidden="true" />
                <div>
                  <h3>Changes needing human review</h3>
                  <p>Recorded candidates cannot rewrite current policy in this read-only view.</p>
                </div>
              </header>
              {available.conflicts.length ? (
                available.conflicts.map((conflict) => {
                  const candidate = available.candidates.find(
                    ({ candidateId }) => candidateId === conflict.candidateId,
                  );
                  if (!candidate) return null;
                  return (
                    <article key={conflict.conflictId}>
                      <p className="eyebrow">Proposed change</p>
                      <h4>{candidate.title}</h4>
                      <blockquote>{candidate.proposedStatement}</blockquote>
                      <p>{conflict.explanation}</p>
                      <dl>
                        <div>
                          <dt>Conflicts with</dt>
                          <dd>
                            {conflict.conflictsWithDecisionIds
                              .map(
                                (decisionId) => decisionById.get(decisionId)?.title ?? decisionId,
                              )
                              .join(', ')}
                          </dd>
                        </div>
                      </dl>
                      <EvidenceList
                        evidence={candidate.evidenceIds
                          .map((evidenceId) => evidenceById.get(evidenceId))
                          .filter((item): item is ProductDecisionEvidence => Boolean(item))}
                        resolveEvidence={(evidence) => {
                          const request = evidenceRequest(evidence);
                          return request
                            ? { request, resolution: source.resolveEvidenceNavigation(request) }
                            : null;
                        }}
                        onOpenEvidence={onOpenEvidence}
                      />
                      <p className="product-decisions__read-only-note">
                        Recorded candidate only. Acceptance and rejection are not implemented.
                      </p>
                    </article>
                  );
                })
              ) : (
                <p>No recorded changes need review.</p>
              )}
            </section>
          )}
        </div>
      </section>
    </>
  );
}

function DecisionHierarchy({
  decisions,
  decisionById,
  evidenceById,
  complianceReviewRequests,
  resolveEvidence,
  onOpenEvidence,
}: {
  readonly decisions: readonly ProductDecision[];
  readonly decisionById: ReadonlyMap<string, ProductDecision>;
  readonly evidenceById: ReadonlyMap<string, ProductDecisionEvidence>;
  readonly complianceReviewRequests: readonly {
    readonly requestId: string;
    readonly triggeredByDecisionId: string;
    readonly reason: string;
  }[];
  readonly resolveEvidence: (evidence: ProductDecisionEvidence) => Readonly<{
    request: ProductDecisionEvidenceNavigationRequest;
    resolution: ReturnType<EpicProductDecisionSource['resolveEvidenceNavigation']>;
  }> | null;
  readonly onOpenEvidence?: (request: ProductDecisionEvidenceNavigationRequest) => void;
}) {
  const children = (targetDecisionId?: string) =>
    decisions.filter(
      (decision) => decision.hierarchyRelationship?.targetDecisionId === targetDecisionId,
    );
  const render = (decision: ProductDecision, depth: number) => {
    const relationship = decision.hierarchyRelationship;
    const target = relationship ? decisionById.get(relationship.targetDecisionId) : undefined;
    const decisionEvidence = decision.evidenceIds
      .map((evidenceId) => evidenceById.get(evidenceId))
      .filter((item): item is ProductDecisionEvidence => Boolean(item));
    const reviewRequests = complianceReviewRequests.filter(
      ({ triggeredByDecisionId }) => triggeredByDecisionId === decision.decisionId,
    );
    return (
      <li key={decision.decisionId} data-decision-depth={depth}>
        <article>
          {relationship && target && (
            <p className="product-decisions__relationship">
              {relationshipLabel(relationship.kind)} <strong>{target.title}</strong>
            </p>
          )}
          <h4>{decision.title}</h4>
          <p className="product-decisions__statement">{decision.statement}</p>
          <details>
            <summary>Intent and evidence</summary>
            <div className="product-decisions__decision-detail">
              <h5>Why it matters</h5>
              <p>{decision.intent}</p>
              {decision.lineage.kind !== 'introduced' && (
                <p className="product-decisions__lineage">
                  <GitMerge size={16} aria-hidden="true" />
                  {decision.lineage.kind === 'combined'
                    ? `Combines ${decision.lineage.supersedesDecisionIds.length} recorded decisions.`
                    : 'Refines one recorded decision.'}
                </p>
              )}
              <EvidenceList
                evidence={decisionEvidence}
                resolveEvidence={resolveEvidence}
                onOpenEvidence={onOpenEvidence}
              />
              {reviewRequests.map((review) => (
                <aside className="product-decisions__compliance" key={review.requestId}>
                  <ShieldCheck size={18} aria-hidden="true" />
                  <div>
                    <strong>Codebase review requested</strong>
                    <p>{review.reason}</p>
                  </div>
                </aside>
              ))}
            </div>
          </details>
        </article>
        {children(decision.decisionId).length > 0 && (
          <ul>{children(decision.decisionId).map((child) => render(child, depth + 1))}</ul>
        )}
      </li>
    );
  };
  return (
    <ul className="product-decisions__hierarchy">{children().map((item) => render(item, 0))}</ul>
  );
}

function EvidenceList({
  evidence,
  resolveEvidence,
  onOpenEvidence,
}: {
  readonly evidence: readonly ProductDecisionEvidence[];
  readonly resolveEvidence: (evidence: ProductDecisionEvidence) => Readonly<{
    request: ProductDecisionEvidenceNavigationRequest;
    resolution: ReturnType<EpicProductDecisionSource['resolveEvidenceNavigation']>;
  }> | null;
  readonly onOpenEvidence?: (request: ProductDecisionEvidenceNavigationRequest) => void;
}) {
  return (
    <section className="product-decisions__sources">
      <h5>Evidence on record</h5>
      <ul>
        {evidence.map((item) => (
          <li key={item.evidenceId}>
            <span>{evidenceKindLabel(item.originReference.kind)}</span>
            <strong>{item.label}</strong>
            <small>Origin reference: {item.originReference.opaqueId}</small>
            {(() => {
              const resolved = resolveEvidence(item);
              return resolved?.resolution.kind === 'available' && onOpenEvidence ? (
                <button type="button" onClick={() => onOpenEvidence(resolved.request)}>
                  Open supporting Agent Session passage
                </button>
              ) : (
                <small className="product-decisions__evidence-unavailable">
                  Exact supporting evidence is unavailable.
                </small>
              );
            })()}
          </li>
        ))}
      </ul>
    </section>
  );
}

function relationshipLabel(kind: NonNullable<ProductDecision['hierarchyRelationship']>['kind']) {
  return {
    derives_from: 'Builds on',
    expands: 'Expands',
    contradicts: 'Contradicts',
  }[kind];
}

function evidenceKindLabel(kind: ProductDecisionEvidence['originReference']['kind']) {
  return {
    human_interaction: 'Human input',
    agent_session_completed: 'Agent Session',
    work_unit_approved: 'Work Unit approval',
    sprint_completed: 'Sprint completion',
    epic_completed: 'Epic completion',
  }[kind];
}
