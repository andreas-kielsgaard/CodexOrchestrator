import { useCallback, useEffect, useState } from 'react';
import {
  worktreeReviewErrorMessage,
  type WorktreeReviewClient,
  type WorktreeReviewReadiness,
} from '../../application/worktreeReview';
import { WorktreeReviewLauncherView } from './HumanReviewLauncherView';
import './humanReviewLauncher.css';

export function WorktreeReviewScreen({ client }: { readonly client: WorktreeReviewClient }) {
  const [readiness, setReadiness] = useState<WorktreeReviewReadiness | null>(null);
  const [repositoryRoot, setRepositoryRoot] = useState('');
  const [busy, setBusy] = useState(false);

  const refresh = useCallback(async () => {
    setBusy(true);
    try {
      const next = await client.readiness();
      setReadiness(next);
      setRepositoryRoot((current) => current || next.repositoryRoot || '');
    } catch (cause) {
      setReadiness({
        status: 'repositoryUnavailable',
        message: message(cause, 'Worktree Review readiness could not be read.'),
      });
    } finally {
      setBusy(false);
    }
  }, [client]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  async function selectRepository() {
    const selectedRoot = repositoryRoot.trim();
    if (!selectedRoot) return;
    setBusy(true);
    try {
      setReadiness(await client.selectRepository(selectedRoot));
    } catch (cause) {
      setReadiness({
        status: 'repositoryUnavailable',
        message: message(cause, 'The selected repository is unavailable.'),
        repositoryRoot: selectedRoot,
      });
    } finally {
      setBusy(false);
    }
  }

  if (readiness?.status === 'ready') {
    return <WorktreeReviewLauncherView key={readiness.repositoryRoot} client={client} />;
  }

  return (
    <main className="human-review" aria-label="Worktree review readiness" aria-busy={busy}>
      <header className="human-review__header">
        <div>
          <p className="eyebrow">Product capability</p>
          <h1>Worktree review</h1>
          <p>Build and open another repository worktree without switching this checkout.</p>
        </div>
        <button type="button" disabled={busy} onClick={() => void refresh()}>
          Check readiness
        </button>
      </header>
      <section className="human-review__prepare" aria-labelledby="review-readiness-title">
        <div className="human-review__prepare-intro">
          <h2 id="review-readiness-title">
            {readiness ? readinessTitle(readiness.status) : 'Checking readiness'}
          </h2>
          <p role="status">
            {readiness?.message ?? 'Reading the selected repository and required toolchain.'}
          </p>
        </div>
        {readiness?.status !== 'storageUnavailable' && (
          <div className="human-review__repository-selection">
            <label htmlFor="worktree-review-repository">Repository folder</label>
            <input
              id="worktree-review-repository"
              value={repositoryRoot}
              disabled={busy}
              placeholder="C:\\path\\to\\repository"
              onChange={(event) => setRepositoryRoot(event.target.value)}
            />
            <button
              type="button"
              disabled={busy || repositoryRoot.trim().length === 0}
              onClick={() => void selectRepository()}
            >
              Use repository
            </button>
          </div>
        )}
      </section>
    </main>
  );
}

function readinessTitle(status: WorktreeReviewReadiness['status']) {
  switch (status) {
    case 'ready':
      return 'Ready';
    case 'needsRepository':
      return 'Choose a repository';
    case 'missingTool':
      return 'Required tool unavailable';
    case 'runtimeUnavailable':
      return 'Review runtime unavailable';
    case 'storageUnavailable':
      return 'Review storage unavailable';
    case 'repositoryUnavailable':
      return 'Repository unavailable';
  }
}

function message(cause: unknown, fallback: string) {
  return worktreeReviewErrorMessage(cause, fallback);
}
