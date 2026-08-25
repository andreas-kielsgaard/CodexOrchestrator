import { useEffect, useState } from 'react';
import type { WorktreeReviewClient } from '../../application/worktreeReview';

export function WorktreeReviewSettings({ client }: { readonly client: WorktreeReviewClient }) {
  const [enabled, setEnabled] = useState(false);
  const [loaded, setLoaded] = useState(false);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState('');

  useEffect(() => {
    let active = true;
    void client.settings().then(
      (settings) => {
        if (!active) return;
        setEnabled(settings.cleanupDetachedBuilds);
        setLoaded(true);
      },
      () => {
        if (!active) return;
        setMessage('Worktree Review settings are unavailable.');
        setLoaded(true);
      },
    );
    return () => {
      active = false;
    };
  }, [client]);

  async function update(value: boolean) {
    setBusy(true);
    setMessage('');
    try {
      const settings = await client.updateSettings({ cleanupDetachedBuilds: value });
      setEnabled(settings.cleanupDetachedBuilds);
      setMessage('Worktree Review cleanup setting saved.');
    } catch {
      setMessage('Worktree Review cleanup setting could not be saved.');
    } finally {
      setBusy(false);
    }
  }

  return (
    <section aria-labelledby="worktree-review-settings-title">
      <h2 id="worktree-review-settings-title">Worktree Review</h2>
      <label className="native-profile-setting-toggle">
        <input
          type="checkbox"
          role="switch"
          checked={enabled}
          disabled={!loaded || busy}
          onChange={(event) => void update(event.target.checked)}
        />
        <span>
          <strong>Clean builds when their worktree becomes detached</strong>
          <small>
            Removes the launcher-owned build, application data, logs, and runtime record. Shared
            dependency downloads are kept.
          </small>
        </span>
      </label>
      {message && <p role="status">{message}</p>}
    </section>
  );
}
