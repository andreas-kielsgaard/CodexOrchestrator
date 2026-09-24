import { useEffect, useRef, useState, type Ref } from 'react';
import {
  attemptLabel,
  sourceLabel,
  type BuildId,
  type ReviewBuild,
  type BuildLogChunk,
} from '../../application/worktreeReview';

export function BuildHistory({
  builds,
  openingBuildId,
  onOpen,
  onCreate,
  onRebuild,
  readLog,
  createButtonRef,
}: {
  readonly builds: readonly ReviewBuild[];
  readonly openingBuildId?: BuildId;
  readonly onOpen: (buildId: BuildId) => void;
  readonly onCreate: () => void;
  readonly onRebuild: (build: ReviewBuild) => void;
  readonly readLog: (buildId: BuildId, attemptId: string, offset: number) => Promise<BuildLogChunk>;
  readonly createButtonRef?: Ref<HTMLButtonElement>;
}) {
  const [expanded, setExpanded] = useState<ReadonlySet<string>>(new Set());
  return (
    <section className="worktree-review__section" aria-labelledby="worktree-review-builds">
      <div className="worktree-review__section-heading">
        <div>
          <p className="worktree-review__step">4 · Retained results</p>
          <h2 id="worktree-review-builds">Builds</h2>
        </div>
        <div className="worktree-review__build-heading-actions">
          <span className="worktree-review__count">{builds.length}</span>
          <button
            type="button"
            className="worktree-review__primary"
            ref={createButtonRef}
            onClick={onCreate}
          >
            Create a build
          </button>
        </div>
      </div>
      {builds.length === 0 ? (
        <p className="worktree-review__empty">No builds have been created for this branch.</p>
      ) : (
        <div className="worktree-review__build-list">
          {builds.map((build) => (
            <article className="worktree-review__build" key={build.buildId}>
              <div className="worktree-review__build-title">
                <h3>{build.name}</h3>
                <span className={outputTone(build)}>{outputStatus(build)}</span>
              </div>
              <dl>
                <div>
                  <dt>Branch</dt>
                  <dd>{build.branchRef ?? 'Detached commit'}</dd>
                </div>
                <div>
                  <dt>Initiated</dt>
                  <dd>{new Date(build.initiatedAt).toLocaleString()}</dd>
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
                  <dt>Build mode</dt>
                  <dd>
                    {build.profile === 'release'
                      ? 'Normal'
                      : build.profile === 'debug'
                        ? 'Debugging'
                        : 'Unspecified'}
                  </dd>
                </div>
                <div>
                  <dt>Application instance</dt>
                  <dd>
                    {build.applicationLabel}
                    <small className="worktree-review__application-id">
                      {build.applicationIdentifier}
                    </small>
                  </dd>
                </div>
              </dl>
              {build.latestAttempt?.failure && (
                <p className="worktree-review__error-text">
                  {build.latestAttempt.failure.stage}: {build.latestAttempt.failure.summary}
                </p>
              )}
              {build.latestAttempt && (
                <div className="worktree-review__build-log-action">
                  <button
                    type="button"
                    className="worktree-review__secondary"
                    onClick={() =>
                      setExpanded((current) => {
                        const next = new Set(current);
                        if (next.has(build.buildId)) next.delete(build.buildId);
                        else next.add(build.buildId);
                        return next;
                      })
                    }
                  >
                    {expanded.has(build.buildId) ? 'Hide build log' : 'View build log'}
                  </button>
                  {(expanded.has(build.buildId) ||
                    build.latestAttempt.executionState === 'running') && (
                    <BuildLogPane
                      key={build.latestAttempt.attemptId}
                      build={build}
                      readLog={readLog}
                    />
                  )}
                </div>
              )}
              <div className="worktree-review__actions">
                {isLiveBuild(build) && (
                  <button
                    type="button"
                    className="worktree-review__secondary"
                    onClick={() => onRebuild(build)}
                  >
                    Rebuild
                  </button>
                )}
                {build.output.state === 'available' && (
                  <button
                    type="button"
                    className="worktree-review__primary"
                    disabled={openingBuildId !== undefined}
                    onClick={() => onOpen(build.buildId)}
                  >
                    {openingBuildId === build.buildId ? 'Launching…' : 'Launch'}
                  </button>
                )}
              </div>
            </article>
          ))}
        </div>
      )}
    </section>
  );
}

const stageLabels: Readonly<Record<string, string>> = {
  dependencies: 'Installing dependencies',
  typescript: 'Checking TypeScript',
  frontend: 'Building frontend',
  cargo: 'Compiling Tauri/Cargo',
  publication: 'Retaining application output',
};

function BuildLogPane({
  build,
  readLog,
}: {
  readonly build: ReviewBuild;
  readonly readLog: (buildId: BuildId, attemptId: string, offset: number) => Promise<BuildLogChunk>;
}) {
  const attempt = build.latestAttempt!;
  const [text, setText] = useState('');
  const offset = useRef(0);
  const [stage, setStage] = useState(attempt.stage);
  const [error, setError] = useState<string | null>(null);
  const [clock, setClock] = useState(Date.now());
  const running = attempt.executionState === 'running' || attempt.executionState === 'pending';
  useEffect(() => {
    let active = true;
    const poll = () => {
      void readLog(build.buildId, attempt.attemptId, offset.current)
        .then((chunk) => {
          if (!active) return;
          if (chunk.truncatedBefore) setText('(Earlier output omitted)\n');
          if (chunk.text) {
            setText((current) => `${current}${chunk.text}`.slice(-100_000));
            const stages = [...chunk.text.matchAll(/@@ORCHID_STAGE:([a-z]+)/g)];
            const last = stages.at(-1)?.[1];
            if (last) setStage(stageLabels[last] ?? last);
          }
          offset.current = chunk.nextOffset;
          setClock(Date.now());
        })
        .catch((cause) => {
          if (active) setError(String(cause));
        });
    };
    poll();
    if (running) {
      const timer = window.setInterval(poll, 1200);
      return () => {
        active = false;
        window.clearInterval(timer);
      };
    }
    return () => {
      active = false;
    };
  }, [readLog, build.buildId, attempt.attemptId, running]);
  const elapsed = Math.max(0, Math.floor((clock - Date.parse(attempt.startedAt)) / 1000));
  return (
    <div className="worktree-review__build-log">
      <p role="status">
        {running ? stage : attemptLabel(attempt)} · elapsed {Math.floor(elapsed / 60)}m{' '}
        {elapsed % 60}s · time remaining unknown
      </p>
      {error && <p role="alert">Could not read build log: {error}</p>}
      <pre aria-label={`Build log for ${build.name}`}>
        {text || (running ? 'Waiting for build output…' : 'No retained build log.')}
      </pre>
    </div>
  );
}

function outputStatus(build: ReviewBuild): string {
  if (build.latestAttempt?.executionState === 'running' || build.latestAttempt?.executionState === 'pending')
    return 'Building';
  if (build.latestAttempt?.outcome === 'failed' || build.latestAttempt?.executionState === 'interrupted')
    return 'Build failed';
  return build.output.state === 'available' ? 'Available' : 'No longer available';
}

function outputTone(build: ReviewBuild): string {
  if (build.output.state === 'available')
    return 'worktree-review__status worktree-review__status--completed';
  if (
    build.latestAttempt?.outcome === 'failed' ||
    build.latestAttempt?.executionState === 'interrupted'
  ) {
    return 'worktree-review__status worktree-review__status--failed';
  }
  return 'worktree-review__status';
}

function isLiveBuild(build: ReviewBuild): boolean {
  return (
    build.source.kind === 'existing_worktree' ||
    (build.source.kind === 'physical_worktree' && !build.source.snapshot)
  );
}
