import { useState } from 'react';
import { ModalDialog } from '../../components/ModalDialog';
import type {
  DeviceCommandSpecDto,
  ExecutionDeviceConfigurationDto,
} from '../../application/executionTargets/contracts';
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

export function LegacyDeviceSettings({
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
          arguments: draft.arguments
            .split('\n')
            .map((value) => value.trim())
            .filter(Boolean),
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
    <ModalDialog
      labelledBy="execution-device-dialog-title"
      onClose={onClose}
      dismissible={!busy}
      className="execution-device-dialog"
    >
      <header>
        <h2 id="execution-device-dialog-title">{device ? 'Configure device' : 'Add device'}</h2>
        <p>
          Lifecycle programs run on this laptop. Store credentials in the program's environment or
          operating-system credential store.
        </p>
      </header>
      <label>
        <span>Device ID</span>
        <input
          value={deviceId}
          disabled={device !== null}
          onChange={(event) => setDeviceId(event.currentTarget.value)}
        />
      </label>
      <label>
        <span>Display name</span>
        <input
          value={displayName}
          onChange={(event) => setDisplayName(event.currentTarget.value)}
        />
      </label>
      <LifecycleCommandEditor label="Start program" value={start} onChange={setStart} />
      <LifecycleCommandEditor label="Stop program" value={stop} onChange={setStop} />
      <fieldset>
        <label>
          <input
            type="checkbox"
            checked={idleEnabled}
            onChange={(event) => setIdleEnabled(event.currentTarget.checked)}
          />{' '}
          Automatically stop after Orchid-managed work becomes idle
        </label>
        {idleEnabled ? (
          <label>
            <span>Idle minutes</span>
            <input
              type="number"
              min={1}
              max={43200}
              value={idleMinutes}
              onChange={(event) => setIdleMinutes(Number(event.currentTarget.value))}
            />
          </label>
        ) : null}
        <p>
          Only Orchid-managed agent work counts as activity. Manual work on the server does not
          postpone shutdown.
        </p>
      </fieldset>
      {error ? <p role="alert">{error}</p> : null}
      <footer>
        <button type="button" onClick={onClose} disabled={busy}>
          Cancel
        </button>
        <button
          type="button"
          disabled={busy || !deviceId.trim() || !displayName.trim()}
          onClick={() => void save()}
        >
          {busy ? 'Saving…' : 'Save device'}
        </button>
      </footer>
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
      <label>
        <input
          type="checkbox"
          checked={value.enabled}
          onChange={(event) => onChange({ ...value, enabled: event.currentTarget.checked })}
        />{' '}
        {label}
      </label>
      {value.enabled ? (
        <>
          <label>
            <span>Absolute program path</span>
            <input
              value={value.program}
              onChange={(event) => onChange({ ...value, program: event.currentTarget.value })}
            />
          </label>
          <label>
            <span>Arguments, one per line</span>
            <textarea
              rows={3}
              value={value.arguments}
              onChange={(event) => onChange({ ...value, arguments: event.currentTarget.value })}
            />
          </label>
          <label>
            <span>Working directory (optional)</span>
            <input
              value={value.workingDirectory}
              onChange={(event) =>
                onChange({ ...value, workingDirectory: event.currentTarget.value })
              }
            />
          </label>
          <label>
            <span>Timeout seconds</span>
            <input
              type="number"
              min={1}
              max={900}
              value={value.timeoutSeconds}
              onChange={(event) =>
                onChange({ ...value, timeoutSeconds: Number(event.currentTarget.value) })
              }
            />
          </label>
        </>
      ) : null}
    </fieldset>
  );
}
