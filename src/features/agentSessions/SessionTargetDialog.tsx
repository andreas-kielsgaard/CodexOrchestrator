import {
  WorktreeCreationConfirmation,
  type WorktreeCreationRequest,
} from './WorktreeCreationConfirmation';
import { Monitor, Server, Check, X } from 'lucide-react';
import { useEffect, useMemo, useState } from 'react';
import { ModalDialog } from '../../components/ModalDialog';
import { BranchBrowser } from '../branches/BranchBrowser';
import { BranchGraphBrowser } from '../branches/BranchGraphBrowser';
import type {
  RepositoryBranchSource,
  ReviewBranch,
  ReviewTarget,
} from '../../application/branches';
import type { RegisteredRepository } from '../../application/repositoryCatalog';
import type {
  ExecutionTargetClient,
  ExecutionTargetDeviceDto,
  SessionExecutionTargetDto,
  SessionExecutionSelectionDto,
} from '../../application/executionTargets/contracts';
import { agentProviderLabel } from '../../application/agentProviders';
import { toSessionExecutionTarget } from '../../application/executionTargets/presentation';
import { orderTargetBranches } from '../../application/executionTargets/presentation';
import type { ProfileWorktreeTargetsDto } from '../../application/executionTargets/contracts';
import '../worktreeReview/worktreeReview.css';
export function SessionTargetDialog({
  client,
  source,
  selected,
  onClose,
  onSelect,
  deviceId,
  capabilityProfileId,
  sessionId,
  onSelectSelection,
  initialRepositoryId,
}: {
  readonly deviceId?: string | null;
  readonly capabilityProfileId?: string | null;
  readonly sessionId?: string;
  readonly onSelectSelection?: (selection: SessionExecutionSelectionDto) => void;
  readonly client: ExecutionTargetClient;
  readonly source: RepositoryBranchSource;
  readonly selected: SessionExecutionTargetDto | null;
  readonly onClose: () => void;
  readonly onSelect: (target: SessionExecutionTargetDto | null) => void;
  readonly initialRepositoryId?: string;
}) {
  const [creation, setCreation] = useState<WorktreeCreationRequest | null>(null);
  const [confirmedBranchKey, setConfirmedBranchKey] = useState('');
  const [repositories, setRepositories] = useState<readonly RegisteredRepository[]>([]);
  const [loadingRepositories, setLoadingRepositories] = useState(true);
  const [repositoryId, setRepositoryId] = useState(
    selected?.repositoryId ?? initialRepositoryId ?? '',
  );
  const [branches, setBranches] = useState<readonly ReviewBranch[]>([]);
  const [referenceTarget, setReferenceTarget] = useState<ReviewTarget | null>(null);
  const [inventoryProfiles, setInventoryProfiles] = useState<readonly ProfileWorktreeTargetsDto[]>(
    [],
  );
  const [branchRef, setBranchRef] = useState(selected?.branchRef ?? '');
  const [devices, setDevices] = useState<readonly ExecutionTargetDeviceDto[]>([]);
  const [candidate, setCandidate] = useState(selected);
  const [graphOpen, setGraphOpen] = useState(false);
  const [loadingBranches, setLoadingBranches] = useState(false);
  const [loadingDevices, setLoadingDevices] = useState(false);
  const [repositoryError, setRepositoryError] = useState<string | null>(null);
  const [deviceError, setDeviceError] = useState<string | null>(null);
  const [refresh, setRefresh] = useState(0);
  useEffect(() => {
    let current = true;
    setLoadingRepositories(true);
    void source
      .listRepositories()
      .then(
        (value) => {
          if (current) setRepositories(value);
        },
        (error) => {
          if (current) setRepositoryError(String(error));
        },
      )
      .finally(() => {
        if (current) setLoadingRepositories(false);
      });
    return () => {
      current = false;
    };
  }, [source]);
  useEffect(() => {
    let current = true;
    setBranches([]);
    setLoadingBranches(false);
    setRepositoryError(null);
    if (!repositoryId) return;
    setLoadingBranches(true);
    void source
      .branchGraph(repositoryId)
      .then(
        (graph) => {
          if (current) {
            setBranches(graph.targets.filter((branch) => branch.target.kind === 'branch'));
            setReferenceTarget(graph.referenceTarget);
            const defaultBranch =
              graph.referenceTarget?.kind === 'branch' ? graph.referenceTarget.branchRef : null;
            if (defaultBranch) setBranchRef((currentBranch) => currentBranch || defaultBranch);
          }
        },
        (error) => {
          if (current) setRepositoryError(String(error));
        },
      )
      .finally(() => {
        if (current) setLoadingBranches(false);
      });
    return () => {
      current = false;
    };
  }, [source, repositoryId]);
  useEffect(() => {
    let current = true;
    setInventoryProfiles([]);
    if (!repositoryId) return;
    const scope = deviceId ? { kind: 'device' as const, deviceId } : { kind: 'local' as const };
    void client.listWorktreeChoices(scope).then(
      (choices) => {
        if (current)
          setInventoryProfiles(
            choices.find((choice) => choice.repositoryId === repositoryId)?.profiles ?? [],
          );
      },
      () => {
        if (current) setInventoryProfiles([]);
      },
    );
    return () => {
      current = false;
    };
  }, [client, deviceId, repositoryId]);
  useEffect(() => {
    let current = true;
    setDevices([]);
    setLoadingDevices(false);
    setDeviceError(null);
    if (!repositoryId || !branchRef) return;
    setLoadingDevices(true);
    void client
      .listTargets(repositoryId, branchRef)
      .then(
        (value) => {
          if (current) setDevices(value);
        },
        (error) => {
          if (current) setDeviceError(String(error));
        },
      )
      .finally(() => {
        if (current) setLoadingDevices(false);
      });
    return () => {
      current = false;
    };
  }, [client, repositoryId, branchRef, refresh]);
  useEffect(() => {
    if (candidate || loadingDevices || !branchRef) return;
    const matches = devices
      .filter((device) => !deviceId || device.deviceId === deviceId)
      .flatMap((device) => device.profiles)
      .filter(
        (item) =>
          !item.error && (!capabilityProfileId || item.capabilityProfileId === capabilityProfileId),
      )
      .flatMap((profile) => profile.instances.map((instance) => ({ profile, instance })))
      .filter(
        ({ instance }) => !instance.sisterLock || instance.sisterLock.ownerSessionId === sessionId,
      );
    if (matches.length === 1)
      setCandidate(toSessionExecutionTarget(repositoryId, matches[0].profile, matches[0].instance));
  }, [branchRef, candidate, capabilityProfileId, deviceId, devices, loadingDevices, repositoryId]);
  useEffect(() => {
    if (!deviceId || !onSelectSelection || loadingDevices || !branchRef) return;
    const key = `${repositoryId}:${branchRef}:${deviceId}`;
    if (confirmedBranchKey === key) return;
    const matching =
      devices
        .find((device) => device.deviceId === deviceId)
        ?.profiles.filter(
          (profile) => !capabilityProfileId || profile.capabilityProfileId === capabilityProfileId,
        ) ?? [];
    if (matching.length !== 1 || matching[0].error || matching[0].instances.length) return;
    setConfirmedBranchKey(key);
    setCreation({
      repositoryId,
      repositoryName:
        repositories.find((repo) => repo.repositoryId === repositoryId)?.name ?? repositoryId,
      branchRef,
      profile: matching[0],
    });
  }, [
    deviceId,
    onSelectSelection,
    loadingDevices,
    branchRef,
    repositoryId,
    confirmedBranchKey,
    devices,
    capabilityProfileId,
    repositories,
  ]);
  const selectedBranch = useMemo<ReviewTarget | null>(
    () => (branchRef ? { kind: 'branch', repositoryId, branchRef } : null),
    [repositoryId, branchRef],
  );
  const orderedBranches = useMemo(
    () => orderTargetBranches(branches, referenceTarget, inventoryProfiles),
    [branches, inventoryProfiles, referenceTarget],
  );
  const candidateAvailable =
    candidate &&
    devices.some((device) =>
      device.profiles.some(
        (profile) =>
          !profile.error &&
          profile.capabilityProfileId === candidate.capabilityProfileId &&
          profile.instances.some((instance) => instance.worktreeId === candidate.worktreeId),
      ),
    );
  const candidateLocked =
    candidate &&
    devices.some((device) =>
      device.profiles.some((profile) =>
        profile.instances.some(
          (instance) =>
            instance.worktreeId === candidate.worktreeId &&
            instance.sisterLock &&
            instance.sisterLock.ownerSessionId !== sessionId,
        ),
      ),
    );

  function chooseBranch(target: ReviewTarget) {
    if (target.kind !== 'branch') return;
    setBranchRef(target.branchRef);
    setCandidate(null);
    setGraphOpen(false);
  }
  return (
    <>
      <ModalDialog
        labelledBy="session-target-title"
        className="session-target-dialog"
        onClose={onClose}
      >
        <header className="session-target-dialog__header">
          <div>
            <p className="worktree-review__step">Next message</p>
            <h2 id="session-target-title">Target worktree</h2>
            <p>Choose where the next prompt will run.</p>
          </div>
          <button type="button" onClick={onClose} aria-label="Close target selector">
            <X size={18} />
          </button>
        </header>
        <label className="worktree-review__field">
          <span>Repository / project</span>
          <select
            aria-label="Target repository"
            value={repositoryId}
            onChange={(event) => {
              setRepositoryId(event.target.value);
              setBranchRef('');
              setCandidate(null);
              setGraphOpen(false);
            }}
          >
            <option value="">Choose a repository…</option>
            {repositories.map((repo) => (
              <option key={repo.repositoryId} value={repo.repositoryId}>
                {repo.name} — {repo.locationLabel}
              </option>
            ))}
          </select>
        </label>
        {repositoryError && <p role="alert">{repositoryError}</p>}
        {!repositoryId ? (
          <p className="session-target-dialog__placeholder">
            {loadingRepositories
              ? 'Loading registered repositories…'
              : repositories.length === 0
                ? 'No repositories are registered. Add a repository in Worktree Review, then reopen this selector.'
                : 'Select a registered repository to browse its branches.'}
          </p>
        ) : graphOpen ? (
          <div className="session-target-dialog__graph">
            <button
              type="button"
              className="worktree-review__secondary"
              onClick={() => setGraphOpen(false)}
            >
              Back to worktrees
            </button>
            <BranchGraphBrowser
              source={source}
              repositoryId={repositoryId}
              selectedTarget={selectedBranch}
              branchesOnly
              onSelect={chooseBranch}
            />
          </div>
        ) : (
          <div className="session-target-dialog__body">
            <aside>
              {loadingBranches ? (
                <p role="status">Loading branches…</p>
              ) : (
                <BranchBrowser
                  repositoryId={repositoryId}
                  branches={orderedBranches}
                  selectedTarget={selectedBranch}
                  onBranchChange={chooseBranch}
                  onOpenGraph={() => setGraphOpen(true)}
                />
              )}
            </aside>
            <section className="session-target-dialog__devices" aria-label="Worktrees by device">
              <header>
                <div>
                  <h3>Devices and worktrees</h3>
                  <p>
                    {branchRef
                      ? branchRef.replace(/^refs\/heads\//, '')
                      : 'Choose a branch to see its worktrees.'}
                  </p>
                </div>
                {branchRef && (
                  <button
                    type="button"
                    disabled={loadingDevices}
                    onClick={() => {
                      setCandidate(null);
                      setRefresh((value) => value + 1);
                    }}
                  >
                    Refresh
                  </button>
                )}
              </header>
              {loadingDevices && <p role="status">Finding existing worktrees…</p>}
              {deviceError && <p role="alert">{deviceError}</p>}
              {!loadingDevices && branchRef && !deviceError && devices.length === 0 && (
                <p>No devices are configured. Add a Capability Profile in Technical Settings.</p>
              )}
              {devices
                .filter((device) => !deviceId || device.deviceId === deviceId)
                .map((device) => (
                  <article
                    className="session-target-device"
                    key={device.deviceId}
                    aria-label={device.deviceName}
                  >
                    <h4>
                      {device.profiles.some(
                        (profile) => profile.execution.connection.kind === 'ssh',
                      ) ? (
                        <Server size={17} />
                      ) : (
                        <Monitor size={17} />
                      )}
                      {device.deviceName}
                    </h4>
                    {device.profiles.map((profile) => (
                      <div className="session-target-profile" key={profile.capabilityProfileId}>
                        <p>
                          {profile.capabilityProfileName} ·{' '}
                          {agentProviderLabel(profile.execution.provider)}
                        </p>
                        {profile.error ? (
                          <p role="status" className="session-target-unavailable">
                            Unavailable: {profile.error}
                          </p>
                        ) : profile.instances.length === 0 ? (
                          <div className="session-target-empty">
                            <p>No existing worktree for this branch.</p>
                            {profile.sisterLock &&
                            profile.sisterLock.ownerSessionId !== sessionId ? (
                              <p className="session-target-unavailable">
                                Locked to a Session on {profile.sisterLock.activeDeviceId}.
                              </p>
                            ) : (
                              onSelectSelection && (
                                <button
                                  type="button"
                                  onClick={() =>
                                    setCreation({
                                      repositoryId,
                                      repositoryName:
                                        repositories.find(
                                          (repo) => repo.repositoryId === repositoryId,
                                        )?.name ?? repositoryId,
                                      branchRef,
                                      profile,
                                    })
                                  }
                                >
                                  Create worktree on Send…
                                </button>
                              )
                            )}
                          </div>
                        ) : (
                          profile.instances.map((instance) => {
                            const active =
                              candidate?.worktreeId === instance.worktreeId &&
                              candidate?.capabilityProfileId === profile.capabilityProfileId;
                            const locked =
                              instance.sisterLock &&
                              instance.sisterLock.ownerSessionId !== sessionId;
                            return (
                              <button
                                key={instance.worktreeId}
                                type="button"
                                className={`session-target-instance${!candidate && branchRef ? ' is-branch-match' : ''}`}
                                aria-pressed={active}
                                disabled={Boolean(locked)}
                                onClick={() =>
                                  setCandidate(
                                    toSessionExecutionTarget(repositoryId, profile, instance),
                                  )
                                }
                              >
                                <span>
                                  <strong>{instance.path}</strong>
                                  <code>HEAD {instance.head?.slice(0, 10) ?? 'unknown'}</code>
                                  {locked && <small>Locked to another Session</small>}
                                </span>
                                {active && <Check size={17} aria-hidden="true" />}
                              </button>
                            );
                          })
                        )}
                      </div>
                    ))}
                  </article>
                ))}
            </section>
          </div>
        )}
        <footer className="session-target-dialog__footer">
          <div>
            {selected && (
              <button
                type="button"
                className="worktree-review__secondary"
                onClick={() => onSelect(null)}
              >
                Clear target
              </button>
            )}
          </div>
          <button
            type="button"
            className="worktree-review__primary"
            disabled={
              !candidateAvailable || Boolean(candidateLocked) || loadingDevices || graphOpen
            }
            onClick={() => candidateAvailable && candidate && onSelect(candidate)}
          >
            Use worktree
          </button>
        </footer>
      </ModalDialog>
      {creation && onSelectSelection && (
        <WorktreeCreationConfirmation
          client={client}
          request={creation}
          onCancel={() => setCreation(null)}
          onConfirm={onSelectSelection}
        />
      )}
    </>
  );
}
