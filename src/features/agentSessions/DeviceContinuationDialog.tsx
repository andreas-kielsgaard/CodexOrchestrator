import { TargetSwitchChoice } from './TargetSwitchChoice';
import { targetSwitchRequest } from './targetSwitchRequest';
import type {
  TargetSwitchRequestDto,
  TargetSwitchIntentDto,
  MigrationStrategyDto,
} from '../../application/agentSessions/sessionQueue';
import type { RepositoryBranchSource, ReviewTarget } from '../../application/branches';
import type {
  ExecutionTargetClient,
  ExecutionTargetDeviceDto,
  SessionExecutionSelectionDto,
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
  readonly sessionId?: string;
  readonly source: RepositoryBranchSource;
  /** The fixed source worktree of the conversation being continued. */
  readonly sourceTarget: SessionExecutionTargetDto | null;
  readonly onChooseDevice?: (deviceId: string) => void;
  readonly onQueueSelection?: (request: TargetSwitchRequestDto) => Promise<void> | void;
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

function matchingWorktrees(device: ExecutionTargetDeviceDto | null) {
  return (
    device?.profiles.flatMap((profile) =>
      profile.instances.map((instance) => ({ instance, profile })),
    ) ?? []
  );
}

/** A device switch chooses the destination recorded by the Session queue. */
export function DeviceContinuationDialog({
  client,
  sessionId,
  source,
  sourceTarget,
  onChooseDevice,
  onQueueSelection,
  onClose,
}: DeviceContinuationDialogProps) {
  const [intent, setIntent] = useState<TargetSwitchIntentDto>('change_conversation_target');
  const [strategy, setStrategy] = useState<MigrationStrategyDto | ''>('');
  const [devices, setDevices] = useState<readonly ExecutionTargetDeviceDto[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string>();
  const [selectedDeviceId, setSelectedDeviceId] = useState<string>();
  const [selectedWorktreeKey, setSelectedWorktreeKey] = useState<string | 'create'>();
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
        (resolved) => {
          if (!cancelled) setDevices(resolved);
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
  const targetInstances = matchingWorktrees(selectedDevice);
  const sibling = targetInstances.find(({ instance }) => instance.isSister) ?? null;
  const siblingKey = sibling
    ? `${sibling.profile.capabilityProfileId}:${sibling.profile.capabilityProfileRevision}:${sibling.instance.worktreeId}`
    : undefined;
  const targetBranchLockedElsewhere =
    selectedDevice?.profiles.some(
      (profile) => profile.sisterLock && profile.sisterLock.ownerSessionId !== sessionId,
    ) ?? false;
  const sourceBranch =
    sourceTarget?.branchRef.replace(/^refs\/heads\//, '') ?? 'No source worktree';
  const chosen = targetInstances.find(
    ({ instance, profile }) =>
      `${profile.capabilityProfileId}:${profile.capabilityProfileRevision}:${instance.worktreeId}` ===
      selectedWorktreeKey,
  );
  useEffect(() => {
    if (siblingKey) setSelectedWorktreeKey(siblingKey);
  }, [siblingKey]);
  const destination = useMemo<SessionExecutionSelectionDto | null>(() => {
    if (!sourceTarget || !selectedDevice) return null;
    if (chosen) {
      const { instance, profile } = chosen;
      return {
        capabilityProfileId: profile.capabilityProfileId,
        capabilityProfileRevision: profile.capabilityProfileRevision,
        execution: profile.execution,
        workspace: {
          kind: 'existing',
          target: {
            capabilityProfileId: profile.capabilityProfileId,
            capabilityProfileRevision: profile.capabilityProfileRevision,
            execution: profile.execution,
            repositoryId: sourceTarget.repositoryId,
            branchRef: sourceTarget.branchRef,
            worktreeId: instance.worktreeId,
            path: instance.path,
            head: instance.head,
          },
        },
      };
    }
    if (selectedWorktreeKey === 'create') {
      const profile = selectedDevice.profiles[0];
      if (!profile) return null;
      return {
        capabilityProfileId: profile.capabilityProfileId,
        capabilityProfileRevision: profile.capabilityProfileRevision,
        execution: profile.execution,
        workspace: {
          kind: 'create',
          repositoryId: sourceTarget.repositoryId,
          branchRef: sourceTarget.branchRef,
          commit: sourceTarget.head ?? '',
          attachment: 'branch',
        },
      };
    }
    return null;
  }, [chosen, selectedDevice, selectedWorktreeKey, sourceTarget]);

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
          <span>2</span> Choose worktree
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
              ? 'Select a device, then compare its same-branch worktrees with the current conversation worktree.'
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
                  onClick={() => {
                    setSelectedDeviceId(device.deviceId);
                    setSelectedWorktreeKey(undefined);
                  }}
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
            <p>
              Compare the fixed branch and choose the destination worktree on{' '}
              {selectedDevice?.deviceName}.
            </p>
          </div>
          <div className="device-continuation__branch-context">
            <header>
              <p className="worktree-review__step">Fixed conversation branch</p>
              <h3>{sourceBranch}</h3>
              <p>
                The graph gives repository context; the decisions here apply only to its worktrees.
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
              <p>Source state is captured only if the selected destination needs migration.</p>
            </section>
            <section className="device-continuation__target-worktrees">
              <header>
                <div>
                  <p className="worktree-review__step">Target worktrees</p>
                  <h3>{selectedDevice?.deviceName}</h3>
                </div>
                <span>{targetInstances.length} found</span>
              </header>
              <div
                className="device-continuation__worktree-list"
                role="radiogroup"
                aria-label="Destination worktree"
              >
                {targetInstances.map(({ instance, profile }) => {
                  const key = `${profile.capabilityProfileId}:${profile.capabilityProfileRevision}:${instance.worktreeId}`;
                  const lockedElsewhere =
                    instance.sisterLock && instance.sisterLock.ownerSessionId !== sessionId;
                  const differentFromSibling = Boolean(sibling && !instance.isSister);
                  const disabled = Boolean(lockedElsewhere);
                  return (
                    <button
                      key={key}
                      type="button"
                      role="radio"
                      aria-checked={selectedWorktreeKey === key}
                      className={`device-continuation__worktree ${selectedWorktreeKey === key ? 'is-selected' : ''}`}
                      disabled={disabled}
                      onClick={() => setSelectedWorktreeKey(key)}
                    >
                      <div>
                        <strong>{instance.path.split(/[\\/]/).filter(Boolean).at(-1)}</strong>
                        <code>{instance.path}</code>
                      </div>
                      <div className="device-continuation__worktree-facts">
                        <TargetState source={sourceTarget} target={instance} />
                        <code>HEAD {shortCommit(instance.head)}</code>
                      </div>
                      {lockedElsewhere && (
                        <p>Locked to a Session on {instance.sisterLock?.activeDeviceId}</p>
                      )}
                      {differentFromSibling && (
                        <p>This worktree can be linked explicitly when continuing work.</p>
                      )}
                    </button>
                  );
                })}
                <button
                  type="button"
                  role="radio"
                  aria-checked={selectedWorktreeKey === 'create'}
                  className={`device-continuation__worktree device-continuation__worktree--create ${selectedWorktreeKey === 'create' ? 'is-selected' : ''}`}
                  disabled={targetBranchLockedElsewhere}
                  onClick={() => setSelectedWorktreeKey('create')}
                >
                  <strong>Create a new worktree</strong>
                  <span>
                    {sibling
                      ? 'A sister worktree already exists on this device.'
                      : targetBranchLockedElsewhere
                        ? 'This branch is locked to another Session.'
                        : `Start from source HEAD ${shortCommit(sourceTarget.head)}. Continuing work also links and migrates its state.`}
                  </span>
                </button>
              </div>
            </section>
          </div>
        </section>
      ) : null}

      {step === 'worktrees' && (
        <TargetSwitchChoice
          destination={destination}
          source={sourceTarget}
          intent={intent}
          strategy={strategy}
          isSister={Boolean(
            chosen?.instance.isSister &&
            devices.some(
              (device) =>
                device.deviceId === sourceTarget?.execution.deviceId &&
                device.profiles.some((profile) =>
                  profile.instances.some(
                    (instance) =>
                      instance.worktreeId === sourceTarget?.worktreeId && instance.isSister,
                  ),
                ),
            ),
          )}
          onIntent={setIntent}
          onStrategy={setStrategy}
        />
      )}
      <footer className="device-continuation__footer">
        <p>
          {onQueueSelection
            ? 'The switch is added to the Session queue. It is evaluated and prepared when it reaches the front.'
            : 'Choose the device for this conversation target.'}
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
            disabled={
              !destination || !targetSwitchRequest(destination, sourceTarget, intent, strategy)
            }
            onClick={() => {
              if (onQueueSelection && destination) {
                const request = targetSwitchRequest(destination, sourceTarget, intent, strategy);
                if (!request) return;
                void onQueueSelection(request);
                onClose();
              } else if (selectedDeviceId) {
                onChooseDevice?.(selectedDeviceId);
                onClose();
              }
            }}
          >
            {onQueueSelection ? 'Queue device switch' : 'Use device'}
          </button>
        )}
      </footer>
    </ModalDialog>
  );
}
