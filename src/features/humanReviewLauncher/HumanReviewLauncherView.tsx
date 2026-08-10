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
  const [name, setName] = useState('Worktree review');
  const [busy, setBusy] = useState<string | null>('load');
  const [error, setError] = useState<string | null>(null);
  const [progress, setProgress] = useState<Record<string, HumanReviewOperationProgress>>({});
  const [ownedProgress, setOwnedProgress] = useState<HumanReviewOperationProgress | null>(null);
  const [detail, setDetail] = useState<WorktreeBuildDetail | null>(null);
  const [surface, setSurface] = useState<'overview' | 'details' | 'files'>('overview');
  const [expandedOperationRef, setExpandedOperationRef] = useState<string | undefined>();
  const [history, setHistory] = useState<HumanReviewSourceHistory | null>(null);
  const [historyBusy, setHistoryBusy] = useState(false);
  const [selectedInstanceRef, setSelectedInstanceRef] = useState('');
  const [showPrepare, setShowPrepare] = useState(false);
  const historyRequest = useRef(0);
  const historyTrigger = useRef<HTMLElement | null>(null);

  const load = useCallback(async () => {
    setBusy('load');
    setError(null);
    try {
      const [nextSources, nextInstances] = await Promise.all([
        client.listSources(),
        client.listInstances(),
      ]);
      setSources(nextSources);
      setInstances(nextInstances);
      setSourceRef((current) =>
        nextSources.some((source) => source.sourceRef === current)
          ? current
          : nextSources[0]?.sourceRef || '',
      );
    } catch (cause) {
      setError(message(cause));
    } finally {
      setBusy(null);
    }
  }, [client]);

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

  useEffect(() => void load(), [load]);

  useEffect(() => {
    setSelectedInstanceRef((current) => {
      const available = instances.filter((instance) => instance.sourceRef === sourceRef);
      return available.some((instance) => instance.instanceRef === current)
        ? current
        : available[0]?.instanceRef || '';
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

  async function prepare() {
    setBusy('prepare');
    setError(null);
    const operationRef = operationId('prepare');
    const polling = pollProgress(client, operationRef, (value) =>
      setProgress((current) => ({ ...current, prepare: value })),
    );
    try {
      const prepared = await client.prepare(operationRef, sourceRef, name.trim());
      update(prepared);
      setSelectedInstanceRef(prepared.instanceRef);
      setShowPrepare(false);
    } catch (cause) {
      setError(message(cause));
    } finally {
      polling.stop();
      await polling.refresh();
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
    setShowPrepare(false);
    setSourceRef(value);
  }

  async function attachWorktree(value: string) {
    setBusy('attach');
    setError(null);
    try {
      const attached = await client.attachWorktree(value);
      const nextSources = await client.listSources();
      setSources(nextSources);
      setSourceRef(attached.sourceRef);
    } catch (cause) {
      setError(message(cause));
    } finally {
      setBusy(null);
    }
  }

  async function rebuild(instance: HumanReviewInstance) {
    setBusy(`rebuild:${instance.instanceRef}`);
    setError(null);
    const prepareOperationRef = operationId('prepare');
    const preparePolling = pollProgress(client, prepareOperationRef, (value) =>
      setProgress((current) => ({ ...current, [instance.instanceRef]: value })),
    );
    let buildPolling: ReturnType<typeof pollProgress> | null = null;
    try {
      const replacement = await client.prepare(
        prepareOperationRef,
        instance.sourceRef,
        instance.name,
      );
      update(replacement);
      setSelectedInstanceRef(replacement.instanceRef);
      setBusy(`rebuild:${replacement.instanceRef}`);
      preparePolling.stop();
      await preparePolling.refresh();
      const buildOperationRef = operationId('build');
      buildPolling = pollProgress(client, buildOperationRef, (value) =>
        setProgress((current) => ({ ...current, [replacement.instanceRef]: value })),
      );
      update(await client.build(buildOperationRef, replacement.instanceRef));
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
          <span>{detail.name} · machine main HEAD → complete selected worktree</span>
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
      aria-busy={busy !== null || historyBusy}
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
        <button type="button" onClick={() => void load()} disabled={busy !== null}>
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
          <h2 id="prepare-review-title">Choose repository history</h2>
          <p>
            Select a branch, tag, or archived branch. A review worktree is required before a build
            can be prepared.
          </p>
        </div>
        <WorktreeSourcePicker
          sources={sources}
          selectedSourceRef={sourceRef}
          disabled={busy !== null || historyBusy}
          historyLoading={historyBusy}
          attaching={busy === 'attach'}
          onSelect={selectSource}
          onAttach={(value) => void attachWorktree(value)}
          onViewHistory={(value, trigger) => void openHistory(value, trigger)}
        />
        {selectedSource?.attached && selectedSource.compatibility === 'incompatible' && (
          <p className="human-review__compatibility" role="status">
            {selectedSource.compatibilityMessage}
          </p>
        )}
      </section>

      {history && <CommitHistoryDialog history={history} onClose={closeHistory} />}

      {error && (
        <p className="human-review__error" role="alert">
          {error}
        </p>
      )}
      <section className="human-review__instances" aria-label="Retained review builds">
        <header>
          <div>
            <h2>Retained builds</h2>
            <p>
              {selectedSource
                ? `Builds prepared from ${selectedSource.label}.`
                : 'Select an attached worktree to inspect its retained builds.'}
            </p>
          </div>
          <button
            type="button"
            onClick={() => setShowPrepare((current) => !current)}
            disabled={
              busy !== null ||
              !selectedSource ||
              !selectedSource.attached ||
              selectedSource.compatibility === 'incompatible'
            }
            aria-expanded={showPrepare}
          >
            Prepare new build
          </button>
        </header>
        {showPrepare && (
          <div className="human-review__prepare-new" aria-label="Prepare a new retained build">
            <label>
              Build name
              <input
                value={name}
                maxLength={64}
                onChange={(event) => setName(event.target.value)}
              />
            </label>
            <button
              type="button"
              onClick={() => void prepare()}
              disabled={busy !== null || !sourceRef || !name.trim()}
            >
              Prepare build
            </button>
            <button type="button" onClick={() => setShowPrepare(false)} disabled={busy !== null}>
              Cancel
            </button>
            {progress.prepare && <OperationProgress progress={progress.prepare} />}
          </div>
        )}
        <div className="human-review__build-browser">
          <nav
            className="human-review__build-list"
            aria-label="Retained builds for selected worktree"
          >
            {selectedInstances.length === 0 && busy === null ? (
              <p>No retained builds for this worktree yet.</p>
            ) : (
              selectedInstances.map((instance) => (
                <button
                  key={instance.instanceRef}
                  type="button"
                  className={
                    instance.instanceRef === selectedInstanceRef ? 'is-selected' : undefined
                  }
                  aria-pressed={instance.instanceRef === selectedInstanceRef}
                  onClick={() => setSelectedInstanceRef(instance.instanceRef)}
                >
                  <strong>{instance.name}</strong>
                  <span>{buildFreshness(instance)}</span>
                  <small>
                    {instance.phase} · {instance.build}
                  </small>
                </button>
              ))
            )}
          </nav>
          <section
            className="human-review__build-detail"
            aria-label="Selected retained build details"
          >
            {selectedInstance ? (
              <RetainedBuildDetails
                instance={selectedInstance}
                pending={busy?.endsWith(selectedInstance.instanceRef) ?? false}
                progress={progress[selectedInstance.instanceRef]}
                onOpenDetail={() => void openDetail(selectedInstance.instanceRef)}
                onBuild={() =>
                  void act(selectedInstance, 'build', (operationRef) =>
                    client.build(operationRef, selectedInstance.instanceRef),
                  )
                }
                onRebuild={() => void rebuild(selectedInstance)}
                onOpen={() =>
                  void act(selectedInstance, 'start', (operationRef) =>
                    client.start(operationRef, selectedInstance.instanceRef),
                  )
                }
                onFocus={() =>
                  void act(selectedInstance, 'focus', () =>
                    client.focus(selectedInstance.instanceRef),
                  )
                }
                onStatus={() =>
                  void act(selectedInstance, 'status', () =>
                    client.status(selectedInstance.instanceRef),
                  )
                }
                onStop={() =>
                  void act(selectedInstance, 'stop', () =>
                    client.stop(selectedInstance.instanceRef),
                  )
                }
                onRecover={() =>
                  void act(selectedInstance, 'recover', () =>
                    client.recover(selectedInstance.instanceRef),
                  )
                }
              />
            ) : (
              <div className="human-review__build-empty">
                <h3>No build selected</h3>
                <p>Choose a retained build, or prepare a new build from this worktree.</p>
              </div>
            )}
          </section>
        </div>
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

function RetainedBuildDetails({
  instance,
  pending,
  progress,
  onOpenDetail,
  onBuild,
  onRebuild,
  onOpen,
  onFocus,
  onStatus,
  onStop,
  onRecover,
}: {
  readonly instance: HumanReviewInstance;
  readonly pending: boolean;
  readonly progress?: HumanReviewOperationProgress;
  readonly onOpenDetail: () => void;
  readonly onBuild: () => void;
  readonly onRebuild: () => void;
  readonly onOpen: () => void;
  readonly onFocus: () => void;
  readonly onStatus: () => void;
  readonly onStop: () => void;
  readonly onRecover: () => void;
}) {
  const running = instance.phase === 'running';
  const replacementRequired =
    instance.build === 'superseded' ||
    instance.sourceState === 'outdated' ||
    instance.sourceState === 'changed';
  return (
    <article>
      <div className="human-review__card-title">
        <div>
          <p className="eyebrow">Selected build</p>
          <h3>{instance.name}</h3>
        </div>
        <button type="button" onClick={onOpenDetail} disabled={pending}>
          Build details
        </button>
      </div>
      <p className={`human-review__freshness human-review__freshness--${instance.sourceState}`}>
        <strong>{buildFreshness(instance)}</strong>
        {replacementRequired && (
          <span>
            Rebuild creates a replacement at the current branch head and retains this history.
          </span>
        )}
      </p>
      <p className="human-review__purpose">{instance.purpose}</p>
      <dl>
        <div>
          <dt>Lifecycle</dt>
          <dd>{instance.phase}</dd>
        </div>
        <div>
          <dt>Current use</dt>
          <dd>{instance.currentUse}</dd>
        </div>
        <div>
          <dt>Health</dt>
          <dd>{instance.stale ? 'Needs recovery' : instance.health}</dd>
        </div>
        <div>
          <dt>Build</dt>
          <dd>{instance.build}</dd>
        </div>
        <div>
          <dt>Built revision</dt>
          <dd>{instance.preparedRevision ?? 'Unavailable'}</dd>
        </div>
        <div>
          <dt>Branch revision</dt>
          <dd>{instance.currentRevision ?? 'Unavailable'}</dd>
        </div>
      </dl>
      <p className={instance.actionRequired ? 'human-review__action-needed' : undefined}>
        <strong>{instance.actionRequired ? 'Human action needed: ' : 'Next safe action: '}</strong>
        {instance.actionSummary}
      </p>
      <p className="human-review__cleanup">{instance.cleanup}</p>
      {progress && <OperationProgress progress={progress} />}
      <div className="human-review__actions">
        {replacementRequired ? (
          <button
            type="button"
            disabled={pending || running || instance.compatibility === 'incompatible'}
            onClick={onRebuild}
          >
            Rebuild
          </button>
        ) : (
          <button
            type="button"
            disabled={pending || running || instance.compatibility === 'incompatible'}
            onClick={onBuild}
          >
            {instance.build === 'rebuild-required' ? 'Rebuild' : 'Build'}
          </button>
        )}
        <button
          type="button"
          disabled={
            pending ||
            running ||
            replacementRequired ||
            instance.build !== 'passed' ||
            instance.compatibility === 'incompatible'
          }
          onClick={onOpen}
        >
          Open
        </button>
        <button type="button" disabled={pending || !instance.canFocus} onClick={onFocus}>
          Focus window
        </button>
        <button type="button" disabled={pending} onClick={onStatus}>
          Check status
        </button>
        <button type="button" disabled={pending || !running} onClick={onStop}>
          Stop
        </button>
        <button
          type="button"
          disabled={pending || (!instance.stale && running && instance.health === 'healthy')}
          onClick={onRecover}
        >
          Recover
        </button>
      </div>
    </article>
  );
}

function buildFreshness(instance: HumanReviewInstance) {
  switch (instance.sourceState) {
    case 'current':
      return 'Built from the current branch head';
    case 'outdated': {
      const count = instance.outdatedByCommits ?? 0;
      return `Outdated by ${count} ${count === 1 ? 'commit' : 'commits'}`;
    }
    case 'changed':
      return 'Branch history or working state changed';
    case 'unavailable':
      return 'Current branch state is unavailable';
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
