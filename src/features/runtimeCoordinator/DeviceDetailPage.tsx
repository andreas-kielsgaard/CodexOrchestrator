import { useEffect, useState } from 'react';
import type {
  CoordinatorDevice,
  RuntimeCoordinatorClient,
} from '../../application/runtimeCoordinator/contracts';

export function DeviceDetailPage({
  client,
  device,
  executionName,
  activeWork,
  update,
  back,
}: {
  readonly client: RuntimeCoordinatorClient;
  readonly device: CoordinatorDevice;
  readonly executionName: string | null;
  readonly activeWork: number;
  readonly update: (device: CoordinatorDevice) => void;
  readonly back: () => void;
}) {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [editing, setEditing] = useState(false);
  const [submissionUncertain, setSubmissionUncertain] = useState(false);
  const [name, setName] = useState(device.displayName);
  const active = device.operation?.status === 'accepted' || device.operation?.status === 'running';
  useEffect(() => {
    let mounted = true;
    const observe = async () => {
      try {
        const next = await client.refresh(device.id);
        if (mounted) {
          update(next);
          setError(null);
        }
      } catch (cause) {
        if (mounted) setError(String(cause));
      }
    };
    void observe();
    const timer = active ? window.setInterval(() => void observe(), 3000) : undefined;
    return () => {
      mounted = false;
      if (timer) window.clearInterval(timer);
    };
  }, [client, device.id, active, update]);
  const run = async (task: () => Promise<void>) => {
    setBusy(true);
    setError(null);
    try {
      await task();
    } catch (cause) {
      setError(String(cause));
    } finally {
      setBusy(false);
    }
  };
  const power = (command: 'start' | 'stop') =>
    run(async () => {
      let operation;
      try {
        operation = await client.power(device.id, { operationId: crypto.randomUUID(), command });
      } catch (cause) {
        setSubmissionUncertain(true);
        throw cause;
      }
      setSubmissionUncertain(false);
      update({ ...device, operation });
      update(await client.refresh(device.id));
    });
  const unresolved = device.operation?.status === 'unconfirmed';
  return (
    <section className="coordinator-detail">
      <button type="button" className="coordinator-back" onClick={back}>
        ‹ Devices
      </button>
      <div className="coordinator-page-heading">
        <div>
          <h2>{device.displayName}</h2>
          <p>
            Hetzner Linux · {device.server.name} · #{device.server.id}
          </p>
        </div>
        <button
          type="button"
          disabled={busy}
          onClick={() =>
            void run(async () => {
              update(await client.refresh(device.id));
              setSubmissionUncertain(false);
            })
          }
        >
          Refresh
        </button>
      </div>
      <dl className="coordinator-status-grid">
        <div>
          <dt>Power state</dt>
          <dd>{device.server.status}</dd>
        </div>
        <div>
          <dt>Address</dt>
          <dd>{device.server.address ?? 'Unavailable'}</dd>
        </div>
        <div>
          <dt>Location</dt>
          <dd>{device.server.location ?? 'Unavailable'}</dd>
        </div>
        <div>
          <dt>Last observed</dt>
          <dd>{new Date(device.observedAt).toLocaleString()}</dd>
        </div>
        <div>
          <dt>Credential</dt>
          <dd>{device.credentialConfigured ? 'Configured on coordinator' : 'Missing'}</dd>
        </div>
        <div>
          <dt>Execution setup</dt>
          <dd>{executionName ?? 'Not configured'}</dd>
        </div>
      </dl>
      <div className="coordinator-actions">
        <button
          type="button"
          disabled={
            busy ||
            active ||
            unresolved ||
            submissionUncertain ||
            !!device.observationError ||
            device.server.status !== 'off'
          }
          onClick={() => void power('start')}
        >
          Start
        </button>
        <button
          type="button"
          disabled={
            busy ||
            active ||
            unresolved ||
            submissionUncertain ||
            !!device.observationError ||
            activeWork > 0 ||
            device.server.status !== 'running'
          }
          onClick={() => void power('stop')}
        >
          Stop
        </button>
        <button
          type="button"
          disabled={busy}
          onClick={() => {
            setName(device.displayName);
            setEditing(!editing);
          }}
        >
          Edit setup
        </button>
      </div>
      <p>
        Stop requests a graceful shutdown. Power controls are explicit; running power state alone
        does not indicate that an agent is ready.
      </p>
      {submissionUncertain && (
        <p role="alert">
          The coordinator did not confirm this submission. Refresh before issuing another power
          request.
        </p>
      )}
      {activeWork > 0 && <p>{activeWork} Orchid-managed operations active. Stop is unavailable.</p>}
      {device.operation && (
        <div className="coordinator-operation" role="status">
          <strong>
            {device.operation.command === 'start' ? 'Start' : 'Stop'} · {device.operation.status}
          </strong>
          <p>
            {device.operation.detail ??
              (active
                ? 'Waiting for the provider action to finish.'
                : 'Refresh to observe the current power state.')}
          </p>
          {device.operation.providerActionId && (
            <small>Provider action #{device.operation.providerActionId}</small>
          )}
        </div>
      )}
      {unresolved && (
        <p role="alert">
          The provider did not confirm this request. Refresh checks the resulting power state;
          Orchid will not resubmit it automatically.
        </p>
      )}
      {device.observationError && <p role="alert">{device.observationError}</p>}
      {error && <p role="alert">{error}</p>}
      {editing && (
        <form
          className="coordinator-form"
          onSubmit={(event) => {
            event.preventDefault();
            void run(async () => {
              update(await client.rename(device.id, name.trim()));
              setEditing(false);
            });
          }}
        >
          <label>
            Display name
            <input value={name} onChange={(event) => setName(event.currentTarget.value)} />
          </label>
          <button disabled={busy || !name.trim()}>Save setup</button>
        </form>
      )}
    </section>
  );
}
