import { useCallback, useEffect, useState } from 'react';
import type {
  AgentCliUpdateClient,
  AgentCliUpdateOverviewDto,
  AgentCliUpdateStatusDto,
} from '../../application/agentCliUpdates';

const ACTIVE_REFRESH_MS = 1_500;
const SETTLED_REFRESH_MS = 60_000;

export function AgentCliUpdateNotice({ client }: { readonly client: AgentCliUpdateClient }) {
  const [overview, setOverview] = useState<AgentCliUpdateOverviewDto | null>(null);
  const [error, setError] = useState<string | null>(null);
  const load = useCallback(async () => {
    try {
      setOverview(await client.overview());
      setError(null);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    }
  }, [client]);

  useEffect(() => {
    void load();
  }, [load]);
  useEffect(() => {
    const active = overview?.providers.some(
      (provider) => provider.phase === 'checking' || provider.phase === 'updating',
    );
    const timer = window.setTimeout(
      () => void load(),
      active ? ACTIVE_REFRESH_MS : SETTLED_REFRESH_MS,
    );
    return () => window.clearTimeout(timer);
  }, [load, overview]);

  return (
    <section className="agent-cli-updates" aria-labelledby="agent-cli-updates-title">
      <header>
        <div>
          <p>Background maintenance</p>
          <h2 id="agent-cli-updates-title">Agent tool updates</h2>
        </div>
        <span>Stable channel</span>
      </header>
      {overview?.providers.map((provider) => (
        <ProviderUpdate key={provider.provider} provider={provider} />
      ))}
      {!overview && !error ? <p role="status">Loading agent tool update status…</p> : null}
      {error ? <p role="alert">Could not load agent tool update status: {error}</p> : null}
    </section>
  );
}

function ProviderUpdate({ provider }: { readonly provider: AgentCliUpdateStatusDto }) {
  const active = provider.phase === 'checking' || provider.phase === 'updating';
  const failed = provider.phase === 'failed' || provider.phase === 'unavailable';
  return (
    <article className={`agent-cli-update agent-cli-update--${provider.phase}`}>
      <div>
        <strong>{provider.label}</strong>
        <span>{phaseLabel(provider)}</span>
      </div>
      <p role={failed ? 'alert' : active ? 'status' : undefined}>{provider.summary}</p>
      {provider.checkedAt ? (
        <small>Checked {new Date(provider.checkedAt).toLocaleString()}</small>
      ) : null}
      {provider.notes.length ? (
        <details>
          <summary>Update notes</summary>
          <pre>{provider.notes.join('\n')}</pre>
        </details>
      ) : null}
    </article>
  );
}

function phaseLabel(provider: AgentCliUpdateStatusDto): string {
  switch (provider.phase) {
    case 'checking':
      return 'Checking…';
    case 'updating':
      return 'Updating…';
    case 'current':
      return 'Current';
    case 'updated':
      return 'Updated';
    case 'controller_managed':
      return 'Managed by controller';
    case 'failed':
      return 'Update failed';
    case 'unavailable':
      return 'Unavailable';
  }
}
