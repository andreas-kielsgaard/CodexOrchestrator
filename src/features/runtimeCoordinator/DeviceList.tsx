import type {
  CoordinatorLocation,
  CoordinatorOverview,
} from '../../application/runtimeCoordinator/contracts';

export function DeviceList({
  overview,
  navigate,
}: {
  readonly overview: CoordinatorOverview;
  readonly navigate: (location: CoordinatorLocation) => void;
}) {
  const linked = new Set(
    overview.bindings
      .filter((binding) => binding.coordinatorId === overview.coordinatorId)
      .map((binding) => binding.executionDeviceId),
  );
  return (
    <>
      <div className="coordinator-page-heading">
        <div>
          <h2>Devices</h2>
          <p>Manage this laptop and connected Linux machines.</p>
        </div>
        <button
          type="button"
          disabled={!overview.coordinatorId || !!overview.error}
          onClick={() => navigate({ view: 'setup' })}
        >
          Add device
        </button>
      </div>
      <div className="coordinator-device-list">
        {overview.legacyDevices
          .filter((device) => !linked.has(device.deviceId))
          .map((device) => (
            <button
              className="coordinator-device-row"
              type="button"
              key={device.deviceId}
              onClick={() => navigate({ view: 'legacy', deviceId: device.deviceId })}
            >
              <span>
                <strong>{device.displayName}</strong>
                <small>
                  {device.deviceId === 'local' ? 'This laptop' : 'Existing execution device'} ·{' '}
                  {device.connectionSummary}
                </small>
              </span>
              <span>
                {device.activeLeases
                  ? `${device.activeLeases} active operations`
                  : 'No active work'}
              </span>
              <span aria-hidden="true">›</span>
            </button>
          ))}
        {overview.devices.map((device) => (
          <button
            className="coordinator-device-row"
            type="button"
            key={device.id}
            onClick={() => navigate({ view: 'device', deviceId: device.id })}
          >
            <span>
              <strong>{device.displayName}</strong>
              <small>Hetzner Linux · {device.server.address ?? device.server.name}</small>
            </span>
            <span>
              {device.observationError ? 'Observation unavailable' : device.server.status}
              <small>
                {device.operation
                  ? `${device.operation.command} · ${device.operation.status}`
                  : `Observed ${new Date(device.observedAt).toLocaleString()}`}
              </small>
            </span>
            <span aria-hidden="true">›</span>
          </button>
        ))}
      </div>
      {!overview.devices.length && (
        <p className="coordinator-note">
          {overview.error
            ? 'Choose Configure coordinator to connect locally or to Docker. Once connected, Add device sets up your Hetzner server.'
            : 'Coordinator connected. Choose Add device, connect your Hetzner Cloud account, and select an existing Linux server. Its detail page provides Refresh, Start and Stop.'}
        </p>
      )}
    </>
  );
}
