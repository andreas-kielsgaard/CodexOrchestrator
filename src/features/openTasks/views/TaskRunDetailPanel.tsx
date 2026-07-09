import {
  Activity,
  AlertCircle,
  FileText,
  RefreshCw,
  Route,
  ScrollText,
  ShieldCheck,
  X,
  type LucideIcon,
} from 'lucide-react';
import type { ReactNode } from 'react';
import type {
  ArtifactBucketViewModel,
  DetailTermViewModel,
  EventTimelineItemViewModel,
  RunDetailCardViewModel,
  TaskRunDetailPanelViewModel,
  ValidationRowViewModel,
} from '../viewModels/taskDetailViewModel';

export interface TaskRunDetailPanelProps {
  detail: TaskRunDetailPanelViewModel;
  onClose(): void;
  onReload(): void;
}

export function TaskRunDetailPanel({ detail, onClose, onReload }: TaskRunDetailPanelProps) {
  return (
    <aside className="detail-panel" aria-label="Task run detail">
      <header className="detail-header">
        <div>
          <p className="eyebrow">Task detail</p>
          <h2>{detail.title}</h2>
        </div>
        <div className="detail-actions">
          <button
            className="icon-button"
            type="button"
            onClick={onReload}
            disabled={detail.reloadDisabled}
            title="Reload detail"
            aria-label="Reload task detail"
          >
            <RefreshCw size={16} aria-hidden="true" />
          </button>
          <button
            className="icon-button"
            type="button"
            onClick={onClose}
            disabled={detail.closeDisabled}
            title="Close detail"
            aria-label="Close task detail"
          >
            <X size={16} aria-hidden="true" />
          </button>
        </div>
      </header>

      {detail.status === 'idle' && <p className="detail-empty">Select a task to inspect.</p>}

      {detail.status === 'loading' && (
        <p className="detail-state" role="status">
          Loading task detail...
        </p>
      )}

      {detail.status === 'failed' && (
        <section className="notice error detail-notice" role="status">
          <AlertCircle size={18} aria-hidden="true" />
          <span>{detail.error}</span>
        </section>
      )}

      {detail.status === 'loaded' && (
        <div className="detail-content">
          <DetailSection title="Anchors" icon={Route}>
            <dl className="anchor-grid">
              {detail.anchors.map((anchor) => (
                <DetailTerm key={anchor.label} term={anchor} />
              ))}
            </dl>
          </DetailSection>

          <DetailSection title="Runs" icon={Activity}>
            {detail.runs.length === 0 ? (
              <p className="detail-empty">No runs recorded.</p>
            ) : (
              <div className="run-history">
                {detail.runs.map((run) => (
                  <RunDetailCard key={run.id} run={run} />
                ))}
              </div>
            )}
          </DetailSection>

          <DetailSection title="Task Artifacts" icon={FileText}>
            <ArtifactBucketSummary buckets={detail.unlinkedArtifacts} />
            <ValidationList validationRuns={detail.unlinkedValidations} />
          </DetailSection>

          <DetailSection title="Timeline" icon={ScrollText}>
            {detail.timeline.length === 0 ? (
              <p className="detail-empty">No events recorded.</p>
            ) : (
              <ol className="event-timeline">
                {detail.timeline.map((event) => (
                  <EventTimelineItem key={event.id} event={event} />
                ))}
              </ol>
            )}
          </DetailSection>
        </div>
      )}
    </aside>
  );
}

interface DetailSectionProps {
  title: string;
  icon: LucideIcon;
  children: ReactNode;
}

function DetailSection({ title, icon: Icon, children }: DetailSectionProps) {
  return (
    <section className="detail-section">
      <header>
        <Icon size={16} aria-hidden="true" />
        <h3>{title}</h3>
      </header>
      {children}
    </section>
  );
}

interface DetailTermProps {
  term: DetailTermViewModel;
}

function DetailTerm({ term }: DetailTermProps) {
  return (
    <div>
      <dt>{term.label}</dt>
      <dd title={term.title}>{term.value}</dd>
    </div>
  );
}

interface RunDetailCardProps {
  run: RunDetailCardViewModel;
}

function RunDetailCard({ run }: RunDetailCardProps) {
  return (
    <article className="run-detail-card">
      <header>
        <div>
          <h4>{run.id}</h4>
          <p>{run.timestampLabel}</p>
        </div>
        <span className={`state-pill ${run.executionState}`}>{run.executionState}</span>
      </header>
      <div className="metric-row">
        {run.metrics.map((metric) => (
          <span key={metric}>{metric}</span>
        ))}
      </div>
      <ArtifactBucketSummary buckets={run.artifacts} />
      <ValidationList validationRuns={run.validations} />
      {run.recentEvents.length > 0 && (
        <ol className="mini-events">
          {run.recentEvents.map((event) => (
            <EventTimelineItem key={event.id} event={event} />
          ))}
        </ol>
      )}
    </article>
  );
}

interface ArtifactBucketSummaryProps {
  buckets: ArtifactBucketViewModel[];
}

function ArtifactBucketSummary({ buckets }: ArtifactBucketSummaryProps) {
  if (buckets.length === 0) {
    return <p className="detail-empty">No artifacts recorded.</p>;
  }

  return (
    <div className="artifact-buckets">
      {buckets.map((bucket) => (
        <div className="artifact-bucket" key={bucket.label}>
          <strong>
            {bucket.label}
            <span>{bucket.count}</span>
          </strong>
          <ul>
            {bucket.artifacts.map((artifact) => (
              <li key={artifact.id}>
                <span title={artifact.title}>{artifact.title}</span>
                <small>{artifact.preview}</small>
              </li>
            ))}
          </ul>
        </div>
      ))}
    </div>
  );
}

interface ValidationListProps {
  validationRuns: ValidationRowViewModel[];
}

function ValidationList({ validationRuns }: ValidationListProps) {
  if (validationRuns.length === 0) {
    return null;
  }

  return (
    <div className="validation-list">
      {validationRuns.map((run) => (
        <div className="validation-row" key={run.id}>
          <ShieldCheck size={15} aria-hidden="true" />
          <span>{run.command}</span>
          <strong>{run.status}</strong>
          {run.exitCodeLabel && <small>{run.exitCodeLabel}</small>}
          {run.outputArtifactTitle && (
            <small title={run.outputArtifactTitle}>{run.outputArtifactTitle}</small>
          )}
        </div>
      ))}
    </div>
  );
}

interface EventTimelineItemProps {
  event: EventTimelineItemViewModel;
}

function EventTimelineItem({ event }: EventTimelineItemProps) {
  return (
    <li>
      <time dateTime={event.occurredAt}>{event.occurredAtLabel}</time>
      <span>{event.kind}</span>
      {event.summary && <small>{event.summary}</small>}
    </li>
  );
}
