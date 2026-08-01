import {
  ArrowLeft,
  Box,
  CircleAlert,
  FileText,
  GitBranch,
  GitCommitHorizontal,
  GitCompareArrows,
  HeartPulse,
  History,
  PackageCheck,
  Trees,
} from 'lucide-react';
import { useEffect, useRef } from 'react';
import type { WorktreeBuildDetail } from '../../application/worktreeBuild';
import './worktreeBuild.css';

export function WorktreeBuildDetailScreen({
  detail,
  onBack,
  onCompare,
  expandedOperationRef,
}: {
  readonly detail: WorktreeBuildDetail;
  readonly onBack: () => void;
  readonly onCompare: () => void;
  readonly expandedOperationRef?: string;
}) {
  const context = detail.context;
  const expandedOperation = useRef<HTMLDetailsElement | null>(null);
  useEffect(() => {
    expandedOperation.current?.scrollIntoView?.({ block: 'center' });
  }, [expandedOperationRef, detail]);
  return (
    <main className="worktree-build-detail" aria-label="Worktree build details">
      <header>
        <button type="button" onClick={onBack}>
          <ArrowLeft size={16} />
          Back
        </button>
        <div>
          <p className="eyebrow">Worktree build</p>
          <h1>{detail.name}</h1>
          <p>{detail.orientation}</p>
        </div>
        <button type="button" className="worktree-build-detail__compare" onClick={onCompare}>
          <GitCompareArrows size={16} />
          Review files and changes
        </button>
      </header>

      <section className="worktree-build-detail__orientation" aria-label="Build orientation">
        <article>
          <Box size={20} />
          <div>
            <span>Why it exists</span>
            <strong>{detail.purpose}</strong>
            <p>{detail.sourceLabel}</p>
          </div>
        </article>
        <article>
          <PackageCheck size={20} />
          <div>
            <span>Prepare, Build, Open</span>
            <strong>{detail.prepareProduced}</strong>
            <p>{detail.buildProduced}</p>
            <p>{detail.openProduced}</p>
          </div>
        </article>
        <article>
          <HeartPulse size={20} />
          <div>
            <span>Current condition</span>
            <strong>{detail.currentCondition}</strong>
            <p>{detail.actionSummary}</p>
            <p>{detail.reusableSummary}</p>
          </div>
        </article>
        <article>
          <CircleAlert size={20} />
          <div>
            <span>Retention and cleanup</span>
            <strong>{detail.retention.policy}</strong>
            <p>{detail.retention.cleanup}</p>
            <p>{detail.retention.automatic ? 'Cleanup is automatic.' : 'Cleanup is manual.'}</p>
          </div>
        </article>
      </section>

      <section className="worktree-build-detail__facts" aria-label="Source identity">
        <article>
          <GitBranch size={20} />
          <div>
            <span>Selected worktree</span>
            <strong>{context.branch ?? 'Detached HEAD'}</strong>
            <p>
              {context.dirty.dirty
                ? `${context.dirty.staged} staged · ${context.dirty.unstaged} unstaged · ${context.dirty.untracked} untracked`
                : 'Clean working tree'}
            </p>
          </div>
        </article>
        <article>
          <GitCommitHorizontal size={20} />
          <div>
            <span>Current HEAD</span>
            <strong>{context.head.abbreviatedId}</strong>
            <p>{context.head.message}</p>
            <time>{formatTime(context.head.committedAt)}</time>
          </div>
        </article>
        <article>
          <Trees size={20} />
          <div>
            <span>Machine main</span>
            <strong>
              {context.main.branch ?? 'Detached HEAD'} · {context.main.head.abbreviatedId}
            </strong>
            <p>{context.main.head.message}</p>
            <p>
              {context.main.dirty.dirty
                ? 'Main has uncommitted changes; they are reported but not used as the comparison base.'
                : 'Main checkout is clean.'}
            </p>
          </div>
        </article>
        <article>
          <GitCompareArrows size={20} />
          <div>
            <span>Relationship</span>
            <strong>{context.relationship.summary}</strong>
            <p>
              Merge base {context.relationship.mergeBase?.slice(0, 12) ?? 'unavailable'} is used
              only for ahead/behind history, not as the file comparison base.
            </p>
          </div>
        </article>
      </section>

      <section className="worktree-build-detail__section">
        <h2>Generated material</h2>
        <div className="worktree-build-detail__artifacts">
          {detail.artifacts.map((artifact) => (
            <article key={artifact.label}>
              <FileText size={17} />
              <div>
                <strong>{artifact.label}</strong>
                <span>{artifact.state.replace('-', ' ')}</span>
                <p>{artifact.summary}</p>
              </div>
            </article>
          ))}
        </div>
      </section>

      <section className="worktree-build-detail__section">
        <h2>Lifecycle and build history</h2>
        {detail.lifecycleHistory.length ? (
          <ol className="worktree-build-detail__history">
            {detail.lifecycleHistory.map((event, index) => (
              <li key={`${event.occurredAtMs}-${event.kind}-${index}`}>
                <History size={16} />
                <div>
                  <strong>{event.kind}</strong>
                  <span>{event.summary}</span>
                </div>
                <time>{formatTimestamp(event.occurredAtMs)}</time>
              </li>
            ))}
          </ol>
        ) : (
          <p>No retained lifecycle events are available for this pre-history instance.</p>
        )}
      </section>

      <section className="worktree-build-detail__section">
        <h2>Operation and build output</h2>
        <p>
          Complete sanitized output retained for this instance is shown below. Secrets, private
          roots, ports, process identities, and command lines remain hidden.
        </p>
        {detail.operations.length ? (
          detail.operations.map((operation) => (
            <details
              key={operation.operationRef}
              open={
                operation.state === 'pending' || operation.operationRef === expandedOperationRef
              }
              ref={(element) => {
                if (operation.operationRef === expandedOperationRef) {
                  expandedOperation.current = element;
                }
              }}
            >
              <summary>
                {operation.operation} · {operation.stageLabel} · {operation.state}
              </summary>
              <p>
                {formatTimestamp(operation.startedAtMs)} · {operation.output.length} safe lines
                {!operation.outputComplete && ' · oldest output trimmed by the safety bound'}
              </p>
              <pre>
                {operation.output.length ? operation.output.join('\n') : 'No safe output recorded.'}
              </pre>
            </details>
          ))
        ) : (
          <p>No operation output has been retained yet.</p>
        )}
      </section>

      <section className="worktree-build-detail__section">
        <h2>Branch history and comparison semantics</h2>
        <p>{context.comparisonBasis}</p>
        {context.relatedBranches.length > 0 && (
          <div className="worktree-build-detail__related">
            <strong>Nearest local branch relationships</strong>
            <ul>
              {context.relatedBranches.map((branch) => (
                <li key={branch.name}>
                  <code>{branch.name}</code>
                  <span>{branch.summary}</span>
                </li>
              ))}
            </ul>
          </div>
        )}
        {context.history.length ? (
          <ol className="worktree-build-detail__commits">
            {context.history.map((commit) => (
              <li key={commit.id}>
                <code>{commit.abbreviatedId}</code>
                <div>
                  <strong>{commit.message}</strong>
                  <time>{formatTime(commit.committedAt)}</time>
                </div>
              </li>
            ))}
          </ol>
        ) : (
          <p>No selected-worktree commits are ahead of machine main HEAD.</p>
        )}
      </section>
    </main>
  );
}

function formatTime(value: string) {
  const time = new Date(value);
  return Number.isNaN(time.valueOf()) ? value : time.toLocaleString();
}

function formatTimestamp(value: number) {
  return value > 0 ? new Date(value).toLocaleString() : 'Time unavailable';
}
