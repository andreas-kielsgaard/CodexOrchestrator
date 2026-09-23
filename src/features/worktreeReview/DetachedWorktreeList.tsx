import type { DetachedWorktreeDetail } from '../../application/worktreeReview';

export function DetachedWorktreeList({
  worktrees,
  loading,
  error,
}: {
  readonly worktrees: readonly DetachedWorktreeDetail[];
  readonly loading: boolean;
  readonly error: string | null;
}) {
  return (
    <section className="worktree-review__section" aria-label="Detached worktree details">
      <div className="worktree-review__section-heading">
        <div>
          <p className="worktree-review__step">Repository inspection</p>
          <h2>Detached worktrees</h2>
        </div>
      </div>
      <p className="worktree-review__supporting">
        These checkouts have no attached Git branch. Recorded branch associations are shown
        separately; a detached HEAD does not by itself mean a checkout is unused.
      </p>
      {loading && <p role="status">Inspecting detached worktrees and disk usage…</p>}
      {error && <p role="alert">Could not inspect detached worktrees: {error}</p>}
      {!loading && !error && worktrees.length === 0 && <p>No detached worktrees were found.</p>}
      <div className="worktree-review__detached-list">
        {worktrees.map((worktree) => {
          const changed = worktree.stagedFiles + worktree.unstagedFiles + worktree.untrackedFiles;
          return (
            <article key={worktree.worktreeId} className="worktree-review__worktree">
              <div className="worktree-review__worktree-content">
                <strong>
                  {worktree.locationLabel.split(/[\\/]/).at(-1) || worktree.locationLabel}
                </strong>
                <code>{worktree.locationLabel}</code>
                <span>
                  HEAD {worktree.head.abbreviatedObjectId} · {worktree.head.subject}
                </span>
                <span>
                  {changed
                    ? `Dirty: ${worktree.stagedFiles} staged, ${worktree.unstagedFiles} unstaged, ${worktree.untrackedFiles} untracked`
                    : 'Clean worktree'}
                </span>
                <span>
                  Last activity:{' '}
                  {worktree.lastActivity
                    ? `${new Date(worktree.lastActivity.changedAt).toLocaleString()} (approximate)`
                    : 'Unknown'}
                </span>
                <span>
                  Recorded branch association:{' '}
                  {worktree.recordedBranches.length
                    ? worktree.recordedBranches.join(', ')
                    : 'None recorded'}
                </span>
                <span>
                  First recorded by Orchid:{' '}
                  {worktree.firstRecordedAt
                    ? new Date(worktree.firstRecordedAt).toLocaleString()
                    : 'Unknown'}
                </span>
                <span>
                  Disk size:{' '}
                  {worktree.diskSizeBytes === null
                    ? 'Unavailable'
                    : `${worktree.diskSizeLimited ? 'At least ' : 'Approximately '}${(worktree.diskSizeBytes / (1024 * 1024)).toFixed(1)} MiB`}
                </span>
              </div>
            </article>
          );
        })}
      </div>
    </section>
  );
}
