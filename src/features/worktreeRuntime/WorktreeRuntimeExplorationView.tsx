import { useCallback, useEffect, useState } from 'react';
import type {
  WorktreeRuntimeEvidenceKind,
  WorktreeRuntimeExplorationSnapshot,
  WorktreeRuntimeExplorationSource,
} from '../../application/worktreeRuntime';
import './worktreeRuntimeExploration.css';

export function WorktreeRuntimeExplorationView({
  source,
}: {
  readonly source: WorktreeRuntimeExplorationSource;
}) {
  const [snapshot, setSnapshot] = useState<WorktreeRuntimeExplorationSnapshot | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const refresh = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      setSnapshot(await source.load());
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : 'Runtime evidence is unavailable.');
    } finally {
      setLoading(false);
    }
  }, [source]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  if (!snapshot) {
    return (
      <main className="worktree-runtime" aria-label="Worktree runtime">
        <p role={error ? 'alert' : 'status'}>{error ?? 'Loading worktree runtime evidenceâ€¦'}</p>
        {error && (
          <button type="button" onClick={() => void refresh()}>
            Retry
          </button>
        )}
      </main>
    );
  }

  return (
    <main className="worktree-runtime" aria-label="Worktree runtime" aria-busy={loading}>
      <header className="worktree-runtime__header">
        <div>
          <p className="eyebrow">Development proof</p>
          <h1>Worktree runtime</h1>
          <p>{snapshot.notice}</p>
        </div>
        <div className="worktree-runtime__header-actions">
          <EvidenceBadge kind="observed" label={snapshot.label} />
          <button type="button" onClick={() => void refresh()} disabled={loading}>
            {loading ? 'Refreshingâ€¦' : 'Refresh evidence'}
          </button>
        </div>
      </header>

      <dl className="worktree-runtime__identity" aria-label="Test instance identity">
        <IdentityValue label="Instance" value={snapshot.identity.instanceId} />
        <IdentityValue label="Build" value={shortHash(snapshot.identity.sourceFingerprint)} />
        <IdentityValue label="Session" value={snapshot.identity.sessionId} />
        <IdentityValue label="Commit" value={shortHash(snapshot.identity.gitCommit)} />
        <IdentityValue label="Worktree" value={snapshot.identity.worktreePath} wide />
        <IdentityValue
          label="Application identity"
          value={snapshot.identity.tauriIdentifier}
          wide
        />
      </dl>

      <div className="worktree-runtime__grid">
        <section className="worktree-runtime__panel" aria-labelledby="runtime-isolation-title">
          <div className="worktree-runtime__section-heading">
            <div>
              <p className="eyebrow">Material boundary</p>
              <h2 id="runtime-isolation-title">Shared only when keyed</h2>
            </div>
            <p>Paths remain instance-local unless a row says shared.</p>
          </div>
          <div className="worktree-runtime__table-wrap">
            <table>
              <thead>
                <tr>
                  <th>Material</th>
                  <th>Disposition</th>
                  <th>Evidence</th>
                </tr>
              </thead>
              <tbody>
                {snapshot.materials.map((item) => (
                  <tr key={item.material}>
                    <td>
                      <strong>{item.material}</strong>
                      <small>{item.detail}</small>
                    </td>
                    <td>
                      <span className={`worktree-runtime__boundary ${item.disposition}`}>
                        {boundaryLabel(item.disposition)}
                      </span>
                    </td>
                    <td>
                      <EvidenceBadge kind={item.evidence} />
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </section>

        <section className="worktree-runtime__panel" aria-labelledby="runtime-lifecycle-title">
          <div className="worktree-runtime__section-heading">
            <div>
              <p className="eyebrow">Lifecycle</p>
              <h2 id="runtime-lifecycle-title">Projected versus actual</h2>
            </div>
            <p>Requests do not become evidence until they are observed.</p>
          </div>
          <ol className="worktree-runtime__timeline">
            {snapshot.lifecycle.map((item) => (
              <li key={item.stage}>
                <span className="worktree-runtime__timeline-marker" aria-hidden="true" />
                <div>
                  <div className="worktree-runtime__timeline-title">
                    <strong>{item.stage}</strong>
                    <EvidenceBadge kind={item.evidence} />
                  </div>
                  <p>{item.state}</p>
                  <small>{item.detail}</small>
                </div>
              </li>
            ))}
          </ol>
        </section>
      </div>

      <div className="worktree-runtime__grid worktree-runtime__grid--lower">
        <section className="worktree-runtime__panel" aria-labelledby="runtime-limits-title">
          <p className="eyebrow">Not established</p>
          <h2 id="runtime-limits-title">Unsupported boundaries</h2>
          <ul className="worktree-runtime__limits">
            {snapshot.unsupported.map((item) => (
              <li key={item}>{item}</li>
            ))}
          </ul>
        </section>
        <section
          className="worktree-runtime__panel worktree-runtime__review"
          aria-labelledby="runtime-review-title"
        >
          <p className="eyebrow">Human control</p>
          <h2 id="runtime-review-title">Review before product work</h2>
          <ol>
            {snapshot.reviewPoints.map((item) => (
              <li key={item}>{item}</li>
            ))}
          </ol>
        </section>
      </div>

      <footer className="worktree-runtime__footer">
        Evidence checked {new Date(snapshot.checkedAt).toLocaleString()}. This view is inspect-only.
      </footer>
    </main>
  );
}

function IdentityValue({
  label,
  value,
  wide = false,
}: {
  readonly label: string;
  readonly value: string;
  readonly wide?: boolean;
}) {
  return (
    <div className={wide ? 'wide' : undefined}>
      <dt>{label}</dt>
      <dd title={value}>{value}</dd>
    </div>
  );
}

function EvidenceBadge({
  kind,
  label,
}: {
  readonly kind: WorktreeRuntimeEvidenceKind;
  readonly label?: string;
}) {
  return <span className={`worktree-runtime__evidence ${kind}`}>{label ?? kind}</span>;
}

function shortHash(value: string): string {
  return value.length > 12 ? value.slice(0, 12) : value;
}

function boundaryLabel(value: 'isolated' | 'shared-keyed' | 'unsupported'): string {
  if (value === 'shared-keyed') return 'Shared Â· keyed';
  if (value === 'unsupported') return 'Unsupported';
  return 'Isolated';
}
