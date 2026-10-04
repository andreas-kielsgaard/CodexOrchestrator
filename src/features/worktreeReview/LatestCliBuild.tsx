import { useEffect, useState } from 'react';
import {
  tauriWorktreeCliBuilds,
  type WorktreeCliBuild,
} from '../../infrastructure/tauriWorktreeCliBuilds';

export function LatestCliBuild({
  path,
  client = tauriWorktreeCliBuilds,
}: {
  readonly path: string;
  readonly client?: typeof tauriWorktreeCliBuilds;
}) {
  const [build, setBuild] = useState<WorktreeCliBuild | null>(null);
  const [opening, setOpening] = useState(false);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    let active = true;
    setBuild(null);
    setError(null);
    const refresh = () =>
      void client.latest(path).then(
        (result) => {
          if (active) setBuild(result);
        },
        (failure: unknown) => {
          if (active) setError(String(failure));
        },
      );
    refresh();
    const timer = window.setInterval(refresh, 60_000);
    return () => {
      active = false;
      window.clearInterval(timer);
    };
  }, [path, client]);
  const open = async () => {
    setOpening(true);
    setError(null);
    try {
      await client.open(path);
    } catch (failure) {
      setError(String(failure));
    } finally {
      setOpening(false);
    }
  };
  if (!build && !error) return null;
  return (
    <section className="worktree-review__section" aria-label="Latest CLI build">
      {build && (
        <>
          <div className="worktree-review__section-heading">
            <h2>Latest worktree build</h2>
            <button
              type="button"
              className="worktree-review__secondary"
              disabled={opening}
              onClick={() => void open()}
            >
              {opening ? 'Opening…' : 'Open latest build'}
            </button>
          </div>
          <p className="worktree-review__supporting">
            {build.profile} build · Opens the latest available CLI build without rebuilding.
          </p>
        </>
      )}
      {error && (
        <p role="alert" className="worktree-review__error-text">
          {error}
        </p>
      )}
    </section>
  );
}
