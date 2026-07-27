import {
  ArrowUpRight,
  CheckCircle2,
  CircleDotDashed,
  FileCheck2,
  FlaskConical,
  ShieldAlert,
} from 'lucide-react';
import {
  recordedAgentReviewLab,
  type AgentReviewLabRecord,
  type AgentReviewLaneRecord,
} from './recordedAgentReview';
import './agentReviewLab.css';

export interface AgentReviewLabProps {
  readonly record?: AgentReviewLabRecord;
}

export function AgentReviewLab({ record = recordedAgentReviewLab }: AgentReviewLabProps) {
  return (
    <main className="agent-review-lab" aria-labelledby="agent-review-lab-heading">
      <header className="agent-review-lab__header">
        <div className="agent-review-lab__heading">
          <span className="agent-review-lab__mark" aria-hidden="true">
            <FlaskConical size={19} />
          </span>
          <div>
            <p className="eyebrow">Development / test</p>
            <h1 id="agent-review-lab-heading">Agent Review Lab</h1>
            <p>
              Three evidence lanes keep renderer checks, real-window inspection, and native
              contracts distinct while converging on one owned worktree instance.
            </p>
          </div>
        </div>
        <dl className="agent-review-lab__repository" aria-label="Recorded repository">
          <div>
            <dt>Revision</dt>
            <dd>{shortRevision(record.repository.revision)}</dd>
          </div>
          <div>
            <dt>Branch</dt>
            <dd>{record.repository.branch}</dd>
          </div>
          <div>
            <dt>Worktree</dt>
            <dd title={record.repository.worktree}>{record.repository.worktree}</dd>
          </div>
        </dl>
      </header>

      <aside className="agent-review-lab__warning" role="note" aria-label="Production warning">
        <ShieldAlert size={18} aria-hidden="true" />
        <div>
          <strong>Development evidence only</strong>
          <span>
            This screen reports bounded observations. It grants no production, native, or
            orchestration authority.
          </span>
        </div>
      </aside>

      <section className="agent-review-lab__lanes" aria-labelledby="review-lanes-heading">
        <header className="agent-review-lab__section-heading">
          <div>
            <p className="eyebrow">Review lanes</p>
            <h2 id="review-lanes-heading">Request → evidence → disposition</h2>
          </div>
          <p>Drivers stay in development adapters; retained facts cross the review boundary.</p>
        </header>
        <ol>
          {record.lanes.map((lane) => (
            <li key={lane.id}>
              <ReviewLane lane={lane} />
            </li>
          ))}
        </ol>
      </section>

      <section className="agent-review-lab__handoff" aria-labelledby="worktree-handoff-heading">
        <header className="agent-review-lab__section-heading">
          <div>
            <p className="eyebrow">Worktree convergence</p>
            <h2 id="worktree-handoff-heading">Owned instance → bounded evidence</h2>
          </div>
          <p>{record.worktreeHandoff.status}</p>
        </header>
        <div className="agent-review-lab__handoff-flow">
          <ReviewFact label="Application request" copy={record.worktreeHandoff.request} />
          <ReviewFact label="Runtime result" copy={record.worktreeHandoff.instance} />
          <ReviewFact label="Review adapter" copy={record.worktreeHandoff.review} />
        </div>
        <p className="agent-review-lab__handoff-limit">{record.worktreeHandoff.unverified}</p>
      </section>

      <section className="agent-review-lab__boundaries" aria-labelledby="boundaries-heading">
        <header className="agent-review-lab__section-heading">
          <div>
            <p className="eyebrow">Operating boundary</p>
            <h2 id="boundaries-heading">Keep inspection contained</h2>
          </div>
        </header>
        <dl>
          {record.boundaries.map((boundary) => (
            <div key={boundary.label}>
              <dt>{boundary.label}</dt>
              <dd>{boundary.detail}</dd>
            </div>
          ))}
        </dl>
      </section>
    </main>
  );
}

function ReviewLane({ lane }: { readonly lane: AgentReviewLaneRecord }) {
  const verified = lane.status === 'verified';
  const StatusIcon = verified ? CheckCircle2 : CircleDotDashed;
  return (
    <article className={`agent-review-lane agent-review-lane--${lane.status}`}>
      <header className="agent-review-lane__header">
        <span className="agent-review-lane__ordinal" aria-hidden="true">
          {lane.ordinal}
        </span>
        <div>
          <p>
            {lane.scope} · {friendlyEvidenceLane(lane.evidenceLane)}
          </p>
          <h3>{lane.title}</h3>
        </div>
        <span className="agent-review-lane__status">
          <StatusIcon size={15} aria-hidden="true" />
          {lane.statusLabel}
        </span>
      </header>

      <div className="agent-review-lane__flow">
        <ReviewFact label="Request" copy={lane.request} />
        <ReviewFact label="Evidence" copy={lane.evidence} />
        <ReviewFact label="Disposition" annotation={lane.dispositionKind} copy={lane.disposition} />
      </div>

      <dl className="agent-review-lane__metadata">
        {lane.metadata.map((item) => (
          <div key={item.label}>
            <dt>{item.label}</dt>
            <dd>{item.value}</dd>
          </div>
        ))}
      </dl>

      {lane.action && (
        <a className="agent-review-lane__action" href={lane.action.href}>
          {lane.action.label}
          <ArrowUpRight size={16} aria-hidden="true" />
        </a>
      )}

      <details className="agent-review-lane__details">
        <summary>
          <FileCheck2 size={16} aria-hidden="true" />
          Reproduce and inspect evidence
        </summary>
        <div className="agent-review-lane__details-body">
          {lane.reproduction.length > 0 && (
            <div>
              <h4>Reproduce</h4>
              <pre>
                <code>{lane.reproduction.join('\n')}</code>
              </pre>
            </div>
          )}
          {lane.reproductionNote && <p>{lane.reproductionNote}</p>}
          {lane.evidenceFiles.length > 0 && (
            <div>
              <h4>Retained files</h4>
              <ul className="agent-review-lane__files">
                {lane.evidenceFiles.map((file) => (
                  <li key={file.path}>
                    <span>{file.label}</span>
                    <code>{file.path}</code>
                  </li>
                ))}
              </ul>
            </div>
          )}
          <div>
            <h4>Not established</h4>
            <ul className="agent-review-lane__claims">
              {lane.unverifiedClaims.map((claim) => (
                <li key={claim}>{claim}</li>
              ))}
            </ul>
          </div>
        </div>
      </details>
    </article>
  );
}

function ReviewFact({
  label,
  annotation,
  copy,
}: {
  readonly label: string;
  readonly annotation?: string;
  readonly copy: string;
}) {
  return (
    <section>
      <h4>
        {label}
        {annotation && <code>{annotation}</code>}
      </h4>
      <p>{copy}</p>
    </section>
  );
}

function shortRevision(revision: string): string {
  return revision.slice(0, 12);
}

function friendlyEvidenceLane(lane: AgentReviewLaneRecord['evidenceLane']): string {
  return lane.replace('-', ' ');
}
