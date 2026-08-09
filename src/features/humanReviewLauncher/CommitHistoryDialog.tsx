import { Files, GitBranch, Minus, Plus, X } from 'lucide-react';
import { Fragment, useEffect, useMemo, useRef, useState } from 'react';
import type {
  HumanReviewCommit,
  HumanReviewSourceHistory,
} from '../../application/humanReviewLauncher';

export function CommitHistoryDialog({
  history,
  onClose,
}: {
  readonly history: HumanReviewSourceHistory;
  readonly onClose: () => void;
}) {
  const [selectedCommitId, setSelectedCommitId] = useState(history.commits[0]?.id ?? '');
  const closeButton = useRef<HTMLButtonElement>(null);
  const selected = history.commits.find((commit) => commit.id === selectedCommitId);
  const markers = useMemo(
    () => new Map(history.lineageMarkers.map((marker) => [marker.commitId, marker])),
    [history.lineageMarkers],
  );

  useEffect(() => {
    closeButton.current?.focus();
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') onClose();
    };
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, [onClose]);

  return (
    <div
      className="commit-history__backdrop"
      onMouseDown={(event) => event.target === event.currentTarget && onClose()}
    >
      <section
        className="commit-history"
        role="dialog"
        aria-modal="true"
        aria-labelledby="commit-history-title"
      >
        <header className="commit-history__header">
          <div>
            <p>Commit history</p>
            <h2 id="commit-history-title">{history.branch}</h2>
            <span>
              {history.commitCount} {history.commitCount === 1 ? 'commit' : 'commits'} since main ·
              fork {history.forkRevision}
            </span>
          </div>
          <button ref={closeButton} type="button" onClick={onClose}>
            <X size={16} />
            Close history
          </button>
        </header>

        <div className="commit-history__columns">
          <section className="commit-history__list" aria-label="Commits, newest first">
            <header>
              <h3>Commits</h3>
              <span>Newest first</span>
            </header>
            {history.commits.length === 0 ? (
              <p className="commit-history__empty">This branch has no commits beyond main.</p>
            ) : (
              <div className="commit-history__rail">
                {history.commits.map((commit) => {
                  const marker = markers.get(commit.id);
                  return (
                    <Fragment key={commit.id}>
                      <button
                        type="button"
                        className={commit.id === selectedCommitId ? 'is-selected' : undefined}
                        aria-pressed={commit.id === selectedCommitId}
                        onClick={() => setSelectedCommitId(commit.id)}
                      >
                        <span className="commit-history__dot" aria-hidden="true" />
                        <span>
                          <strong>{commit.subject}</strong>
                          <small>
                            {commit.abbreviatedId} · {commit.author} ·{' '}
                            {formatDate(commit.committedAt)}
                          </small>
                        </span>
                      </button>
                      {marker && (
                        <div className="commit-history__lineage">
                          <GitBranch size={14} />
                          Branched from <strong>{marker.branch}</strong> at {marker.abbreviatedId}
                        </div>
                      )}
                    </Fragment>
                  );
                })}
              </div>
            )}
          </section>

          <CommitDetails commit={selected} />
        </div>
      </section>
    </div>
  );
}

function CommitDetails({ commit }: { readonly commit?: HumanReviewCommit }) {
  if (!commit) {
    return (
      <section className="commit-history__details" aria-label="Commit details">
        <p className="commit-history__empty">Select a commit to inspect its details.</p>
      </section>
    );
  }
  return (
    <section className="commit-history__details" aria-label="Commit details">
      <header>
        <p>Selected commit</p>
        <h3>{commit.subject}</h3>
        <span>
          {commit.abbreviatedId} · {commit.author} · {formatDate(commit.committedAt)}
        </span>
      </header>
      <div className="commit-history__description">
        <h4>Description</h4>
        <p>{commit.description || 'No commit description.'}</p>
      </div>
      <div className="commit-history__summary">
        <h4>Change summary</h4>
        <div>
          <span>
            <Files size={17} />
            <strong>{commit.filesChanged}</strong> files changed
          </span>
          <span className="is-added">
            <Plus size={17} />
            <strong>{commit.insertions}</strong> lines
          </span>
          <span className="is-removed">
            <Minus size={17} />
            <strong>{commit.deletions}</strong> lines
          </span>
        </div>
        <p>Changed-file details are not implemented yet.</p>
      </div>
    </section>
  );
}

function formatDate(value: string) {
  const date = new Date(value);
  return Number.isNaN(date.getTime())
    ? value
    : new Intl.DateTimeFormat(undefined, {
        month: 'short',
        day: 'numeric',
        year: 'numeric',
      }).format(date);
}
