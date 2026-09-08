import {
  buildOutputLabel,
  attemptLabel,
  cleanupLabel,
  sourceLabel,
  type BuildId,
  type ReviewBuild,
} from '../../application/worktreeReview';

export function BuildHistory({
  builds,
  openingBuildId,
  onOpen,
}: {
  readonly builds: readonly ReviewBuild[];
  readonly openingBuildId?: BuildId;
  readonly onOpen: (buildId: BuildId) => void;
}) {
  return (
    <section className="worktree-review__section" aria-labelledby="worktree-review-builds">
      <div className="worktree-review__section-heading">
        <div>
          <p className="worktree-review__step">4 · Retained results</p>
          <h2 id="worktree-review-builds">Builds</h2>
        </div>
        <span className="worktree-review__count">{builds.length}</span>
      </div>
      {builds.length === 0 ? (
        <p className="worktree-review__empty">No builds have been created for this branch.</p>
      ) : (
        <div className="worktree-review__build-list">
          {builds.map((build) => (
            <article className="worktree-review__build" key={build.buildId}>
              <div className="worktree-review__build-title">
                <h3>{build.name}</h3>
                <span className={attemptTone(build)}>{attemptLabel(build.latestAttempt)}</span>
              </div>
              <dl>
                <div>
                  <dt>Branch</dt>
                  <dd>{build.branchRef}</dd>
                </div>
                <div>
                  <dt>Source</dt>
                  <dd>{sourceLabel(build.source)}</dd>
                </div>
                <div>
                  <dt>Worktree checkout</dt>
                  <dd>{build.workspace.locationLabel}</dd>
                </div>
                <div>
                  <dt>Worktree state</dt>
                  <dd>{worktreeStateLabel(build.workspace.lifecycle)}</dd>
                </div>
                <div>
                  <dt>Build output</dt>
                  <dd>{buildOutputLabel(build.output)}</dd>
                </div>
                <div>
                  <dt>AppData retention</dt>
                  <dd>{cleanupLabel(build.cleanup)}</dd>
                </div>
              </dl>
              {build.latestAttempt?.failure && (
                <p className="worktree-review__error-text">
                  {build.latestAttempt.failure.stage}: {build.latestAttempt.failure.summary}
                </p>
              )}
              {build.attention && (
                <p className="worktree-review__attention" role="status">
                  {build.attention.summary}
                </p>
              )}
              {build.output.state === 'available' && (
                <div className="worktree-review__actions">
                  <button
                    type="button"
                    className="worktree-review__primary"
                    disabled={openingBuildId !== undefined}
                    onClick={() => onOpen(build.buildId)}
                  >
                    {openingBuildId === build.buildId ? 'Opening…' : 'Open'}
                  </button>
                </div>
              )}
            </article>
          ))}
        </div>
      )}
    </section>
  );
}

function worktreeStateLabel(lifecycle: ReviewBuild['workspace']['lifecycle']): string {
  switch (lifecycle) {
    case 'ready':
      return 'Ready and retained';
    case 'missing':
      return 'Missing';
    case 'removal_pending':
      return 'Removal pending';
    case 'removed':
      return 'Removed';
    case 'unverified':
      return 'Not verified';
  }
}

function attemptTone(build: ReviewBuild): string {
  if (build.latestAttempt?.outcome === 'succeeded')
    return 'worktree-review__status worktree-review__status--completed';
  if (
    build.latestAttempt?.outcome === 'failed' ||
    build.latestAttempt?.executionState === 'interrupted'
  ) {
    return 'worktree-review__status worktree-review__status--failed';
  }
  return 'worktree-review__status';
}
