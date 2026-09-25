import { useCallback, useEffect, useState, type FormEvent } from 'react';
import { displayFolderPath, type ProviderSetupDto } from '../../../application/agentProviders';
import type { ClaudeSetupClient } from '../../../infrastructure/agentProviders/claude/claudeSetupClient';

const STATE_LABELS: Record<ProviderSetupDto['state'], string> = {
  ready: 'Ready',
  needs_login: 'Needs sign-in',
  unavailable: 'Unavailable',
};

/** Claude setups on this device. The default folder appears once Claude Code has created it. */
export function ClaudeSetupSettings({ client }: { readonly client: ClaudeSetupClient }) {
  const [setups, setSetups] = useState<readonly ProviderSetupDto[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [folder, setFolder] = useState('');
  const [executable, setExecutable] = useState('');
  const [busy, setBusy] = useState(false);

  const run = useCallback(
    async (operation: () => Promise<void>) => {
      setBusy(true);
      setError(null);
      try {
        await operation();
        setSetups(await client.listSetups());
      } catch (cause) {
        setError(cause instanceof Error ? cause.message : String(cause));
      } finally {
        setBusy(false);
      }
    },
    [client],
  );

  useEffect(() => {
    void run(async () => undefined);
  }, [run]);

  const add = (event: FormEvent) => {
    event.preventDefault();
    void run(async () => {
      await client.addSetup(folder.trim(), executable);
      setFolder('');
      setExecutable('');
    });
  };

  return (
    <section className="execution-setup-overview" aria-labelledby="claude-setups-title">
      <header>
        <p>Claude</p>
        <h2 id="claude-setups-title">Claude setups</h2>
        <span>
          Each setup is a Claude configuration folder and the Claude Code CLI that uses it. Sign in
          with <code>claude auth login</code>.
        </span>
      </header>
      {setups?.map((setup, index) => (
        <section
          className="execution-setup-overview__card"
          key={setup.configurationId}
          aria-label={`Claude setup ${displayFolderPath(setup.folder)}`}
        >
          <div>
            <p>{index === 0 ? 'Default setup' : 'Setup'}</p>
            <h3>{displayFolderPath(setup.folder)}</h3>
            <span>
              {setup.executable ?? 'claude'} · {STATE_LABELS[setup.state]}
            </span>
          </div>
          <button
            type="button"
            disabled={busy}
            onClick={() => void run(() => client.removeSetup(setup.configurationId))}
          >
            Remove
          </button>
          {setup.detail ? <p>{setup.detail}</p> : null}
        </section>
      ))}
      {setups?.length === 0 ? (
        <p>No Claude setup yet. The default folder is added once Claude Code has signed in here.</p>
      ) : null}
      <form className="execution-setup-overview__card" onSubmit={add} aria-label="Add Claude setup">
        <div>
          <label>
            <span>Configuration folder</span>
            <input value={folder} onChange={(event) => setFolder(event.target.value)} required />
          </label>
          <label>
            <span>CLI executable (optional)</span>
            <input
              value={executable}
              placeholder="claude"
              onChange={(event) => setExecutable(event.target.value)}
            />
          </label>
        </div>
        <button type="submit" disabled={busy || !folder.trim()}>
          Add setup
        </button>
      </form>
      {error ? (
        <p className="execution-setup-overview__error" role="alert">
          {error}
        </p>
      ) : null}
    </section>
  );
}
