import { useEffect, useRef, useState } from 'react';
import { ModalDialog } from '../../components/ModalDialog';
import type { AgentSessionClient } from '../../application/agentSessions';
import type {
  ConfiguredExecutionDeviceDto,
  ExecutionTargetClient,
  SessionExecutionTargetDto,
} from '../../application/executionTargets/contracts';

export function ForkConversationDialog({
  sourceId,
  sessions,
  targets,
  onCreated,
  onClose,
}: {
  sourceId: string;
  sessions: AgentSessionClient;
  targets: ExecutionTargetClient;
  onCreated(id: string): void;
  onClose(): void;
}) {
  const [devices, setDevices] = useState<readonly ConfiguredExecutionDeviceDto[]>([]);
  const [device, setDevice] = useState('');
  const [choices, setChoices] = useState<SessionExecutionTargetDto[]>([]);
  const [selected, setSelected] = useState('');
  const [error, setError] = useState<string>();
  const [busy, setBusy] = useState(false);
  const [loading, setLoading] = useState(false);
  const requests = useRef(new Map<string, string>());
  useEffect(() => {
    let active = true;
    void targets.listDevices().then(
      (rows) => {
        if (active) setDevices(rows);
      },
      (e) => {
        if (active) setError(String(e));
      },
    );
    return () => {
      active = false;
    };
  }, [targets]);
  useEffect(() => {
    let active = true;
    setChoices([]);
    setSelected('');
    setError(undefined);
    if (!device) return;
    setLoading(true);
    void targets
      .listWorktreeChoices({ kind: 'device', deviceId: device })
      .then(
        (repositories) => {
          if (active)
            setChoices(
              repositories.flatMap((repository) =>
                repository.profiles.flatMap((profile) =>
                  profile.instances.map((instance) => ({
                    capabilityProfileId: profile.capabilityProfileId,
                    capabilityProfileRevision: profile.capabilityProfileRevision,
                    execution: profile.execution,
                    repositoryId: repository.repositoryId,
                    branchRef: instance.branchRef,
                    worktreeId: instance.worktreeId,
                    path: instance.path,
                    head: instance.head,
                  })),
                ),
              ),
            );
        },
        (e) => {
          if (active) setError(String(e));
        },
      )
      .finally(() => {
        if (active) setLoading(false);
      });
    return () => {
      active = false;
    };
  }, [device, targets]);
  const fork = async () => {
    const target = choices[Number(selected)];
    if (!target || !sessions.forkConversation) return;
    const key = JSON.stringify(target);
    let id = requests.current.get(key);
    if (!id) {
      id = crypto.randomUUID();
      requests.current.set(key, id);
    }
    setBusy(true);
    setError(undefined);
    try {
      const created = await sessions.forkConversation(sourceId, id, target);
      onCreated(created.id);
      onClose();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };
  return (
    <ModalDialog labelledBy="fork-conversation-title" onClose={onClose}>
      <h2 id="fork-conversation-title">Fork conversation</h2>
      <p>
        Copies completed synchronized turns. Worktree changes on the source device are not included.
      </p>
      <label>
        Execution device{' '}
        <select value={device} disabled={busy} onChange={(e) => setDevice(e.target.value)}>
          <option value="">Choose device</option>
          {devices.map((item) => (
            <option key={item.deviceId} value={item.deviceId}>
              {item.deviceName}
            </option>
          ))}
        </select>
      </label>
      <label>
        New worktree target{' '}
        <select
          value={selected}
          disabled={busy || loading}
          onChange={(e) => setSelected(e.target.value)}
        >
          <option value="">{loading ? 'Loading worktrees…' : 'Choose worktree'}</option>
          {choices.map((target, index) => (
            <option key={`${target.capabilityProfileId}:${target.worktreeId}`} value={index}>
              {target.path} · {target.execution.provider}
            </option>
          ))}
        </select>
      </label>
      {error && <p role="alert">{error}</p>}
      <button type="button" disabled={busy || selected === ''} onClick={() => void fork()}>
        {busy ? 'Creating fork…' : 'Create fork'}
      </button>
      <button type="button" onClick={onClose} disabled={busy}>
        Cancel
      </button>
    </ModalDialog>
  );
}
