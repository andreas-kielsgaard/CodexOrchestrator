import { useState } from 'react';
import type {
  CoordinatorOverview,
  RuntimeCoordinatorClient,
} from '../../application/runtimeCoordinator/contracts';

export function CoordinatorSettingsPage({
  client,
  overview,
  saved,
  back,
}: {
  readonly client: RuntimeCoordinatorClient;
  readonly overview: CoordinatorOverview;
  readonly saved: (overview: CoordinatorOverview) => void;
  readonly back: () => void;
}) {
  const [external, setExternal] = useState(overview.mode === 'external');
  const [url, setUrl] = useState(overview.endpoint ?? 'http://127.0.0.1:41800');
  const [token, setToken] = useState('');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const apply = async (connect: () => Promise<CoordinatorOverview>) => {
    setBusy(true);
    setError(null);
    try {
      const next = await connect();
      setToken('');
      saved(next);
    } catch (cause) {
      setError(String(cause));
    } finally {
      setBusy(false);
    }
  };
  return (
    <form
      className="coordinator-form"
      onSubmit={(event) => {
        event.preventDefault();
        void apply(() =>
          client.settings({
            externalUrl: external ? url.trim() : null,
            token: token.trim() || null,
          }),
        );
      }}
    >
      <button type="button" className="coordinator-back" disabled={busy} onClick={back}>
        ‹ Devices
      </button>
      <h2>Coordinator settings</h2>
      <p>
        First connect Orchid to a coordinator. Then choose Add device to connect your Hetzner
        account and select the server you want to control.
      </p>
      <fieldset disabled={busy}>
        <legend>Hosting</legend>
        <label className="coordinator-radio">
          <input
            type="radio"
            name="hosting"
            checked={!external}
            onChange={() => setExternal(false)}
          />{' '}
          Local coordinator on this laptop
        </label>
        <label className="coordinator-radio">
          <input
            type="radio"
            name="hosting"
            checked={external}
            onChange={() => setExternal(true)}
          />{' '}
          External coordinator
        </label>
      </fieldset>
      {!external && (
        <p className="coordinator-note">
          Orchid starts its bundled coordinator automatically. No address or access token is needed.
          Choose Use local coordinator to connect and return to the device list.
        </p>
      )}
      {external && (
        <fieldset disabled={busy}>
          <legend>External connection</legend>
          <div className="coordinator-note">
            <strong>Docker on this laptop</strong>
            <p>
              With Docker Desktop and the Orchid coordinator deployment running, connect directly
              using its published port and mounted access token. Orchid keeps the token private.
            </p>
            <button type="button" onClick={() => void apply(() => client.dockerSettings())}>
              Connect to laptop Docker
            </button>
          </div>
          <p>For another coordinator, enter its address and access token below.</p>
          <label>
            Coordinator URL
            <input
              type="url"
              required
              value={url}
              onChange={(event) => setUrl(event.currentTarget.value)}
            />
          </label>
          <label>
            Coordinator access token
            <input
              type="password"
              autoComplete="off"
              value={token}
              placeholder={
                overview.mode === 'external' && url === overview.endpoint
                  ? 'Leave blank to retain the saved token'
                  : ''
              }
              onChange={(event) => setToken(event.currentTarget.value)}
            />
          </label>
          <p>
            Use HTTPS for a remote node. Loopback HTTP supports a Docker rehearsal on this laptop.
            This is the coordinator access token; you enter the Hetzner Cloud token in Add device.
          </p>
        </fieldset>
      )}
      {error && <p role="alert">{error}</p>}
      <footer>
        <button type="button" disabled={busy} onClick={back}>
          Cancel
        </button>
        <button disabled={busy}>
          {busy ? 'Testing connection…' : external ? 'Test and save' : 'Use local coordinator'}
        </button>
      </footer>
    </form>
  );
}
