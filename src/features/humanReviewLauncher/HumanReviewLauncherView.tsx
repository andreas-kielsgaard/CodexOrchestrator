import { useCallback, useEffect, useState } from 'react';
import type {
  HumanReviewInstance,
  HumanReviewLauncherClient,
  HumanReviewSource,
} from '../../application/humanReviewLauncher';
import './humanReviewLauncher.css';

export function HumanReviewLauncherView({ client }: { readonly client: HumanReviewLauncherClient }) {
  const [sources, setSources] = useState<readonly HumanReviewSource[]>([]);
  const [instances, setInstances] = useState<readonly HumanReviewInstance[]>([]);
  const [sourceRef, setSourceRef] = useState('');
  const [name, setName] = useState('Worktree review');
  const [busy, setBusy] = useState<string | null>('load');
  const [error, setError] = useState<string | null>(null);

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

  const update = useCallback((instance: HumanReviewInstance) => {
    setInstances((current) => [instance, ...current.filter((item) => item.instanceRef !== instance.instanceRef)]);
  }, []);

  async function prepare() {
    setBusy('prepare');
    setError(null);
    try {
      update(await client.prepare(sourceRef, name.trim()));
    } catch (cause) {
      setError(message(cause));
    } finally {
      setBusy(null);
    }
  }

  async function act(instance: HumanReviewInstance, label: string, operation: () => Promise<HumanReviewInstance>) {
    setBusy(`${label}:${instance.instanceRef}`);
    setError(null);
    try {
      update(await operation());
    } catch (cause) {
      setError(message(cause));
    } finally {
      setBusy(null);
    }
  }

  return (
    <main className="human-review" aria-label="Worktree review launcher" aria-busy={busy !== null}>
      <header className="human-review__header">
        <div>
          <p className="eyebrow">Development tool</p>
          <h1>Worktree review</h1>
          <p>Open the real application from another worktree without closing this window or switching your checkout.</p>
        </div>
        <button type="button" onClick={() => void load()} disabled={busy !== null}>Refresh</button>
      </header>

      <section className="human-review__prepare" aria-labelledby="prepare-review-title">
        <div>
          <h2 id="prepare-review-title">New review window</h2>
          <p>The application keeps its build, data, browser profile, ports, logs, and processes separate.</p>
        </div>
        <label>Worktree<select value={sourceRef} onChange={(event) => setSourceRef(event.target.value)} disabled={busy !== null}>
          {sources.map((source) => <option key={source.sourceRef} value={source.sourceRef}>{source.label} ({source.revision})</option>)}
        </select></label>
        <label>Window name<input value={name} maxLength={64} onChange={(event) => setName(event.target.value)} /></label>
        <button type="button" onClick={() => void prepare()} disabled={busy !== null || !sourceRef || !name.trim()}>Prepare</button>
      </section>

      {error && <p className="human-review__error" role="alert">{error}</p>}
      <section className="human-review__instances" aria-label="Review windows">
        {instances.length === 0 && busy === null && <p>No review windows prepared yet.</p>}
        {instances.map((instance) => {
          const running = instance.phase === 'running';
          const pending = busy?.endsWith(instance.instanceRef) ?? false;
          return <article className="human-review__card" key={instance.instanceRef}>
            <div><p className="eyebrow">{instance.sourceLabel}</p><h2>{instance.name}</h2></div>
            <dl><div><dt>Lifecycle</dt><dd>{instance.phase}</dd></div><div><dt>Health</dt><dd>{instance.stale ? 'Needs recovery' : instance.health}</dd></div><div><dt>Build</dt><dd>{instance.build}</dd></div></dl>
            <div className="human-review__actions">
              <button type="button" disabled={pending || running} onClick={() => void act(instance, 'build', () => client.build(instance.instanceRef))}>Build</button>
              <button type="button" disabled={pending || running || instance.build !== 'passed'} onClick={() => void act(instance, 'start', () => client.start(instance.instanceRef))}>Open</button>
              <button type="button" disabled={pending || !instance.canFocus} onClick={() => void act(instance, 'focus', () => client.focus(instance.instanceRef))}>Focus window</button>
              <button type="button" disabled={pending} onClick={() => void act(instance, 'status', () => client.status(instance.instanceRef))}>Check status</button>
              <button type="button" disabled={pending || !running} onClick={() => void act(instance, 'stop', () => client.stop(instance.instanceRef))}>Stop</button>
              <button type="button" disabled={pending || (!instance.stale && running && instance.health === 'healthy')} onClick={() => void act(instance, 'recover', () => client.recover(instance.instanceRef))}>Recover</button>
            </div>
          </article>;
        })}
      </section>
      <aside className="human-review__boundary"><strong>Review boundary</strong><span>This opens a human-operated application window. It does not run assertions, capture the screen, or give an agent control of the reviewed app.</span></aside>
    </main>
  );
}

function message(cause: unknown) {
  return cause instanceof Error ? cause.message : String(cause || 'The review action failed.');
}
