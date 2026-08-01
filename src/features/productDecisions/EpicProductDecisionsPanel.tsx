import { AlertTriangle, GitMerge, ShieldCheck } from 'lucide-react';
import { useEffect, useMemo, useState } from 'react';
import type {
  EpicProductDecisionLoadResult,
  EpicProductDecisionSource,
  ProductDecision,
  ProductDecisionEvidence,
} from '../../application/productDecisions';
import './epicProductDecisions.css';

export interface EpicProductDecisionsPanelProps {
  readonly epicId: string;
  readonly source: EpicProductDecisionSource;
}

export function EpicProductDecisionsPanel({ epicId, source }: EpicProductDecisionsPanelProps) {
  const [load, setLoad] = useState<EpicProductDecisionLoadResult | { kind: 'loading' }>({
    kind: 'loading',
  });
  const [section, setSection] = useState<'tree' | 'review'>('tree');
  const [selectedDecisionId, setSelectedDecisionId] = useState<string | null>(null);

  useEffect(() => {
    let active = true;
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

  const snapshot = load.kind === 'available' ? load.snapshot : undefined;
  useEffect(() => {
    if (!snapshot?.decisions.length) {
      setSelectedDecisionId(null);
      return;
    }
    if (!snapshot.decisions.some(({ decisionId }) => decisionId === selectedDecisionId))
      setSelectedDecisionId(snapshot.decisions[0].decisionId);
  }, [selectedDecisionId, snapshot]);

  const decisionById = useMemo(
    () => new Map(snapshot?.decisions.map((decision) => [decision.decisionId, decision]) ?? []),
    [snapshot],
  );
  const evidenceById = useMemo(
    () => new Map(snapshot?.evidence.map((item) => [item.evidenceId, item]) ?? []),
    [snapshot],
  );
  const selected = selectedDecisionId ? decisionById.get(selectedDecisionId) : undefined;

  if (load.kind !== 'available')
    return (
      <section
        className="product-decisions product-decisions--message"
        aria-label="Product decisions"
      >
        <p className="eyebrow">Product decisions</p>
        <h2>
          {load.kind === 'loading' ? 'Loading product decisions' : 'Product decisions unavailable'}
        </h2>
        {load.kind !== 'loading' && <p role="alert">{load.reason}</p>}
      </section>
    );
  const available = load.snapshot;

  return (
    <section className="product-decisions" aria-label="Product decisions">
      <header className="product-decisions__header">
        <div>
          <p className="eyebrow">Epic product identity</p>
          <h2>Product decisions</h2>
          <p>Recorded reasoning intended to guide future work.</p>
        </div>
        <dl className="product-decisions__summary">
          <div>
            <dt>Current</dt>
            <dd>{available.decisions.length}</dd>
          </div>
          <div className={available.conflicts.length ? 'needs-attention' : undefined}>
            <dt>Needs review</dt>
            <dd>{available.conflicts.length}</dd>
          </div>
        </dl>
      </header>

      <nav className="product-decisions__tabs" aria-label="Product decision views">
        <button
          type="button"
          className={section === 'tree' ? 'active' : undefined}
          aria-current={section === 'tree' ? 'page' : undefined}
          onClick={() => setSection('tree')}
        >
          Decision tree
        </button>
        <button
          type="button"
          className={section === 'review' ? 'active' : undefined}
          aria-current={section === 'review' ? 'page' : undefined}
          onClick={() => setSection('review')}
        >
          Review conflicts
          {available.conflicts.length > 0 && <span>{available.conflicts.length}</span>}
        </button>
      </nav>

      {section === 'tree' ? (
        <div className="product-decisions__workspace">
          <nav className="product-decisions__tree" aria-label="Decision tree">
            <DecisionTree
              decisions={available.decisions}
              selectedDecisionId={selectedDecisionId}
              onSelect={setSelectedDecisionId}
            />
          </nav>
          <article className="product-decisions__detail" aria-live="polite">
            {selected ? (
              <>
                <p className="eyebrow">Current decision</p>
                <h3>{selected.title}</h3>
                <p className="product-decisions__statement">{selected.statement}</p>
                <section>
                  <h4>Why it matters</h4>
                  <p>{selected.intent}</p>
                </section>
                {selected.lineage.kind !== 'introduced' && (
                  <p className="product-decisions__lineage">
                    <GitMerge size={16} aria-hidden="true" />
                    {selected.lineage.kind === 'combined'
                      ? `Combines ${selected.lineage.supersedesDecisionIds.length} earlier decisions into this policy.`
                      : 'Refines an earlier expression of this policy.'}
                  </p>
                )}
                <EvidenceList
                  evidence={selected.evidenceIds
                    .map((evidenceId) => evidenceById.get(evidenceId))
                    .filter((item): item is ProductDecisionEvidence => Boolean(item))}
                />
                {available.complianceReviewRequests
                  .filter(
                    ({ triggeredByDecisionId }) => triggeredByDecisionId === selected.decisionId,
                  )
                  .map((review) => (
                    <aside className="product-decisions__compliance" key={review.requestId}>
                      <ShieldCheck size={18} aria-hidden="true" />
                      <div>
                        <strong>Codebase review requested</strong>
                        <p>{review.reason}</p>
                      </div>
                    </aside>
                  ))}
              </>
            ) : (
              <p>No product decision is selected.</p>
            )}
          </article>
        </div>
      ) : (
        <div className="product-decisions__reviews">
          <header>
            <AlertTriangle size={20} aria-hidden="true" />
            <div>
              <h3>Human judgment required</h3>
              <p>Contrary candidates never rewrite current product policy automatically.</p>
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
                          .map((decisionId) => decisionById.get(decisionId)?.title ?? decisionId)
                          .join(', ')}
                      </dd>
                    </div>
                  </dl>
                  <EvidenceList
                    evidence={candidate.evidenceIds
                      .map((evidenceId) => evidenceById.get(evidenceId))
                      .filter((item): item is ProductDecisionEvidence => Boolean(item))}
                  />
                  <p className="product-decisions__read-only-note">
                    Recorded candidate only. Acceptance and rejection are not implemented in this
                    exploration.
                  </p>
                </article>
              );
            })
          ) : (
            <p>No contrary decision candidates need review.</p>
          )}
        </div>
      )}
    </section>
  );
}

function DecisionTree({
  decisions,
  selectedDecisionId,
  onSelect,
}: {
  readonly decisions: readonly ProductDecision[];
  readonly selectedDecisionId: string | null;
  readonly onSelect: (decisionId: string) => void;
}) {
  const children = (parentDecisionId?: string) =>
    decisions.filter((decision) => decision.parentDecisionId === parentDecisionId);
  const render = (decision: ProductDecision, depth: number) => (
    <li key={decision.decisionId}>
      <button
        type="button"
        className={decision.decisionId === selectedDecisionId ? 'active' : undefined}
        aria-current={decision.decisionId === selectedDecisionId ? 'true' : undefined}
        style={{ '--decision-depth': depth } as React.CSSProperties}
        onClick={() => onSelect(decision.decisionId)}
      >
        <span>{decision.title}</span>
        {decision.lineage.kind === 'combined' && <small>Combined</small>}
      </button>
      {children(decision.decisionId).length > 0 && (
        <ul>{children(decision.decisionId).map((child) => render(child, depth + 1))}</ul>
      )}
    </li>
  );
  return <ul>{children().map((decision) => render(decision, 0))}</ul>;
}

function EvidenceList({ evidence }: { readonly evidence: readonly ProductDecisionEvidence[] }) {
  return (
    <section className="product-decisions__sources">
      <h4>Derived from</h4>
      <ul>
        {evidence.map((item) => (
          <li key={item.evidenceId}>
            <span>{evidenceKindLabel(item.kind)}</span>
            <strong>{item.label}</strong>
          </li>
        ))}
      </ul>
    </section>
  );
}

function evidenceKindLabel(kind: ProductDecisionEvidence['kind']) {
  return {
    human_interaction: 'Human input',
    agent_session_completed: 'Agent Session',
    work_unit_approved: 'Work Unit approval',
    sprint_completed: 'Sprint completion',
    epic_completed: 'Epic completion',
  }[kind];
}
