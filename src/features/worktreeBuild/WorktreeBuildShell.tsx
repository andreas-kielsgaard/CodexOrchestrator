import { ArrowLeft, GitBranch, GitCommitHorizontal, GitCompareArrows, Trees } from 'lucide-react';
import { useEffect, useState, type ReactNode } from 'react';
import type { WorktreeBuildClient, WorktreeBuildContext } from '../../application/worktreeBuild';
import { FileReviewScreen } from '../fileReview';
import './worktreeBuild.css';

export function WorktreeBuildShell({
  client,
  children,
}: {
  readonly client: WorktreeBuildClient;
  readonly children: ReactNode;
}) {
  const [context, setContext] = useState<WorktreeBuildContext | null>(null);
  const [error, setError] = useState('');
  const [surface, setSurface] = useState<'application' | 'details' | 'files'>('application');

  useEffect(() => {
    let active = true;
    void client.context().then(
      (value) => {
        if (!active) return;
        setContext(value);
        requestAnimationFrame(
          () =>
            void client
              .markReady()
              .catch(() => setError('This worktree build is not ready for review.')),
        );
      },
      () => active && setError('Worktree identity is unavailable.'),
    );
    return () => {
      active = false;
    };
  }, [client]);

  useEffect(() => {
    let active = true;
    let lastSequence = '';
    const read = () =>
      void client.proofNavigation().then(
        (navigation) => {
          if (!active || !navigation || navigation.sequence === lastSequence) return;
          lastSequence = navigation.sequence;
          setSurface(
            navigation.route === 'worktree-details'
              ? 'details'
              : navigation.route === 'file-review'
                ? 'files'
                : 'application',
          );
        },
        () => undefined,
      );
    read();
    const timer = window.setInterval(read, 300);
    return () => {
      active = false;
      window.clearInterval(timer);
    };
  }, [client]);

  return (
    <div className="worktree-build-shell">
      <div
        className={
          surface === 'application'
            ? 'worktree-build-shell__application'
            : 'worktree-build-shell__application worktree-build-shell__application--hidden'
        }
      >
        {children}
      </div>
      {surface === 'details' && context && (
        <Details
          context={context}
          onBack={() => setSurface('application')}
          onCompare={() => setSurface('files')}
        />
      )}
      {surface === 'files' && context && (
        <section className="worktree-build-route">
          <header className="worktree-build-route__bar">
            <button type="button" onClick={() => setSurface('details')}>
              <ArrowLeft size={16} />
              Worktree details
            </button>
            <span>Machine main HEAD → complete selected worktree</span>
          </header>
          <FileReviewScreen source={client.comparison} />
        </section>
      )}
      <button
        type="button"
        className="worktree-build-indicator"
        onClick={() => setSurface('details')}
        aria-label={`Open Worktree details for ${context?.name ?? 'this build'}`}
      >
        <Trees size={15} />
        <span>Worktree build</span>
        <strong>{context?.name ?? 'Loading identity…'}</strong>
        {context && (
          <small>
            {context.branch ?? `Detached ${context.head.abbreviatedId}`} ·{' '}
            {context.dirty.dirty ? 'Dirty' : 'Clean'}
          </small>
        )}
      </button>
      {error && (
        <p className="worktree-build-error" role="alert">
          {error}
        </p>
      )}
    </div>
  );
}

function Details({
  context,
  onBack,
  onCompare,
}: {
  readonly context: WorktreeBuildContext;
  readonly onBack: () => void;
  readonly onCompare: () => void;
}) {
  return (
    <main className="worktree-details" aria-label="Worktree details">
      <header>
        <button type="button" onClick={onBack}>
          <ArrowLeft size={16} />
          Application
        </button>
        <div>
          <p className="eyebrow">Worktree build</p>
          <h1>{context.name}</h1>
          <p>Verify exactly which source and state this separate application window is running.</p>
        </div>
        <button type="button" className="worktree-details__compare" onClick={onCompare}>
          <GitCompareArrows size={16} />
          Review files and changes
        </button>
      </header>
      <section className="worktree-details__facts">
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
                ? 'Main checkout also has uncommitted changes; those are not the comparison base.'
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
      <section className="worktree-details__basis">
        <h2>Comparison semantics</h2>
        <p>{context.comparisonBasis}</p>
      </section>
      <section className="worktree-details__history">
        <h2>Selected commits not in machine main HEAD</h2>
        {context.history.length ? (
          <ol>
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
