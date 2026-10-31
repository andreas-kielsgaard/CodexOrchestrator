import { useCallback, useEffect, useState } from 'react';
import type {
  ExecutionTargetClient,
  ExecutionDeviceConfigurationDto,
} from '../../application/executionTargets/contracts';
import type {
  CoordinatorDevice,
  CoordinatorLocation,
  CoordinatorOverview,
  DeviceSetupDraft,
  RuntimeCoordinatorClient,
} from '../../application/runtimeCoordinator/contracts';
import { DeviceList } from './DeviceList';
import { DeviceSetupPage } from './DeviceSetupPage';
import { DeviceDetailPage } from './DeviceDetailPage';
import { CoordinatorSettingsPage } from './CoordinatorSettingsPage';
import { LegacyDeviceSettings } from './LegacyDeviceSettings';
import './runtimeCoordinator.css';

export function RuntimeCoordinatorScreen({
  client,
  executionClient,
  location,
  navigate,
  draft,
  changeDraft,
}: {
  readonly client: RuntimeCoordinatorClient;
  readonly executionClient?: ExecutionTargetClient;
  readonly location: CoordinatorLocation;
  readonly navigate: (location: CoordinatorLocation) => void;
  readonly draft: DeviceSetupDraft | null;
  readonly changeDraft: (draft: DeviceSetupDraft | null) => void;
}) {
  const [overview, setOverview] = useState<CoordinatorOverview | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [editing, setEditing] = useState<ExecutionDeviceConfigurationDto | null>(null);
  const [busy, setBusy] = useState(false);
  const reload = useCallback(async () => {
    try {
      setOverview(await client.overview());
      setError(null);
    } catch (cause) {
      setError(String(cause));
    }
  }, [client]);
  useEffect(() => {
    let active = true;
    void client.overview().then(
      (next) => {
        if (active) setOverview(next);
      },
      (cause) => {
        if (active) setError(String(cause));
      },
    );
    return () => {
      active = false;
    };
  }, [client]);
  const update = useCallback(
    (device: CoordinatorDevice) =>
      setOverview((current) =>
        current
          ? {
              ...current,
              devices: current.devices.map((item) => (item.id === device.id ? device : item)),
            }
          : current,
      ),
    [],
  );
  const back = () => navigate({ view: 'list' });
  const legacy =
    overview && 'deviceId' in location && location.view === 'legacy'
      ? overview.legacyDevices.find((item) => item.deviceId === location.deviceId)
      : null;
  const device =
    overview && 'deviceId' in location && location.view === 'device'
      ? overview.devices.find((item) => item.id === location.deviceId)
      : null;
  const binding = overview?.bindings.find((item) => item.deviceId === device?.id);
  const execution = overview?.legacyDevices.find(
    (item) => item.deviceId === binding?.executionDeviceId,
  );
  const lifecycle = async (operation: 'start' | 'stop') => {
    if (!legacy) return;
    setBusy(true);
    setError(null);
    try {
      await (operation === 'start'
        ? executionClient?.startConfiguredDevice?.(legacy.deviceId)
        : executionClient?.stopConfiguredDevice?.(legacy.deviceId));
      await reload();
    } catch (cause) {
      setError(String(cause));
    } finally {
      setBusy(false);
    }
  };
  return (
    <main className="runtime-coordinator">
      <header className="coordinator-page-heading">
        <div>
          <h1>Runtime Coordinator</h1>
          <p>
            {overview
              ? `${overview.mode === 'local' ? 'Local on this laptop' : overview.endpoint} · ${overview.error ? 'Disconnected' : 'Connected'}`
              : 'Connecting to coordinator…'}
          </p>
        </div>
        {overview && (
          <button type="button" onClick={() => navigate({ view: 'settings' })}>
            Configure coordinator
          </button>
        )}
      </header>
      {(error || overview?.error) && (
        <div className="coordinator-error" role="alert">
          {error ?? overview?.error}
          <button type="button" onClick={() => void reload()}>
            Retry
          </button>
        </div>
      )}
      {overview && location.view === 'list' && (
        <DeviceList overview={overview} navigate={navigate} />
      )}
      {overview && location.view === 'settings' && (
        <CoordinatorSettingsPage
          client={client}
          overview={overview}
          back={back}
          saved={(next) => {
            setOverview(next);
            changeDraft(null);
            back();
          }}
        />
      )}
      {overview && location.view === 'setup' && (
        <DeviceSetupPage
          key={overview.coordinatorId}
          client={client}
          overview={overview}
          draft={
            draft?.coordinatorId === overview.coordinatorId
              ? draft
              : {
                  coordinatorId: overview.coordinatorId ?? '',
                  displayName: '',
                  serverId: '',
                  executionDeviceId: '',
                }
          }
          changeDraft={changeDraft}
          cancel={() => {
            changeDraft(null);
            back();
          }}
          saved={(next) => {
            setOverview({ ...overview, devices: [...overview.devices, next] });
            changeDraft(null);
            void reload();
            navigate({ view: 'device', deviceId: next.id });
          }}
        />
      )}
      {device && (
        <DeviceDetailPage
          key={device.id}
          client={client}
          device={device}
          executionName={execution?.displayName ?? null}
          activeWork={execution?.activeLeases ?? 0}
          update={update}
          back={back}
        />
      )}
      {legacy && (
        <section className="coordinator-detail">
          <button type="button" className="coordinator-back" onClick={back}>
            ‹ Devices
          </button>
          <h2>{legacy.displayName}</h2>
          <p>{legacy.connectionSummary}</p>
          <p>{legacy.activeLeases} Orchid-managed operations active.</p>
          {legacy.deviceId === 'local' ? (
            <p>Execution runs on this laptop.</p>
          ) : overview?.bindings.some((item) => item.executionDeviceId === legacy.deviceId) ? (
            <p>
              Power controls belong to another coordinator. Select that coordinator in Configure
              coordinator to manage this device.
            </p>
          ) : (
            <>
              <p>
                Existing command-managed device. Link it when adding its Hetzner server to use
                coordinator power controls.
              </p>
              <div className="coordinator-actions">
                <button type="button" onClick={() => setEditing(legacy)}>
                  Edit setup
                </button>
                {legacy.lifecycle.start && (
                  <button type="button" disabled={busy} onClick={() => void lifecycle('start')}>
                    Start
                  </button>
                )}
                {legacy.lifecycle.stop && (
                  <button
                    type="button"
                    disabled={busy || legacy.activeLeases > 0}
                    onClick={() => void lifecycle('stop')}
                  >
                    Stop
                  </button>
                )}
                {legacy.lifecycle.idleShutdownSeconds &&
                  executionClient?.holdConfiguredDeviceAwake && (
                    <button
                      type="button"
                      disabled={busy}
                      onClick={async () => {
                        setBusy(true);
                        setError(null);
                        try {
                          await executionClient.holdConfiguredDeviceAwake!(legacy.deviceId, 3600);
                          await reload();
                        } catch (cause) {
                          setError(String(cause));
                        } finally {
                          setBusy(false);
                        }
                      }}
                    >
                      Keep awake 1 hour
                    </button>
                  )}
              </div>
              <p>
                {legacy.lifecycle.idleShutdownSeconds
                  ? `Automatic shutdown after ${Math.round(legacy.lifecycle.idleShutdownSeconds / 60)} idle minutes.`
                  : 'Automatic shutdown is off.'}
              </p>
              <p>{legacy.lastLifecycleMessage}</p>
            </>
          )}
        </section>
      )}
      {overview && 'deviceId' in location && !legacy && !device && (
        <p>
          Device not found.{' '}
          <button type="button" onClick={back}>
            Return to devices
          </button>
        </p>
      )}
      {editing && executionClient?.saveConfiguredDevice && (
        <LegacyDeviceSettings
          device={editing}
          onClose={() => setEditing(null)}
          onSave={async (input) => {
            await executionClient.saveConfiguredDevice!(input);
            setEditing(null);
            await reload();
          }}
        />
      )}
    </main>
  );
}
