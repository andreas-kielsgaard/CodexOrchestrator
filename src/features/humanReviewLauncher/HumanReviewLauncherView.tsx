import { ArrowLeft } from 'lucide-react';
import { useCallback, useEffect, useRef, useState } from 'react';
import type {
  HumanReviewInstance,
  HumanReviewLauncherClient,
  HumanReviewOperationProgress,
  HumanReviewSource,
  HumanReviewSourceHistory,
} from '../../application/humanReviewLauncher';
import type { WorktreeBuildDetail } from '../../application/worktreeBuild';
import { FileReviewScreen } from '../fileReview';
import { WorktreeBuildDetailScreen } from '../worktreeBuild';
import { CommitHistoryDialog } from './CommitHistoryDialog';
import { RepositoryHistoryDialog } from './RepositoryHistoryDialog';
import { WorktreeSourcePicker } from './WorktreeSourcePicker';
import './humanReviewLauncher.css';

export function HumanReviewLauncherView({
  client,
}: {
  readonly client: HumanReviewLauncherClient;
}) {
  const [sources, setSources] = useState<readonly HumanReviewSource[]>([]);
  const [instances, setInstances] = useState<readonly HumanReviewInstance[]>([]);
  const [sourceRef, setSourceRef] = useState('');
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [progress, setProgress] = useState<Record<string, HumanReviewOperationProgress>>({});
  const [ownedProgress, setOwnedProgress] = useState<HumanReviewOperationProgress | null>(null);
  const [detail, setDetail] = useState<WorktreeBuildDetail | null>(null);
  const [surface, setSurface] = useState<'overview' | 'details' | 'files'>('overview');
  const [expandedOperationRef, setExpandedOperationRef] = useState<string | undefined>();
  const [history, setHistory] = useState<HumanReviewSourceHistory | null>(null);
  const [historyBusy, setHistoryBusy] = useState(false);
  const [selectedInstanceRef, setSelectedInstanceRef] = useState('');
  const [repositorySources, setRepositorySources] = useState<readonly HumanReviewSource[]>([]);
  const [explorerOpen, setExplorerOpen] = useState(false);
  const [explorerLoading, setExplorerLoading] = useState(false);
  const [sourcesLoading, setSourcesLoading] = useState(true);
  const [instancesLoading, setInstancesLoading] = useState(true);
  const historyRequest = useRef(0);
  const historyTrigger = useRef<HTMLElement | null>(null);

  const applySources = useCallback((nextSources: readonly HumanReviewSource[]) => {
    const attachedSources = nextSources.filter((source) => source.attached && !source.detached);
    setSources(attachedSources);
    setSourceRef((current) =>
      attachedSources.some((source) => source.sourceRef === current)
        ? current
        : attachedSources.find((source) => !source.detached)?.sourceRef || '',
    );
  }, []);

  const loadSources = useCallback(
    async (includeDetached: boolean, refresh = false) => {
      setSourcesLoading(true);
      setError(null);
      try {
        applySources(await client.listSources({ includeDetached, refresh }));
      } catch (cause) {
        setError(message(cause));
      } finally {
        setSourcesLoading(false);
      }
    },
    [applySources, client],
  );

  const loadInstances = useCallback(async () => {
    setInstancesLoading(true);
    try {
      setInstances(await client.listInstances());
    } catch (cause) {
      setError(message(cause));
    } finally {
      setInstancesLoading(false);
    }
  }, [client]);

  const load = useCallback(() => {
    void loadSources(false, true);
    void loadInstances();
  }, [loadInstances, loadSources]);

  useEffect(() => {
    if (!client.proofPresentation) return;
    let active = true;
    let lastSequence = '';
    const read = () =>
      void client.proofPresentation!().then(
        (presentation) => {
          if (!active || !presentation || presentation.sequence === lastSequence) return;
          lastSequence = presentation.sequence;
          if (presentation.sourceRef) setSourceRef(presentation.sourceRef);
          if (presentation.route === 'overview') {
            setExpandedOperationRef(undefined);
            setSurface('overview');
            return;
          }
          if (!presentation.instanceRef) return;
          setBusy(`detail:${presentation.instanceRef}`);
          void client.detail(presentation.instanceRef).then(
            (value) => {
              if (!active) return;
              setDetail(value);
              setExpandedOperationRef(presentation.operationRef);
              setSurface('details');
              setBusy(null);
            },
            (cause) => {
              if (!active) return;
              setError(message(cause));
              setBusy(null);
            },
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

  useEffect(() => {
    void loadSources(false);
    void loadInstances();
  }, [loadInstances, loadSources]);

  useEffect(() => {
    const enriching = sources.some(
      (source) =>
        !source.detached &&
        (source.detailsState === 'pending' || source.detailsState === 'cached'),
    );
    if (!enriching) return;
    let active = true;
    const read = () =>
      void client.listSources({ includeDetached: false }).then(
        (value) => active && applySources(value),
        () => undefined,
      );
    const timer = window.setInterval(read, 300);
    return () => {
      active = false;
      window.clearInterval(timer);
    };
  }, [applySources, client, sources]);

  useEffect(() => {
    setSelectedInstanceRef((current) => {
      const available = instances.filter((instance) => instance.sourceRef === sourceRef);
      return available.some((instance) => instance.instanceRef === current)
        ? current
        : preferredRetainedBuild(available)?.instanceRef || '';
    });
  }, [instances, sourceRef]);

  useEffect(() => {
    let active = true;
    let lastTerminal = '';
    const refresh = () =>
      void client.listProgress().then(
        (operations) => {
          if (!active) return;
          const latest = operations[0] ?? null;
          setOwnedProgress(latest);
          if (latest && latest.state !== 'pending' && latest.operationRef !== lastTerminal) {
            lastTerminal = latest.operationRef;
            void client.listInstances().then((value) => active && setInstances(value));
          }
        },
        () => undefined,
      );
    refresh();
    const timer = window.setInterval(refresh, 500);
    return () => {
      active = false;
      window.clearInterval(timer);
    };
  }, [client]);

  useEffect(() => {
    if (!client.proofDetailNavigation) return;
    let active = true;
    let lastSequence = '';
    const read = () =>
      void client.proofDetailNavigation!().then(
        (navigation) => {
          if (!active || !navigation || navigation.sequence === lastSequence) return;
          lastSequence = navigation.sequence;
          setBusy(`detail:${navigation.instanceRef}`);
          void client.detail(navigation.instanceRef).then(
            (value) => {
              if (!active) return;
              setDetail(value);
              setSurface('details');
              setBusy(null);
            },
            (cause) => {
              if (!active) return;
              setError(message(cause));
              setBusy(null);
            },
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

  const update = useCallback((instance: HumanReviewInstance) => {
    setInstances((current) => [
      instance,
      ...current.filter((item) => item.instanceRef !== instance.instanceRef),
    ]);
  }, []);

  async function openDetail(instanceRef: string) {
    setBusy(`detail:${instanceRef}`);
    setError(null);
    try {
      setDetail(await client.detail(instanceRef));
      setExpandedOperationRef(undefined);
      setSurface('details');
    } catch (cause) {
      setError(message(cause));
    } finally {
      setBusy(null);
    }
  }

  async function createOrUpdateBuild(existing?: HumanReviewInstance) {
    const targetSourceRef = existing?.sourceRef ?? sourceRef;
    const progressKey = existing?.instanceRef ?? 'prepare';
    setBusy(existing ? `rebuild:${existing.instanceRef}` : 'prepare');
    setError(null);
    const prepareOperationRef = operationId('prepare');
    const preparePolling = pollProgress(client, prepareOperationRef, (value) =>
      setProgress((current) => ({ ...current, [progressKey]: value })),
    );
    let buildPolling: ReturnType<typeof pollProgress> | null = null;
    try {
      const prepared = await client.prepare(
        prepareOperationRef,
        targetSourceRef,
        'Worktree review',
      );
      update(prepared);
      setSelectedInstanceRef(prepared.instanceRef);
      preparePolling.stop();
      await preparePolling.refresh();
      setBusy(`build:${prepared.instanceRef}`);
      const buildOperationRef = operationId('build');
      buildPolling = pollProgress(client, buildOperationRef, (value) =>
        setProgress((current) => ({ ...current, [prepared.instanceRef]: value })),
      );
      const built = await client.build(buildOperationRef, prepared.instanceRef);
      setInstances((current) => [
        built,
        ...current.filter((item) => item.sourceRef !== built.sourceRef),
      ]);
    } catch (cause) {
      setError(message(cause));
    } finally {
      preparePolling.stop();
      await preparePolling.refresh();
      buildPolling?.stop();
      await buildPolling?.refresh();
      setBusy(null);
    }
  }

  async function openHistory(selectedSourceRef: string, trigger: HTMLButtonElement) {
    const request = ++historyRequest.current;
    historyTrigger.current = trigger;
    setHistoryBusy(true);
    setError(null);
    try {
      const value = await client.sourceHistory(selectedSourceRef);
      if (request === historyRequest.current) setHistory(value);
    } catch (cause) {
      if (request === historyRequest.current) setError(message(cause));
    } finally {
      if (request === historyRequest.current) setHistoryBusy(false);
    }
  }

  async function attachWorktree(selectedSourceRef: string) {
    setBusy('attach');
    setError(null);
    try {
      const attached = await client.attachWorktree(selectedSourceRef);
      applySources([
        attached,
        ...sources.filter((source) => source.sourceRef !== attached.sourceRef),
      ]);
      setRepositorySources((current) =>
        current.map((source) => source.sourceRef === attached.sourceRef ? attached : source),
      );
      setSourceRef(attached.sourceRef);
    } catch (cause) {
      setError(message(cause));
    } finally {
      setBusy(null);
    }
  }

  async function openExplorer() {
    setExplorerOpen(true);
    if (repositorySources.length > 0) return;
    setExplorerLoading(true);
    setError(null);
    try {
      setRepositorySources(await client.listRepositoryHistory());
    } catch (cause) {
      setError(message(cause));
      setExplorerOpen(false);
    } finally {
      setExplorerLoading(false);
    }
  }

  const closeExplorer = useCallback(() => setExplorerOpen(false), []);

  function useExploredWorktree(value: string) {
    selectSource(value);
    setExplorerOpen(false);
  }

  const closeHistory = useCallback(() => {
    historyRequest.current += 1;
    setHistory(null);
    setHistoryBusy(false);
    window.requestAnimationFrame(() => historyTrigger.current?.focus());
  }, []);

  function selectSource(value: string) {
    historyRequest.current += 1;
    setHistory(null);
    setHistoryBusy(false);
    setSelectedInstanceRef('');
    setSourceRef(value);
  }

  async function act(
    instance: HumanReviewInstance,
    label: string,
    operation: (operationRef: string) => Promise<HumanReviewInstance>,
  ) {
    setBusy(`${label}:${instance.instanceRef}`);
    setError(null);
    const operationRef = operationId(label);
    const polling =
      label === 'build' || label === 'start'
        ? pollProgress(client, operationRef, (value) =>
            setProgress((current) => ({ ...current, [instance.instanceRef]: value })),
          )
        : null;
    try {
      update(await operation(operationRef));
    } catch (cause) {
      setError(message(cause));
    } finally {
      polling?.stop();
      await polling?.refresh();
      setBusy(null);
    }
  }

  if (surface === 'details' && detail) {
    return (
      <WorktreeBuildDetailScreen
        detail={detail}
        onBack={() => setSurface('overview')}
        onCompare={() => setSurface('files')}
        expandedOperationRef={expandedOperationRef}
      />
    );
  }
  if (surface === 'files' && detail) {
    return (
      <section className="human-review__file-route">
        <header>
          <button type="button" onClick={() => setSurface('details')}>
            <ArrowLeft size={16} />
            Build details
          </button>
          <span>{detail.name} - machine main HEAD to complete selected worktree</span>
        </header>
        <FileReviewScreen source={client.comparison(detail.instanceRef)} />
      </section>
    );
  }

  const selectedSource = sources.find((source) => source.sourceRef === sourceRef);
  const selectedInstances = instances.filter((instance) => instance.sourceRef === sourceRef);
  const selectedInstance = selectedInstances.find(
    (instance) => instance.instanceRef === selectedInstanceRef,
  );
  return (
    <main
      className="human-review"
      aria-label="Worktree review launcher"
      aria-busy={busy !== null || historyBusy || sourcesLoading || instancesLoading}
    >
      <header className="human-review__header">
        <div>
          <p className="eyebrow">Development tool</p>
          <h1>Worktree review</h1>
          <p>
            Open the real application from another worktree without closing this window or switching
            your checkout.
          </p>
        </div>
        <button type="button" onClick={load} disabled={busy !== null || sourcesLoading}>
          Refresh
        </button>
      </header>
      {ownedProgress && (
        <section aria-label="Current application-owned review operation">
          <OperationProgress progress={ownedProgress} />
        </section>
      )}

      <section className="human-review__prepare" aria-labelledby="prepare-review-title">
        <div className="human-review__prepare-intro">
          <h2 id="prepare-review-title">Choose a worktree</h2>
          <p>
            Select an attached worktree to build from its current folder. Explore repository
            history only when you need another branch or tag.
          </p>
        </div>
        <WorktreeSourcePicker
          sources={sources}
          selectedSourceRef={sourceRef}
          disabled={busy !== null || historyBusy}
          historyLoading={historyBusy}
          onSelect={selectSource}
          onExplore={() => void openExplorer()}
          onViewHistory={(value, trigger) => void openHistory(value, trigger)}
        />
        {selectedSource?.attached && selectedSource.compatibility === 'incompatible' && (
          <p className="human-review__compatibility" role="status">
            {selectedSource.compatibilityMessage}
          </p>
        )}
      </section>

      {explorerOpen && (
        <RepositoryHistoryDialog
          sources={repositorySources}
          loading={explorerLoading}
          attaching={busy === 'attach'}
          onClose={closeExplorer}
          onAttach={(value) => void attachWorktree(value)}
          onUse={useExploredWorktree}
          onCompare={(value, trigger) => void openHistory(value, trigger)}
        />
      )}

      {history && <CommitHistoryDialog history={history} onClose={closeHistory} />}

      {error && (
        <p className="human-review__error" role="alert">
          {error}
        </p>
      )}
      <section className="human-review__instances" aria-label="Worktree build">
        <header>
          <div>
            <h2>Worktree build</h2>
            <p>
              {selectedSource
                ? `Build directly from ${selectedSource.label}.`
                : 'Select an attached worktree to build and launch it.'}
            </p>
          </div>
        </header>
        <section className="human-review__build-detail" aria-label="Selected worktree build">
          {selectedInstance ? (
            <RetainedBuildDetails
              instance={selectedInstance}
              pending={busy !== null}
              progress={progress[selectedInstance.instanceRef]}
              onOpenDetail={() => void openDetail(selectedInstance.instanceRef)}
              onBuild={() =>
                void act(selectedInstance, 'build', (operationRef) =>
                  client.build(operationRef, selectedInstance.instanceRef),
                )
              }
              onUpdate={() => void createOrUpdateBuild(selectedInstance)}
              onLaunch={() =>
                void act(selectedInstance, 'start', (operationRef) =>
                  client.start(operationRef, selectedInstance.instanceRef),
                )
              }
              onFocus={() =>
                void act(selectedInstance, 'focus', () =>
                  client.focus(selectedInstance.instanceRef),
                )
              }
              onStop={() =>
                void act(selectedInstance, 'stop', () => client.stop(selectedInstance.instanceRef))
              }
            />
          ) : (
            <div className="human-review__build-empty">
              <h3>{instancesLoading ? 'Loading build...' : 'No build for this worktree'}</h3>
              <p>Create one build directly from the attached worktree.</p>
              {progress.prepare && <OperationProgress progress={progress.prepare} />}
              <button
                type="button"
                onClick={() => void createOrUpdateBuild()}
                disabled={
                  busy !== null ||
                  instancesLoading ||
                  !selectedSource ||
                  !selectedSource.attached ||
                  selectedSource.compatibility === 'incompatible' ||
                  selectedSource.detailsState !== 'ready'
                }
              >
                Create build
              </button>
            </div>
          )}
        </section>
      </section>
      <aside className="human-review__boundary">
        <strong>Review boundary</strong>
        <span>
          This opens a human-operated application window. A future troubleshooting Agent Session and
          dedicated Harness may consume the application-owned detail, progress, history, and
          sanitized output shown here; neither is implemented by this launcher.
        </span>
      </aside>
    </main>
  );
}

function preferredRetainedBuild(
  instances: readonly HumanReviewInstance[],
): HumanReviewInstance | undefined {
  return [...instances].sort(
    (left, right) => retainedBuildPreference(right) - retainedBuildPreference(left),
  )[0];
}

function retainedBuildPreference(instance: HumanReviewInstance) {
  if (instance.sourceState === 'current' && instance.build === 'passed') return 3;
  if (instance.sourceState === 'current') return 2;
  if (instance.build === 'passed') return 1;
  return 0;
}

function RetainedBuildDetails({
  instance,
  pending,
  progress,
  onOpenDetail,
  onBuild,
  onUpdate,
  onLaunch,
  onFocus,
  onStop,
}: {
  readonly instance: HumanReviewInstance;
  readonly pending: boolean;
  readonly progress?: HumanReviewOperationProgress;
  readonly onOpenDetail: () => void;
  readonly onBuild: () => void;
  readonly onUpdate: () => void;
  readonly onLaunch: () => void;
  readonly onFocus: () => void;
  readonly onStop: () => void;
}) {
  const running = instance.phase === 'running';
  const updateRequired =
    instance.build === 'superseded' ||
    instance.sourceState === 'outdated' ||
    instance.sourceState === 'changed';
  const buildReady = instance.build === 'passed' && !updateRequired;

  return (
    <article>
      <div className="human-review__card-title">
        <div>
          <p className="eyebrow">Selected worktree</p>
          <h3>
            {buildReady ? 'Build ready' : updateRequired ? 'Update available' : 'Build needed'}
          </h3>
        </div>
        <button type="button" onClick={onOpenDetail} disabled={pending}>
          Build details
        </button>
      </div>
      <p className={`human-review__freshness human-review__freshness--${instance.sourceState}`}>
        <strong>{buildFreshness(instance)}</strong>
      </p>
      <dl>
        <div>
          <dt>Revision</dt>
          <dd>{instance.currentRevision ?? instance.preparedRevision ?? 'Unavailable'}</dd>
        </div>
        <div>
          <dt>Status</dt>
          <dd>{running ? 'Running' : buildReady ? 'Ready to launch' : instance.build}</dd>
        </div>
      </dl>
      {progress && <OperationProgress progress={progress} />}
      <div className="human-review__actions">
        {running ? (
          <>
            <button type="button" disabled={pending} onClick={onFocus}>
              Focus window
            </button>
            <button type="button" disabled={pending} onClick={onStop}>
              Stop
            </button>
          </>
        ) : updateRequired ? (
          <button
            type="button"
            disabled={pending || instance.compatibility === 'incompatible'}
            onClick={onUpdate}
          >
            Update build
          </button>
        ) : buildReady ? (
          <button
            type="button"
            disabled={pending || instance.compatibility === 'incompatible'}
            onClick={onLaunch}
          >
            Launch
          </button>
        ) : (
          <button
            type="button"
            disabled={pending || instance.compatibility === 'incompatible'}
            onClick={onBuild}
          >
            Build
          </button>
        )}
      </div>
    </article>
  );
}

function buildFreshness(instance: HumanReviewInstance) {
  switch (instance.sourceState) {
    case 'current':
      return "Built from the worktree's current HEAD";
    case 'outdated': {
      const count = instance.outdatedByCommits ?? 0;
      return `Outdated by ${count} ${count === 1 ? 'commit' : 'commits'}`;
    }
    case 'changed':
      return 'Worktree HEAD or working state changed';
    case 'unavailable':
      return 'Current worktree state is unavailable';
    case 'unknown':
      return 'Prepared revision is unavailable';
  }
}

function OperationProgress({ progress }: { readonly progress: HumanReviewOperationProgress }) {
  return (
    <section className="human-review__progress" aria-live="polite">
      <div>
        <strong>{progress.stageLabel}</strong>
        <span>{duration(progress.elapsedMs)} elapsed</span>
      </div>
      <p>{progress.condition}</p>
      <dl>
        <div>
          <dt>Expected wait</dt>
          <dd>{progress.expectedWait}</dd>
        </div>
        <div>
          <dt>Evidence</dt>
          <dd>
            {progress.activity === 'quiet'
              ? `No new evidence for ${duration(progress.evidenceAgeMs)}; quiet evidence alone does not establish a failure.`
              : `Updated ${duration(progress.evidenceAgeMs)} ago.`}
          </dd>
        </div>
        {progress.missingReadinessFact && (
          <div>
            <dt>Still required</dt>
            <dd>{progress.missingReadinessFact}</dd>
          </div>
        )}
        <div>
          <dt>{progress.actionRequired ? 'Action required' : 'Human action'}</dt>
          <dd>{progress.actionGuidance}</dd>
        </div>
        <div>
          <dt>Reusable work</dt>
          <dd>{progress.reusableSummary}</dd>
        </div>
      </dl>
      {progress.recentOutput.length > 0 && (
        <details open={progress.state === 'pending'}>
          <summary>Recent safe output</summary>
          <pre>{progress.recentOutput.join('\n')}</pre>
        </details>
      )}
    </section>
  );
}

function pollProgress(
  client: HumanReviewLauncherClient,
  operationRef: string,
  update: (progress: HumanReviewOperationProgress) => void,
) {
  let active = true;
  const refresh = async () => {
    try {
      const value = await client.progress(operationRef);
      if (active) update(value);
    } catch {
      // The operation may not be registered until its blocking task begins.
    }
  };
  const timer = window.setInterval(() => void refresh(), 500);
  void refresh();
  return {
    refresh: async () => {
      const wasActive = active;
      active = true;
      await refresh();
      active = wasActive;
    },
    stop: () => {
      active = false;
      window.clearInterval(timer);
    },
  };
}

function operationId(label: string) {
  return `${label}-${crypto.randomUUID().replaceAll('-', '')}`;
}

function duration(value: number) {
  const seconds = Math.max(0, Math.floor(value / 1000));
  if (seconds < 60) return `${seconds}s`;
  return `${Math.floor(seconds / 60)}m ${seconds % 60}s`;
}

function message(cause: unknown) {
  return cause instanceof Error ? cause.message : String(cause || 'The review action failed.');
}
