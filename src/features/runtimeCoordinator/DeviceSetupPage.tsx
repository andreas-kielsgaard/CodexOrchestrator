import { useState } from 'react';
import type {
  CoordinatorDevice,
  CoordinatorOverview,
  DeviceSetupDraft,
  RuntimeCoordinatorClient,
} from '../../application/runtimeCoordinator/contracts';

export function DeviceSetupPage({
  client,
  overview,
  draft,
  changeDraft,
  cancel,
  saved,
}: {
  readonly client: RuntimeCoordinatorClient;
  readonly overview: CoordinatorOverview;
  readonly draft: DeviceSetupDraft;
  readonly changeDraft: (draft: DeviceSetupDraft) => void;
  readonly cancel: () => void;
  readonly saved: (device: CoordinatorDevice) => void;
}) {
  const [token, setToken] = useState('');
  const discovery = draft.discovery ?? null;
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const connect = async () => {
    setBusy(true);
    setError(null);
    try {
      const found = await client.connect(token.trim());
      changeDraft({ ...draft, discovery: found });
      setToken('');
      if (!found.servers.length) setError('This account has no existing servers.');
    } catch (cause) {
      setError(String(cause));
    } finally {
      setBusy(false);
    }
  };
  const save = async () => {
    if (!discovery) return;
    setBusy(true);
    setError(null);
    try {
      saved(
        await client.register({
          credentialId: discovery.credentialId,
          serverId: Number(draft.serverId),
          displayName: draft.displayName.trim(),
          executionDeviceId: draft.executionDeviceId || null,
        }),
      );
    } catch (cause) {
      setError(String(cause));
    } finally {
      setBusy(false);
    }
  };
  const server = discovery?.servers.find((item) => String(item.id) === draft.serverId);
  const linked = new Set(overview.bindings.map((binding) => binding.executionDeviceId));
  return (
    <section className="coordinator-form">
      <button type="button" className="coordinator-back" onClick={cancel} disabled={busy}>
        ‹ Devices
      </button>
      <h2>Add Linux device</h2>
      <p>
        Connect your Hetzner account, choose an existing Linux server, then add it to view status
        and test Start and Stop. Adding a device leaves its power state unchanged.
      </p>
      <p className="coordinator-note">
        {overview.mode === 'local'
          ? 'The local coordinator on this laptop'
          : 'The external coordinator'}{' '}
        will store the credential and manage this device.
      </p>
      <fieldset disabled={busy}>
        <legend>1. Connect to Hetzner</legend>
        {discovery ? (
          <p>Connected · {discovery.servers.length} servers available</p>
        ) : (
          <>
            <label>
              Hetzner Cloud API token
              <input
                type="password"
                autoComplete="off"
                value={token}
                onChange={(event) => setToken(event.currentTarget.value)}
              />
            </label>
            <button
              type="button"
              disabled={!token.trim() || !overview.coordinatorId || !!overview.error}
              onClick={() => void connect()}
            >
              {busy ? 'Connecting…' : 'Connect'}
            </button>
            <p>
              Use a Hetzner Cloud project API token with read/write access. Connect tests the token
              by listing servers. It does not change their power state.
            </p>
          </>
        )}
      </fieldset>
      {discovery && (
        <fieldset disabled={busy}>
          <legend>2. Choose the server</legend>
          <label>
            Server
            <select
              value={draft.serverId}
              onChange={(event) => {
                const selected = discovery?.servers.find(
                  (item) => String(item.id) === event.currentTarget.value,
                );
                changeDraft({
                  ...draft,
                  serverId: event.currentTarget.value,
                  displayName: selected?.name ?? '',
                });
              }}
            >
              <option value="">Select an existing server</option>
              {discovery?.servers.map((item) => (
                <option key={item.id} value={item.id}>
                  {item.name} · {item.location ?? 'Unknown location'} · {item.status}
                </option>
              ))}
            </select>
          </label>
          {server && (
            <p>
              {server.address ?? 'No public IPv4 address'} · Hetzner server #{server.id}
            </p>
          )}
          <label>
            Display name
            <input
              value={draft.displayName}
              onChange={(event) =>
                changeDraft({ ...draft, displayName: event.currentTarget.value })
              }
            />
          </label>
          <label>
            Existing Orchid execution device (optional)
            <select
              value={draft.executionDeviceId}
              onChange={(event) =>
                changeDraft({ ...draft, executionDeviceId: event.currentTarget.value })
              }
            >
              <option value="">Power controls only</option>
              {overview.legacyDevices
                .filter((item) => item.deviceId !== 'local' && !linked.has(item.deviceId))
                .map((item) => (
                  <option key={item.deviceId} value={item.deviceId}>
                    {item.displayName}
                  </option>
                ))}
            </select>
          </label>
          <p>
            An association keeps the existing execution setup. Agent installation and sign-in remain
            separate.
          </p>
        </fieldset>
      )}
      {error && <p role="alert">{error}</p>}
      <footer>
        <button type="button" disabled={busy} onClick={cancel}>
          Cancel
        </button>
        <button
          type="button"
          disabled={busy || !server || !draft.displayName.trim()}
          onClick={() => void save()}
        >
          {busy ? 'Working…' : 'Add device'}
        </button>
      </footer>
    </section>
  );
}
