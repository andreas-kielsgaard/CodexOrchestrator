import { ReviewDialog } from './ReviewDialog';
import { useEffect, useMemo, useState } from 'react';
import type {
  LocalRepositoryRegistrationCandidate,
  RegisteredRepository,
  RepositoryCatalogClient,
  RepositoryCatalogOverview,
} from '../../application/repositoryCatalog';

export function RepositoryRegistrationModal({
  catalog,
  onClose,
  onRegistered,
}: {
  readonly catalog: RepositoryCatalogClient;
  readonly onClose: () => void;
  readonly onRegistered: (repository: RegisteredRepository) => void;
}) {
  const [overview, setOverview] = useState<RepositoryCatalogOverview | null>(null);
  const [directory, setDirectory] = useState('');
  const [query, setQuery] = useState('');
  const [busy, setBusy] = useState<string | null>('loading');
  const [error, setError] = useState<string | null>(null);

  async function load() {
    setBusy('loading');
    setError(null);
    try {
      setOverview(await catalog.overview());
    } catch (cause) {
      setError(message(cause));
    } finally {
      setBusy(null);
    }
  }

  useEffect(() => {
    void load();
    // The modal intentionally loads one snapshot when it opens.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const repositories = useMemo(() => {
    const needle = query.trim().toLocaleLowerCase();
    if (!needle) return overview?.repositories ?? [];
    return (overview?.repositories ?? []).filter(
      (repository) =>
        repository.name.toLocaleLowerCase().includes(needle) ||
        repository.localInstances.some((instance) =>
          instance.locationLabel.toLocaleLowerCase().includes(needle),
        ),
    );
  }, [overview, query]);

  async function registerDirectory() {
    if (!directory.trim()) return;
    setBusy('directory');
    setError(null);
    try {
      onRegistered(await catalog.registerDirectory(directory.trim()));
    } catch (cause) {
      setError(message(cause));
      setBusy(null);
    }
  }

  async function registerCodex(candidate: LocalRepositoryRegistrationCandidate) {
    setBusy(candidate.repositoryId);
    setError(null);
    try {
      onRegistered(await catalog.registerCodexRepository(candidate.repositoryId));
    } catch (cause) {
      setError(message(cause));
      setBusy(null);
    }
  }

  return (
    <ReviewDialog
      labelledBy="repository-registration-title"
      onClose={onClose}
      dismissible={busy === null}
      className="worktree-review__registration-dialog"
    >
      <header className="worktree-review__modal-header">
        <div>
          <p className="worktree-review__step">Repository registration</p>
          <h2 id="repository-registration-title">Add a repository</h2>
          <p>
            Registration records a verified local Git repository. Codex and GitHub only help
            discover candidates.
          </p>
        </div>
        <button
          type="button"
          className="worktree-review__secondary"
          disabled={busy !== null}
          onClick={onClose}
        >
          Close
        </button>
      </header>

      {error && (
        <div className="worktree-review__alert" role="alert">
          <strong>Repository registration could not complete.</strong>
          <span>{error}</span>
        </div>
      )}

      <section className="worktree-review__registration-section">
        <h3>Add from a directory</h3>
        <form
          onSubmit={(event) => {
            event.preventDefault();
            void registerDirectory();
          }}
        >
          <label className="worktree-review__field">
            <span>Directory inside a Git repository</span>
            <input
              aria-label="Directory inside a Git repository"
              value={directory}
              placeholder="C:\\Projects\\Repository"
              disabled={busy !== null}
              onChange={(event) => setDirectory(event.target.value)}
            />
          </label>
          <button
            type="submit"
            className="worktree-review__primary"
            disabled={busy !== null || !directory.trim()}
          >
            {busy === 'directory' ? 'Verifying…' : 'Add directory'}
          </button>
        </form>
      </section>

      <section className="worktree-review__registration-section">
        <div className="worktree-review__registration-heading">
          <div>
            <h3>Known repositories</h3>
            <p>
              Local instances come from Codex task directories or prior registration. GitHub
              repositories without a local instance are shown for transparency and cannot yet be
              cloned here.
            </p>
          </div>
          <button
            type="button"
            className="worktree-review__secondary"
            disabled={busy !== null}
            onClick={() => void load()}
          >
            Retry discovery
          </button>
        </div>
        {overview && (
          <div className="worktree-review__discovery-status">
            <p>
              <strong>Codex:</strong> {overview.codex.message}
            </p>
            <p>
              <strong>GitHub:</strong> {overview.github.message}
              {overview.github.login ? ` Signed in as ${overview.github.login}.` : ''}
            </p>
          </div>
        )}
        <label className="worktree-review__field">
          <span>Filter repositories</span>
          <input
            type="search"
            value={query}
            disabled={busy === 'loading'}
            onChange={(event) => setQuery(event.target.value)}
          />
        </label>
        {busy === 'loading' && !overview ? (
          <p role="status">Discovering repositories…</p>
        ) : repositories.length === 0 ? (
          <p className="worktree-review__empty">No matching repositories were discovered.</p>
        ) : (
          <ul className="worktree-review__registration-list" aria-label="Known repositories">
            {repositories.map((repository) => (
              <li key={repository.catalogId}>
                <div className="worktree-review__registration-repository">
                  <div>
                    <strong>{repository.name}</strong>
                    {repository.github && <span>GitHub · {repository.github.visibility}</span>}
                  </div>
                  {repository.localInstances.length === 0 && (
                    <span className="worktree-review__badge">Not available locally</span>
                  )}
                </div>
                {repository.localInstances.map((instance) => (
                  <div className="worktree-review__local-instance" key={instance.repositoryId}>
                    <div>
                      <code>{instance.locationLabel}</code>
                      <span>{disclosureLabel(instance.disclosures)}</span>
                    </div>
                    <button
                      type="button"
                      className={
                        instance.registered
                          ? 'worktree-review__secondary'
                          : 'worktree-review__primary'
                      }
                      disabled={busy !== null || instance.registered}
                      onClick={() => void registerCodex(instance)}
                    >
                      {instance.registered ? 'Registered' : 'Register repository'}
                    </button>
                  </div>
                ))}
              </li>
            ))}
          </ul>
        )}
      </section>
    </ReviewDialog>
  );
}

function disclosureLabel(disclosures: readonly string[]): string {
  const labels = disclosures.map((value) =>
    value === 'codex_task' ? 'Exposed through Codex' : 'Added from directory',
  );
  return labels.length > 0 ? labels.join(' · ') : 'Local Git repository';
}

function message(cause: unknown): string {
  return cause instanceof Error ? cause.message : String(cause);
}
