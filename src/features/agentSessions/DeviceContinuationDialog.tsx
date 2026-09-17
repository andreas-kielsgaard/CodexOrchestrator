import type { RepositoryBranchSource, ReviewTarget } from '../../application/branches';
import type {
  ExecutionTargetClient,
  ExecutionTargetDeviceDto,
  SessionExecutionTargetDto,
  TargetWorktreeDto,
} from '../../application/executionTargets/contracts';
import { BranchGraphBrowser } from '../branches/BranchGraphBrowser';
import { ModalDialog } from '../../components/ModalDialog';
import { ArrowLeft, Check, Monitor, Server, X } from 'lucide-react';
import { useEffect, useMemo, useState } from 'react';
import './deviceContinuation.css';

export interface DeviceContinuationDialogProps {
  readonly client: ExecutionTargetClient;
  readonly source: RepositoryBranchSource;
  /** The fixed source worktree of the conversation being continued. */
  readonly sourceTarget: SessionExecutionTargetDto | null;
  readonly onChooseDevice?: (deviceId: string) => void;
  onClose(): void;
}

function shortCommit(value: string | null) {
  return value ? value.slice(0, 10) : 'unknown';
}

function DeviceIcon({ device }: { readonly device: ExecutionTargetDeviceDto }) {
  return device.profiles.some((profile) => profile.execution.connection.kind === 'ssh') ? (
    <Server size={17} aria-hidden="true" />
  ) : (
    <Monitor size={17} aria-hidden="true" />
  );
}

function TargetState({
  source,
  target,
}: {
  readonly source: SessionExecutionTargetDto;
  readonly target: TargetWorktreeDto;
}) {
  if (!target.head)
    return (
      <span className="device-continuation__state device-continuation__state--unknown">
        HEAD unavailable
      </span>
    );
  if (target.head === source.head)
    return (
      <span className="device-continuation__state device-continuation__state--same">
        Matches source HEAD
      </span>
    );
  return (
    <span className="device-continuation__state device-continuation__state--different">
      Different HEAD
    </span>
  );
}

/**
 * The service currently reports worktree heads, but not sister relationships, dirty state,
 * active turns, or snapshot migration.
 */
export function DeviceContinuationDialog({
  client,
  source,
  sourceTarget,
  onChooseDevice,
  onClose,
}: DeviceContinuationDialogProps) {
  const [devices, setDevices] = useState<readonly ExecutionTargetDeviceDto[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string>();
  const [selectedDeviceId, setSelectedDeviceId] = useState<string>();
  const [step, setStep] = useState<'device' | 'worktrees'>('device');

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError(undefined);
    const result = sourceTarget
      ? client.listTargets(sourceTarget.repositoryId, sourceTarget.branchRef)
      : client.listDevices().then((configured) =>
          configured.map<ExecutionTargetDeviceDto>((device) => ({
            ...device,
            profiles: device.profiles.map((profile) => ({
              ...profile,
              instances: [],
              error: null,
            })),
          })),
        );
    void result
      .then(
        (result) => {
          if (!cancelled) setDevices(result);
        },
        (cause) => {
          if (!cancelled) setError(String(cause));
        },
      )
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [client, sourceTarget]);

  const selectedDevice = useMemo(
    () => devices.find((device) => device.deviceId === selectedDeviceId) ?? null,
    [devices, selectedDeviceId],
  );
  const fixedBranch = useMemo<ReviewTarget | null>(
    () =>
      sourceTarget
        ? {
            kind: 'branch',
            repositoryId: sourceTarget.repositoryId,
            branchRef: sourceTarget.branchRef,
          }
        : null,
    [sourceTarget],
  );
  const targetInstances =
    selectedDevice?.profiles.flatMap((profile) =>
      profile.instances.map((instance) => ({ instance, profile })),
    ) ?? [];
  const sourceBranch =
    sourceTarget?.branchRef.replace(/^refs\/heads\//, '') ?? 'No source worktree';

  return (
    <ModalDialog
      labelledBy="device-continuation-title"
      className="device-continuation-dialog"
      onClose={onClose}
    >
      <header className="device-continuation__header">
        <div>
          <p className="worktree-review__step">Continue this conversation elsewhere</p>
          <h2 id="device-continuation-title">Destination device</h2>
          <p>{sourceBranch}</p>
        </div>
        <button type="button" onClick={onClose} aria-label="Close device selector">
          <X size={18} />
        </button>
      </header>

      <ol className="device-continuation__steps" aria-label="Device continuation progress">
        <li className={step === 'device' ? 'is-current' : 'is-complete'}>
          <span>{step === 'device' ? '1' : <Check size={14} />}</span> Choose device
        </li>
        <li className={step === 'worktrees' ? 'is-current' : ''}>
          <span>2</span> Compare worktrees
        </li>
      </ol>

      {step === 'device' ? (
        <section className="device-continuation__device-step" aria-label="Configured devices">
          {sourceTarget && (
            <div className="device-continuation__source-strip">
              <div>
                <span>Current worktree</span>
                <strong>{sourceTarget.path}</strong>
              </div>
              <code>HEAD {shortCommit(sourceTarget.head)}</code>
            </div>
          )}
          <p className="device-continuation__guidance">
            {sourceTarget
              ? 'Each device shows whether it already has a worktree on this branch. Select a destination to inspect the worktrees before any future migration is requested.'
              : 'Choose an execution device. Select a target worktree next before sending a remote prompt.'}
          </p>
          {loading && <p role="status">Finding branch worktrees on configured devices…</p>}
          {error && <p role="alert">{error}</p>}
          <div className="device-continuation__device-list">
            {devices.map((device) => {
              const instanceCount = device.profiles.reduce(
                (count, profile) => count + profile.instances.length,
                0,
              );
              const current = device.deviceId === sourceTarget?.execution.deviceId;
              return (
                <button
                  key={device.deviceId}
                  type="button"
                  className={`device-continuation__device ${selectedDeviceId === device.deviceId ? 'is-selected' : ''}`}
                  disabled={current}
                  aria-pressed={selectedDeviceId === device.deviceId}
                  onClick={() => setSelectedDeviceId(device.deviceId)}
                >
                  <DeviceIcon device={device} />
                  <span className="device-continuation__device-copy">
                    <strong>{device.deviceName}</strong>
                    <small>
                      {current
                        ? 'Current conversation device'
                        : instanceCount
                          ? `${instanceCount} worktree${instanceCount === 1 ? '' : 's'} on this branch`
                          : 'No worktree on this branch'}
                    </small>
                  </span>
                  {current && <span className="device-continuation__current">Current</span>}
                </button>
              );
            })}
          </div>
        </section>
      ) : sourceTarget ? (
        <section className="device-continuation__comparison" aria-label="Worktree comparison">
          <div className="device-continuation__comparison-heading">
            <button
              type="button"
              className="worktree-review__secondary"
              onClick={() => setStep('device')}
            >
              <ArrowLeft size={16} aria-hidden="true" /> Back to devices
            </button>
            <p>Branch context and same-branch worktrees on {selectedDevice?.deviceName}.</p>
          </div>
          <div className="device-continuation__branch-context">
            <header>
              <p className="worktree-review__step">Fixed conversation branch</p>
              <h3>{sourceBranch}</h3>
              <p>
                The graph stays on this branch. It provides repository context while the comparison
                stays focused on its worktrees.
              </p>
            </header>
            {fixedBranch && (
              <BranchGraphBrowser
                source={source}
                repositoryId={sourceTarget.repositoryId}
                selectedTarget={fixedBranch}
                contextualTarget={fixedBranch}
                branchesOnly
              />
            )}
          </div>
          <div className="device-continuation__worktree-context">
            <section className="device-continuation__worktree-summary">
              <p className="worktree-review__step">Source worktree</p>
              <strong>{sourceTarget.execution.deviceName}</strong>
              <code>{sourceTarget.path}</code>
              <span>HEAD {shortCommit(sourceTarget.head)}</span>
              <p>Local changes and Orchid turn activity are not reported by this prototype.</p>
            </section>
            <section className="device-continuation__target-worktrees">
              <header>
                <div>
                  <p className="worktree-review__step">Target worktrees</p>
                  <h3>{selectedDevice?.deviceName}</h3>
                </div>
                <span>{targetInstances.length} found</span>
              </header>
              {targetInstances.length === 0 ? (
                <div className="device-continuation__no-target">
                  <strong>No worktree exists on this branch.</strong>
                  <p>
                    A future continuation request can create a sister worktree from the source
                    state.
                  </p>
                </div>
              ) : (
                <div className="device-continuation__worktree-list">
                  {targetInstances.map(({ instance, profile }) => (
                    <article
                      key={`${profile.capabilityProfileId}:${instance.worktreeId}`}
                      className="device-continuation__worktree"
                    >
                      <div>
                        <strong>{instance.path.split(/[\\/]/).filter(Boolean).at(-1)}</strong>
                        <code>{instance.path}</code>
                      </div>
                      <div className="device-continuation__worktree-facts">
                        <TargetState source={sourceTarget} target={instance} />
                        <code>HEAD {shortCommit(instance.head)}</code>
                      </div>
                      <p>Potential sister worktree · relationship and locks are not tracked yet.</p>
                    </article>
                  ))}
                </div>
              )}
            </section>
          </div>
        </section>
      ) : null}

      <footer className="device-continuation__footer">
        <p>
          {sourceTarget
            ? 'The comparison is a preview. It does not copy files or create a snapshot.'
            : 'Choose a worktree after selecting a remote device.'}
        </p>
        {step === 'device' ? (
          <button
            type="button"
            className="worktree-review__primary"
            disabled={!selectedDeviceId}
            onClick={() => {
              if (sourceTarget) setStep('worktrees');
              else if (selectedDeviceId) {
                onChooseDevice?.(selectedDeviceId);
                onClose();
              }
            }}
          >
            {sourceTarget ? 'Select device' : 'Use device'}
          </button>
        ) : (
          <button
            type="button"
            className="worktree-review__primary"
            onClick={() => {
              if (selectedDeviceId) onChooseDevice?.(selectedDeviceId);
              onClose();
            }}
          >
            Use device
          </button>
        )}
      </footer>
    </ModalDialog>
  );
}
