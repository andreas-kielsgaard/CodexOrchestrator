import { useEffect, useState } from 'react';
import {
  commitTarget,
  type CommitHistoryQuery,
  type GitCommit,
  type GraphConnection,
  type ReviewBranch,
  type ReviewTarget,
  type WorktreeReviewClient,
} from '../../../application/worktreeReview';
import { ReviewDialog } from '../ReviewDialog';
import { useCommitHistory } from '../useCommitHistory';

export function CommitRangeDialog({
  client,
  query,
  source,
  connection,
  onClose,
  onSelect,
  onSelectCommit,
}: {
  readonly client: WorktreeReviewClient;
  readonly query: CommitHistoryQuery;
  readonly source: ReviewBranch;
  readonly connection: GraphConnection;
  readonly onClose: () => void;
  readonly onSelect: (target: ReviewTarget) => void;
  readonly onSelectCommit?: (commit: GitCommit, target: ReviewTarget) => void;
}) {
  const history = useCommitHistory(client, query);
  const [selected, setSelected] = useState<GitCommit>();
  useEffect(() => {
    if (!history.loaded && !history.loading && !history.error) void history.load();
  }, [history]);

  return (
    <ReviewDialog labelledBy="commit-range-title" onClose={onClose} className="commit-range-dialog">
      <header className="branch-graph__header">
        <div>
          <p className="worktree-review__step">{source.displayName}</p>
          <h2 id="commit-range-title">Select a commit</h2>
        </div>
        <button type="button" className="worktree-review__secondary" onClick={onClose}>
          Back to graph
        </button>
      </header>
      <p className="worktree-review__supporting">
        {history.loaded
          ? `${history.total}${connection.incomplete ? '+' : ''} ${history.total === 1 && !connection.incomplete ? 'commit' : 'commits'}`
          : 'Commit range'}{' '}
        ·{' '}
        <code>
          {connection.from.slice(0, 8)} → {connection.to.slice(0, 8)}
        </code>
      </p>
      {connection.incomplete && (
        <p className="worktree-review__supporting">
          Showing loaded history. Return to the graph and load earlier relationships for the
          complete range.
        </p>
      )}
      {history.error && (
        <p role="alert">
          {history.error}{' '}
          <button type="button" onClick={() => void history.load()}>
            Retry
          </button>
        </p>
      )}
      <div className="commit-range-dialog__body">
        <div className="commit-range-dialog__list" role="radiogroup" aria-label="Commits in range">
          {history.commits.map((commit) => (
            <label
              key={commit.objectId}
              className={selected?.objectId === commit.objectId ? 'is-selected' : ''}
            >
              <input
                type="radio"
                name="range-commit"
                checked={selected?.objectId === commit.objectId}
                onChange={() => setSelected(commit)}
              />
              <span>
                <strong>{commit.subject || '(No subject)'}</strong>
                <small>
                  <code>{commit.abbreviatedObjectId}</code> · {commit.author} ·{' '}
                  {new Date(commit.committedAt).toLocaleString()}
                </small>
              </span>
            </label>
          ))}
          {history.loading && <p role="status">Loading commits…</p>}
          {history.hasMore && (
            <button
              type="button"
              className="worktree-review__secondary"
              aria-disabled={history.loading}
              onClick={() => {
                if (!history.loading) void history.loadMore();
              }}
            >
              Load more commits
            </button>
          )}
        </div>
        <aside className="commit-range-dialog__detail" aria-label="Selected commit details">
          {selected ? (
            <>
              <h3>{selected.subject}</h3>
              <code>{selected.objectId}</code>
              <p>{selected.author}</p>
              <p>{new Date(selected.committedAt).toLocaleString()}</p>
              <p>
                This commit will appear in the review page. Build will offer a checkout when needed.
              </p>
            </>
          ) : (
            <p>Select a commit to inspect its details.</p>
          )}
        </aside>
      </div>
      <footer className="worktree-review__actions">
        <button type="button" className="worktree-review__secondary" onClick={onClose}>
          Cancel
        </button>
        <button
          type="button"
          className="worktree-review__primary"
          disabled={!selected}
          onClick={() => {
            if (!selected) return;
            const target = commitTarget(source, selected.objectId);
            onSelectCommit?.(selected, target);
            onSelect(target);
          }}
        >
          Use this commit
        </button>
      </footer>
    </ReviewDialog>
  );
}
