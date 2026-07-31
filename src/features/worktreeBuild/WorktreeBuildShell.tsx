import { ArrowLeft, Trees } from 'lucide-react';
import { useEffect, useState, type ReactNode } from 'react';
import type {
  WorktreeBuildClient,
  WorktreeBuildContext,
  WorktreeBuildDetail,
} from '../../application/worktreeBuild';
import { ApplicationWidget } from '../applicationWidget';
import { FileReviewScreen } from '../fileReview';
import { WorktreeBuildDetailScreen } from './WorktreeBuildDetailScreen';
import './worktreeBuild.css';

export function WorktreeBuildShell({
  client,
  children,
}: {
  readonly client: WorktreeBuildClient;
  readonly children: ReactNode;
}) {
  const [context, setContext] = useState<WorktreeBuildContext | null>(null);
  const [detail, setDetail] = useState<WorktreeBuildDetail | null>(null);
  const [error, setError] = useState('');
  const [surface, setSurface] = useState<'application' | 'details' | 'files'>('application');

  useEffect(() => {
    let active = true;
    void client.context().then(
      (nextContext) => {
        if (!active) return;
        setContext(nextContext);
        requestAnimationFrame(
          () =>
            void client
              .markReady()
              .then(() => client.detail())
              .then(
                (nextDetail) => active && setDetail(nextDetail),
                () =>
                  active &&
                  setError('This worktree build is not ready for review or detail inspection.'),
              ),
        );
      },
      () => active && setError('Worktree build identity and lifecycle details are unavailable.'),
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
      {surface === 'details' && detail && (
        <WorktreeBuildDetailScreen
          detail={detail}
          onBack={() => setSurface('application')}
          onCompare={() => setSurface('files')}
        />
      )}
      {surface === 'files' && context && (
        <section className="worktree-build-route">
          <header className="worktree-build-route__bar">
            <button type="button" onClick={() => setSurface('details')}>
              <ArrowLeft size={16} />
              Build details
            </button>
            <span>Machine main HEAD → complete selected worktree</span>
          </header>
          <FileReviewScreen source={client.comparison} />
        </section>
      )}
      <div className="application-widget-dock">
        <ApplicationWidget
          label="Worktree build"
          title={context?.name ?? 'Loading identity…'}
          summary={
            context
              ? `${context.branch ?? `Detached ${context.head.abbreviatedId}`} · ${context.dirty.dirty ? 'Dirty' : 'Clean'}`
              : 'Reading source identity'
          }
          icon={<Trees size={16} />}
          onOpen={() => setSurface('details')}
        />
      </div>
      {error && (
        <p className="worktree-build-error" role="alert">
          {error}
        </p>
      )}
    </div>
  );
}
