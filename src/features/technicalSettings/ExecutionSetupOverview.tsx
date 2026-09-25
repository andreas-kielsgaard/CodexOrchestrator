import { useEffect, useState } from 'react';
import { ModalDialog } from '../../components/ModalDialog';
import {
  agentProviderDescriptor,
  displayFolderPath,
  type ProviderSetupDto,
} from '../../application/agentProviders';
import type { ExecutionConfigurationClient } from '../../application/executionConfiguration';
import type {
  DeviceCommandSpecDto,
  ExecutionDeviceConfigurationDto,
  ExecutionTargetClient,
} from '../../application/executionTargets/contracts';

/** Every provider's setups; each provider owns its own setup screen. */
function useProviderSetups(client: ExecutionConfigurationClient | undefined) {
  const [setups, setSetups] = useState<readonly ProviderSetupDto[]>([]);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    let active = true;
    if (!client?.listProviderSetups) return;
    void client.listProviderSetups().then(
      (next) => {
        if (active) setSetups(next);
      },
      (cause) => {
        if (active) setError(cause instanceof Error ? cause.message : String(cause));
      },
    );
    return () => {
      active = false;
    };
  }, [client]);
  return { setups, error };
}

const SETUP_STATE_LABELS: Record<ProviderSetupDto['state'], string> = {
  ready: 'Ready',
  needs_login: 'Needs sign-in',
  unavailable: 'Unavailable',
};

export function DeviceSetupOverview({
  executionClient,
  deviceClient,
  providers,
  onConfigureProvider,
}: {
  readonly executionClient?: ExecutionConfigurationClient;
  readonly deviceClient?: ExecutionTargetClient;
  /** Providers with a setup screen in Technical Settings. */
  readonly providers: readonly string[];
  readonly onConfigureProvider: (provider: string) => void;
}) {
  const { setups, error } = useProviderSetups(executionClient);
  const [devices, setDevices] = useState<readonly ExecutionDeviceConfigurationDto[]>([]);
  const [deviceError, setDeviceError] = useState<string | null>(null);
  const [editing, setEditing] = useState<ExecutionDeviceConfigurationDto | 'new' | null>(null);
  const [busyDeviceId, setBusyDeviceId] = useState<string | null>(null);
  useEffect(() => {
    let active = true;
    if (!deviceClient?.listConfiguredDevices) return;
    void deviceClient.listConfiguredDevices().then(
      (configured) => {
        if (active) setDevices(configured);
      },
      (cause) => {
        if (active) setDeviceError(cause instanceof Error ? cause.message : String(cause));
      },
    );
    return () => {
      active = false;
    };
  }, [deviceClient]);
  const runLifecycle = async (deviceId: string, operation: 'start' | 'stop') => {
    const command =
      operation === 'start'
        ? deviceClient?.startConfiguredDevice
        : deviceClient?.stopConfiguredDevice;
    if (!command) return;
    setBusyDeviceId(deviceId);
    setDeviceError(null);
    try {
      await command(deviceId);
      setDevices((await deviceClient?.listConfiguredDevices?.()) ?? devices);
    } catch (cause) {
      setDeviceError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setBusyDeviceId(null);
    }
  };
  return (
    <section className="execution-setup-overview" aria-labelledby="device-setup-title">
      <header>
        <p>Execution setup</p>
        <h2 id="device-setup-title">Devices</h2>
        <span>{devices.length || 1} registered device{(devices.length || 1) === 1 ? '' : 's'}</span>
      </header>
      <p>
        Configure how Orchid prepares and retires execution devices. Connection details and
        provider credentials remain developer-managed outside this screen.
      </p>
      {deviceClient?.saveConfiguredDevice ? (
        <button className="execution-setup-overview__add" type="button" onClick={() => setEditing('new')}>
          Add device
        </button>
      ) : null}
      {devices.map((device) => (
        <section className="execution-setup-overview__card" key={device.deviceId}>
          <div>
            <p>Device</p>
            <h3>{device.displayName}</h3>
            <span>{device.connectionSummary}</span>
          </div>
          <div className="execution-setup-overview__actions">
            <button type="button" onClick={() => setEditing(device)}>Configure</button>
            {device.lifecycle.start ? (
              <button type="button" disabled={busyDeviceId === device.deviceId} onClick={() => void runLifecycle(device.deviceId, 'start')}>
                Start
              </button>
            ) : null}
            {device.lifecycle.stop ? (
              <button type="button" disabled={busyDeviceId === device.deviceId || device.activeLeases > 0} onClick={() => void runLifecycle(device.deviceId, 'stop')}>
                Stop
              </button>
            ) : null}
            {device.lifecycle.idleShutdownSeconds && deviceClient?.holdConfiguredDeviceAwake ? (
              <button
                type="button"
                disabled={busyDeviceId === device.deviceId}
                onClick={async () => {
                  setBusyDeviceId(device.deviceId);
                  try {
                    setDevices(await deviceClient.holdConfiguredDeviceAwake!(device.deviceId, 3600));
                  } catch (cause) {
                    setDeviceError(cause instanceof Error ? cause.message : String(cause));
                  } finally {
                    setBusyDeviceId(null);
                  }
                }}
              >
                Keep awake 1 hour
              </button>
            ) : null}
          </div>
          <p>
            {device.activeLeases
              ? `${device.activeLeases} Orchid-managed operation${device.activeLeases === 1 ? '' : 's'} active.`
              : 'No Orchid-managed work is active.'}
            {device.lifecycle.idleShutdownSeconds
              ? ` Automatic shutdown is configured after ${Math.round(device.lifecycle.idleShutdownSeconds / 60)} idle minutes.`
              : ' Automatic shutdown is off.'}
            {device.keepAwakeUntil ? ` Keep-awake hold: ${new Date(device.keepAwakeUntil).toLocaleString()}.` : ''}
            {device.lastLifecycleMessage ? ` ${device.lastLifecycleMessage}.` : ''}
          </p>
        </section>
      ))}
      {setups.map((setup) => {
        const descriptor = agentProviderDescriptor(setup.provider);
        const key = `${setup.deviceId}-${setup.provider}-${setup.configurationId}`;
        return (
          <section
            className="execution-setup-overview__card"
            aria-labelledby={`harness-${key}`}
            key={key}
          >
            <div>
              <p>Harness</p>
              <h3 id={`harness-${key}`}>
                {descriptor.harnessLabel}
                {setup.selected ? ' · selected' : ''}
              </h3>
              <span>
                {setup.deviceId === 'local' ? 'Runs on this device' : `Runs on ${setup.deviceId}`} ·{' '}
                {SETUP_STATE_LABELS[setup.state]}
              </span>
            </div>
            {providers.includes(setup.provider) ? (
              <button type="button" onClick={() => onConfigureProvider(setup.provider)}>
                Configure this harness
              </button>
            ) : null}
            <p>
              {displayFolderPath(setup.folder)}
              {setup.executable ? ` · ${setup.executable}` : ''} · connected inference source:{' '}
              {descriptor.inferenceLabel}. Account credentials remain in this device-local{' '}
              {descriptor.configurationLabel}.{setup.detail ? ` ${setup.detail}` : ''}
            </p>
          </section>
        );
      })}
      {!setups.length && !error ? (
        <section className="execution-setup-overview__card" aria-label="No configured harnesses">
          <div>
            <p>Harness</p>
            <h3>No harness configured</h3>
          </div>
          {providers.map((provider) => (
            <button key={provider} type="button" onClick={() => onConfigureProvider(provider)}>
              Add local {agentProviderDescriptor(provider).harnessLabel} harness
            </button>
          ))}
          <p>Add a provider setup to create the first local harness.</p>
        </section>
      ) : null}
      {error ? (
        <p className="execution-setup-overview__error">Could not load harnesses: {error}</p>
      ) : null}
      {deviceError ? <p className="execution-setup-overview__error" role="alert">{deviceError}</p> : null}
      {editing && deviceClient?.saveConfiguredDevice ? (
        <DeviceConfigurationDialog
          device={editing === 'new' ? null : editing}
          onClose={() => setEditing(null)}
          onSave={async (input) => {
            setDeviceError(null);
            try {
              setDevices(await deviceClient.saveConfiguredDevice!(input));
              setEditing(null);
            } catch (cause) {
              setDeviceError(cause instanceof Error ? cause.message : String(cause));
              throw cause;
            }
          }}
        />
      ) : null}
    </section>
  );
}

type CommandDraft = {
  enabled: boolean;
  program: string;
  arguments: string;
  workingDirectory: string;
  timeoutSeconds: number;
};

function commandDraft(command: DeviceCommandSpecDto | null): CommandDraft {
  return {
    enabled: command !== null,
    program: command?.program ?? '',
    arguments: command?.arguments.join('\n') ?? '',
    workingDirectory: command?.workingDirectory ?? '',
    timeoutSeconds: command?.timeoutSeconds ?? 120,
  };
}

function DeviceConfigurationDialog({
  device,
  onClose,
  onSave,
}: {
  readonly device: ExecutionDeviceConfigurationDto | null;
  readonly onClose: () => void;
  readonly onSave: (input: {
    readonly deviceId: string;
    readonly displayName: string;
    readonly lifecycle: ExecutionDeviceConfigurationDto['lifecycle'];
  }) => Promise<void>;
}) {
  const [deviceId, setDeviceId] = useState(device?.deviceId ?? '');
  const [displayName, setDisplayName] = useState(device?.displayName ?? '');
  const [start, setStart] = useState(() => commandDraft(device?.lifecycle.start ?? null));
  const [stop, setStop] = useState(() => commandDraft(device?.lifecycle.stop ?? null));
  const [idleEnabled, setIdleEnabled] = useState(device?.lifecycle.idleShutdownSeconds != null);
  const [idleMinutes, setIdleMinutes] = useState(
    Math.round((device?.lifecycle.idleShutdownSeconds ?? 3600) / 60),
  );
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const encodeCommand = (draft: CommandDraft): DeviceCommandSpecDto | null =>
    draft.enabled
      ? {
          program: draft.program.trim(),
          arguments: draft.arguments.split('\n').map((value) => value.trim()).filter(Boolean),
          workingDirectory: draft.workingDirectory.trim() || null,
          timeoutSeconds: draft.timeoutSeconds,
        }
      : null;
  const save = async () => {
    setBusy(true);
    setError(null);
    try {
      await onSave({
        deviceId: deviceId.trim(),
        displayName: displayName.trim(),
        lifecycle: {
          start: encodeCommand(start),
          stop: encodeCommand(stop),
          idleShutdownSeconds: idleEnabled ? idleMinutes * 60 : null,
        },
      });
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setBusy(false);
    }
  };
  return (
    <ModalDialog labelledBy="execution-device-dialog-title" onClose={onClose} dismissible={!busy} className="execution-device-dialog">
      <header>
        <h2 id="execution-device-dialog-title">{device ? 'Configure device' : 'Add device'}</h2>
        <p>Lifecycle programs run on this laptop. Store credentials in the program's environment or operating-system credential store.</p>
      </header>
      <label><span>Device ID</span><input value={deviceId} disabled={device !== null} onChange={(event) => setDeviceId(event.currentTarget.value)} /></label>
      <label><span>Display name</span><input value={displayName} onChange={(event) => setDisplayName(event.currentTarget.value)} /></label>
      <LifecycleCommandEditor label="Start program" value={start} onChange={setStart} />
      <LifecycleCommandEditor label="Stop program" value={stop} onChange={setStop} />
      <fieldset>
        <label><input type="checkbox" checked={idleEnabled} onChange={(event) => setIdleEnabled(event.currentTarget.checked)} /> Automatically stop after Orchid-managed work becomes idle</label>
        {idleEnabled ? <label><span>Idle minutes</span><input type="number" min={1} max={43200} value={idleMinutes} onChange={(event) => setIdleMinutes(Number(event.currentTarget.value))} /></label> : null}
        <p>Only Orchid-managed agent work counts as activity. Manual work on the server does not postpone shutdown.</p>
      </fieldset>
      {error ? <p role="alert">{error}</p> : null}
      <footer><button type="button" onClick={onClose} disabled={busy}>Cancel</button><button type="button" disabled={busy || !deviceId.trim() || !displayName.trim()} onClick={() => void save()}>{busy ? 'Saving…' : 'Save device'}</button></footer>
    </ModalDialog>
  );
}

function LifecycleCommandEditor({
  label,
  value,
  onChange,
}: {
  readonly label: string;
  readonly value: CommandDraft;
  readonly onChange: (value: CommandDraft) => void;
}) {
  return (
    <fieldset>
      <label><input type="checkbox" checked={value.enabled} onChange={(event) => onChange({ ...value, enabled: event.currentTarget.checked })} /> {label}</label>
      {value.enabled ? <>
        <label><span>Absolute program path</span><input value={value.program} onChange={(event) => onChange({ ...value, program: event.currentTarget.value })} /></label>
        <label><span>Arguments, one per line</span><textarea rows={3} value={value.arguments} onChange={(event) => onChange({ ...value, arguments: event.currentTarget.value })} /></label>
        <label><span>Working directory (optional)</span><input value={value.workingDirectory} onChange={(event) => onChange({ ...value, workingDirectory: event.currentTarget.value })} /></label>
        <label><span>Timeout seconds</span><input type="number" min={1} max={900} value={value.timeoutSeconds} onChange={(event) => onChange({ ...value, timeoutSeconds: Number(event.currentTarget.value) })} /></label>
      </> : null}
    </fieldset>
  );
}

export function InferenceSourceOverview({
  executionClient,
}: {
  readonly executionClient?: ExecutionConfigurationClient;
}) {
  const { setups, error } = useProviderSetups(executionClient);
  return (
    <section className="execution-setup-overview" aria-labelledby="inference-source-title">
      <header>
        <p>Execution setup</p>
        <h2 id="inference-source-title">Inference sources</h2>
        <span>{setups.length} connected locally</span>
      </header>
      <p>
        Sources name the account or provider path a harness may use. Orchid stores display metadata
        and bindings, never a provider account credential or API key.
      </p>
      {setups.map((setup) => {
        const descriptor = agentProviderDescriptor(setup.provider);
        const key = `${setup.deviceId}-${setup.provider}-${setup.configurationId}`;
        return (
          <section
            className="execution-setup-overview__card"
            aria-labelledby={`source-${key}`}
            key={key}
          >
            <div>
              <p>Inference source</p>
              <h3 id={`source-${key}`}>{descriptor.inferenceLabel}</h3>
              <span>
                Connected to {descriptor.harnessLabel} · {displayFolderPath(setup.folder)}
              </span>
            </div>
            <p>
              This is the source binding for one harness configuration. A second account needs its
              own authenticated {descriptor.configurationLabel} and therefore a separate harness
              connection.
            </p>
          </section>
        );
      })}
      {!setups.length && !error ? (
        <p className="execution-setup-overview__note">
          No local source is available until a local harness is configured.
        </p>
      ) : null}
      {error ? (
        <p className="execution-setup-overview__error">Could not load sources: {error}</p>
      ) : null}
    </section>
  );
}
