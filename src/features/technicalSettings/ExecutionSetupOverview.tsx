import { useEffect, useState } from 'react';
import { ModalDialog } from '../../components/ModalDialog';
import {
  displayCodexHomePath,
  localCodexRoutes,
} from '../../application/executionConfiguration';
import type {
  NativeProfile,
  NativeProfileClient,
} from '../../infrastructure/nativeProfiles/nativeProfileClient';
import type {
  DeviceCommandSpecDto,
  ExecutionDeviceConfigurationDto,
  ExecutionTargetClient,
} from '../../application/executionTargets/contracts';

function useLocalHarnesses(client: NativeProfileClient) {
  const [harnesses, setHarnesses] = useState<readonly NativeProfile[]>([]);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    let active = true;
    void client.load().then(
      (query) => {
        if (active) {
          const routeIds = new Set(
            localCodexRoutes(query.profiles).map((route) => route.execution.configurationRef),
          );
          setHarnesses(query.profiles.filter((profile) => routeIds.has(profile.id)));
        }
      },
      (cause) => {
        if (active) setError(cause instanceof Error ? cause.message : String(cause));
      },
    );
    return () => {
      active = false;
    };
  }, [client]);
  return { harnesses, error };
}

export function DeviceSetupOverview({
  nativeClient,
  deviceClient,
  onOpenCodexHarness,
}: {
  readonly nativeClient: NativeProfileClient;
  readonly deviceClient?: ExecutionTargetClient;
  readonly onOpenCodexHarness: () => void;
}) {
  const { harnesses, error } = useLocalHarnesses(nativeClient);
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
      {harnesses.map((harness) => (
        <section
          className="execution-setup-overview__card"
          aria-labelledby={`local-harness-${harness.id}`}
          key={harness.id}
        >
          <div>
            <p>Harness</p>
            <h3 id={`local-harness-${harness.id}`}>
              Codex CLI{harness.selected ? ' · selected' : ''}
            </h3>
            <span>Runs on this device</span>
          </div>
          <button type="button" onClick={onOpenCodexHarness}>
            Configure this harness
          </button>
          <p>
            {displayCodexHomePath(harness.homePath)} · connected inference source: OpenAI via this Codex CLI
            configuration. Account credentials remain in this device-local Codex profile.
          </p>
        </section>
      ))}
      {!harnesses.length && !error ? (
        <section className="execution-setup-overview__card" aria-label="No configured harnesses">
          <div>
            <p>Harness</p>
            <h3>No Codex CLI harness configured</h3>
          </div>
          <button type="button" onClick={onOpenCodexHarness}>
            Add local Codex harness
          </button>
          <p>Add a Codex profile to create the first local harness.</p>
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
  nativeClient,
}: {
  readonly nativeClient: NativeProfileClient;
}) {
  const { harnesses, error } = useLocalHarnesses(nativeClient);
  return (
    <section className="execution-setup-overview" aria-labelledby="inference-source-title">
      <header>
        <p>Execution setup</p>
        <h2 id="inference-source-title">Inference sources</h2>
        <span>{harnesses.length} connected locally</span>
      </header>
      <p>
        Sources name the account or provider path a harness may use. Orchid stores display metadata
        and bindings, never a Codex account credential or API key.
      </p>
      {harnesses.map((harness) => (
        <section
          className="execution-setup-overview__card"
          aria-labelledby={`openai-source-${harness.id}`}
          key={harness.id}
        >
          <div>
            <p>Inference source</p>
            <h3 id={`openai-source-${harness.id}`}>OpenAI via Codex CLI</h3>
            <span>Connected to Codex CLI · {displayCodexHomePath(harness.homePath)}</span>
          </div>
          <p>
            This is the source binding for one harness configuration. A second account needs its own
            authenticated Codex profile and therefore a separate harness connection.
          </p>
        </section>
      ))}
      {!harnesses.length && !error ? (
        <p className="execution-setup-overview__note">
          No local source is available until a local Codex CLI harness is configured.
        </p>
      ) : null}
      {error ? (
        <p className="execution-setup-overview__error">Could not load sources: {error}</p>
      ) : null}
    </section>
  );
}
