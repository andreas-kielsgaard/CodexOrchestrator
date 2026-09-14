import {
  localExecutionBinding,
  type ExecutionBindingDto,
} from '../../application/executionTargets/contracts';
export function ExecutionConnectionFields({
  value = localExecutionBinding,
  onChange,
}: {
  readonly value?: ExecutionBindingDto;
  readonly onChange: (value: ExecutionBindingDto) => void;
}) {
  return (
    <div className="execution-connection-fields">
      <label className="execution-configuration__field">
        <span>Execution device</span>
        <select
          aria-label="Execution connection"
          value={value.connection.kind}
          onChange={(event) =>
            onChange(
              event.target.value === 'local'
                ? localExecutionBinding
                : {
                    ...value,
                    deviceId: '',
                    deviceName: '',
                    configurationRef: 'codex-default',
                    connection: {
                      kind: 'ssh',
                      target: '',
                      hostExecutable: '/root/.local/bin/orchid-host',
                    },
                  },
            )
          }
        >
          <option value="local">This laptop</option>
          <option value="ssh">Remote device over SSH</option>
        </select>
      </label>
      <label className="execution-configuration__field">
        <span>Device name</span>
        <input
          aria-label="Device name"
          value={value.deviceName}
          onChange={(event) => onChange({ ...value, deviceName: event.target.value })}
        />
      </label>
      <label className="execution-configuration__field">
        <span>Device ID</span>
        <input
          aria-label="Device ID"
          value={value.deviceId}
          disabled={value.connection.kind === 'local'}
          onChange={(event) => onChange({ ...value, deviceId: event.target.value })}
        />
        <small>Profiles for the same device use the same ID.</small>
      </label>
      {value.connection.kind === 'ssh' && (
        <>
          <label className="execution-configuration__field">
            <span>SSH target or alias</span>
            <input
              aria-label="SSH target"
              value={value.connection.target}
              placeholder="orchid-remote"
              onChange={(event) =>
                value.connection.kind === 'ssh' &&
                onChange({
                  ...value,
                  connection: { ...value.connection, target: event.target.value },
                })
              }
            />
          </label>
          <label className="execution-configuration__field">
            <span>Orchid host executable on device</span>
            <input
              aria-label="Remote host executable"
              value={value.connection.hostExecutable}
              onChange={(event) =>
                value.connection.kind === 'ssh' &&
                onChange({
                  ...value,
                  connection: { ...value.connection, hostExecutable: event.target.value },
                })
              }
            />
          </label>
        </>
      )}
      <label className="execution-configuration__field">
        <span>Codex configuration</span>
        <input
          aria-label="Codex configuration reference"
          value={value.configurationRef}
          placeholder={value.connection.kind === 'local' ? 'selected' : 'codex-default'}
          onChange={(event) => onChange({ ...value, configurationRef: event.target.value })}
        />
        <small>
          {value.connection.kind === 'ssh'
            ? 'Named configuration on the Orchid host. Codex runs and signs in on that device.'
            : 'Use selected to follow the selected local Codex home, or enter a local native profile ID.'}
        </small>
      </label>
    </div>
  );
}
