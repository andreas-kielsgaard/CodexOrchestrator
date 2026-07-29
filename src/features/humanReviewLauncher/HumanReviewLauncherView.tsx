import { useCallback, useEffect, useState } from 'react';
import type {
  HumanReviewInstance,
  HumanReviewLauncherClient,
  HumanReviewOperationProgress,
  HumanReviewSource,
} from '../../application/humanReviewLauncher';
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
      setSourceRef((current) => current || nextSources[0]?.sourceRef || '');
    } catch (cause) {
      setError(message(cause));
    } finally {
      setBusy(null);
    }
  }, [client]);

  useEffect(() => void load(), [load]);

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

  const update = useCallback((instance: HumanReviewInstance) => {
    setInstances((current) => [
      instance,
      ...current.filter((item) => item.instanceRef !== instance.instanceRef),
    ]);
  }, []);

  async function prepare() {
    setBusy('prepare');
    setError(null);
    const operationRef = operationId('prepare');
    const polling = pollProgress(client, operationRef, (value) =>
      setProgress((current) => ({ ...current, prepare: value })),
    );
    try {
      update(await client.prepare(operationRef, sourceRef, name.trim()));
    } catch (cause) {
      setError(message(cause));
    } finally {
      polling.stop();
      await polling.refresh();
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
    const key = instance.instanceRef;
    const polling =
      label === 'build' || label === 'start'
        ? pollProgress(client, operationRef, (value) =>
            setProgress((current) => ({ ...current, [key]: value })),
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

  return (
    <main className="human-review" aria-label="Worktree review launcher" aria-busy={busy !== null}>
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
        <div>
          <h2 id="prepare-review-title">New review window</h2>
          <p>
            The application keeps its build, data, browser profile, ports, logs, and processes
            separate.
          </p>
        </div>
        <label>
          Worktree
          <select
            value={sourceRef}
            onChange={(event) => setSourceRef(event.target.value)}
            disabled={busy !== null}
          >
            {sources.map((source) => (
              <option key={source.sourceRef} value={source.sourceRef}>
                {source.label} ({source.revision})
              </option>
            ))}
          </select>
        </label>
        <label>
          Window name
          <input value={name} maxLength={64} onChange={(event) => setName(event.target.value)} />
        </label>
        <button
          type="button"
          onClick={() => void prepare()}
          disabled={busy !== null || !sourceRef || !name.trim()}
        >
          Prepare
        </button>
        {progress.prepare && <OperationProgress progress={progress.prepare} />}
      </section>

      {error && (
        <p className="human-review__error" role="alert">
          {error}
        </p>
      )}
      <section className="human-review__instances" aria-label="Review windows">
        {instances.length === 0 && busy === null && <p>No review windows prepared yet.</p>}
        {instances.map((instance) => {
          const running = instance.phase === 'running';
          const pending = busy?.endsWith(instance.instanceRef) ?? false;
          return (
            <article className="human-review__card" key={instance.instanceRef}>
              <div>
                <p className="eyebrow">{instance.sourceLabel}</p>
                <h2>{instance.name}</h2>
              </div>
              <dl>
                <div>
                  <dt>Lifecycle</dt>
                  <dd>{instance.phase}</dd>
                </div>
                <div>
                  <dt>Health</dt>
                  <dd>{instance.stale ? 'Needs recovery' : instance.health}</dd>
                </div>
                <div>
                  <dt>Build</dt>
                  <dd>{instance.build}</dd>
                </div>
              </dl>
              {progress[instance.instanceRef] && (
                <>
                  <OperationProgress progress={progress[instance.instanceRef]} />
                  {progress[instance.instanceRef].operation === 'start' && (
                    <p className="human-review__expected-window">
                      Expected result: a separate window titled “Codex Orchestrator [Worktree build:{' '}
                      {instance.name}]”.
                    </p>
                  )}
                </>
              )}
              <div className="human-review__actions">
                <button
                  type="button"
                  disabled={pending || running}
                  onClick={() =>
                    void act(instance, 'build', (operationRef) =>
                      client.build(operationRef, instance.instanceRef),
                    )
                  }
                >
                  Build
                </button>
                <button
                  type="button"
                  disabled={pending || running || instance.build !== 'passed'}
                  onClick={() =>
                    void act(instance, 'start', (operationRef) =>
                      client.start(operationRef, instance.instanceRef),
                    )
                  }
                >
                  Open
                </button>
                <button
                  type="button"
                  disabled={pending || !instance.canFocus}
                  onClick={() =>
                    void act(instance, 'focus', () => client.focus(instance.instanceRef))
                  }
                >
                  Focus window
                </button>
                <button
                  type="button"
                  disabled={pending}
                  onClick={() =>
                    void act(instance, 'status', () => client.status(instance.instanceRef))
                  }
                >
                  Check status
                </button>
                <button
                  type="button"
                  disabled={pending || !running}
                  onClick={() =>
                    void act(instance, 'stop', () => client.stop(instance.instanceRef))
                  }
                >
                  Stop
                </button>
                <button
                  type="button"
                  disabled={
                    pending || (!instance.stale && running && instance.health === 'healthy')
                  }
                  onClick={() =>
                    void act(instance, 'recover', () => client.recover(instance.instanceRef))
                  }
                >
                  Recover
                </button>
              </div>
            </article>
          );
        })}
      </section>
      <aside className="human-review__boundary">
        <strong>Review boundary</strong>
        <span>
          This opens a human-operated application window. It does not run assertions, capture the
          screen, or give an agent control of the reviewed app.
        </span>
      </aside>
    </main>
  );
}

function OperationProgress({ progress }: { readonly progress: HumanReviewOperationProgress }) {
  return (
    <section className="human-review__progress" aria-live="polite">
      <div>
        <strong>{progress.stageLabel}</strong>
        <span>{duration(progress.elapsedMs)} elapsed</span>
      </div>
      <p>
        {progress.activity === 'quiet'
          ? `No new evidence for ${duration(progress.evidenceAgeMs)}. The owned operation is still pending.`
          : progress.state === 'pending'
            ? `Working normally · updated ${duration(progress.evidenceAgeMs)} ago`
            : progress.state === 'succeeded'
              ? 'Finished successfully.'
              : 'Stopped with an error.'}
      </p>
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
