import { useEffect, useState } from 'react';
import type {
  JevCredentialClient,
  JevCredentialStatus,
} from '../../../infrastructure/agentProviders/jev/jevCredentialClient';
import './jevCredentialSettings.css';

export function JevCredentialSettings({ client }: { readonly client: JevCredentialClient }) {
  const [status, setStatus] = useState<JevCredentialStatus | null>(null);
  const [key, setKey] = useState('');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    let active = true;
    void client.loadStatus().then(
      (next) => {
        if (active) setStatus(next);
      },
      (cause) => {
        if (active) setError(String(cause));
      },
    );
    return () => {
      active = false;
    };
  }, [client]);

  const update = async (operation: 'save' | 'remove') => {
    setBusy(true);
    setError(null);
    try {
      setStatus(await (operation === 'save' ? client.saveKey(key.trim()) : client.removeKey()));
      setKey('');
    } catch (cause) {
      setError(String(cause));
    } finally {
      setBusy(false);
    }
  };

  return (
    <section className="jev-credentials" aria-labelledby="jev-credentials-title">
      <h2 id="jev-credentials-title">JEV</h2>
      <p>
        Save your TypeSafe API key in Windows Credential Manager. Orchid uses it across launches and
        updates.
      </p>
      <p role="status">
        {status
          ? status.saved
            ? 'API key saved securely.'
            : status.environmentAvailable
              ? 'Using the launcher’s TYPESAFE_API_KEY.'
              : 'No API key configured.'
          : 'Checking credential status…'}
      </p>
      <form
        onSubmit={(event) => {
          event.preventDefault();
          void update('save');
        }}
      >
        <label htmlFor="jev-api-key">{status?.saved ? 'Replace API key' : 'API key'}</label>
        <input
          id="jev-api-key"
          type="password"
          autoComplete="new-password"
          value={key}
          disabled={busy}
          onChange={(event) => setKey(event.target.value)}
        />
        <div className="jev-credentials__actions">
          <button type="submit" disabled={busy || !key.trim() || !status}>
            Save key
          </button>
          <button
            type="button"
            disabled={busy || !status?.saved}
            onClick={() => {
              void update('remove');
            }}
          >
            Remove saved key
          </button>
        </div>
      </form>
      {error && <p role="alert">{error}</p>}
    </section>
  );
}
