import { FolderGit2, LockKeyhole, Monitor, Server } from 'lucide-react';
import { useState } from 'react';
import type { RepositoryBranchSource } from '../../application/branches';
import type {
  ExecutionTargetClient,
  SessionExecutionTargetDto,
} from '../../application/executionTargets/contracts';
import { SessionTargetDialog } from './SessionTargetDialog';
import './sessionTarget.css';
export interface SessionTargetControlProps {
  readonly client: ExecutionTargetClient;
  readonly branches: RepositoryBranchSource;
  readonly target: SessionExecutionTargetDto | null;
  readonly fixed: boolean;
  readonly disabled?: boolean;
  readonly onChange: (target: SessionExecutionTargetDto | null) => void;
}
export function SessionTargetControl({
  client,
  branches,
  target,
  fixed,
  disabled,
  onChange,
}: SessionTargetControlProps) {
  const [open, setOpen] = useState(false);
  const Device = target?.execution.connection.kind === 'ssh' ? Server : Monitor;
  return (
    <div className="session-target-control">
      <button
        type="button"
        disabled={disabled || fixed}
        onClick={() => setOpen(true)}
        title={
          fixed
            ? 'The worktree is fixed for this session.'
            : 'Select the repository, branch and worktree for this session'
        }
      >
        {target ? (
          <Device size={16} aria-hidden="true" />
        ) : (
          <FolderGit2 size={16} aria-hidden="true" />
        )}
        {target
          ? `${target.branchRef.replace(/^refs\/heads\//, '')} · ${target.execution.deviceName}`
          : 'Target worktree'}
        {fixed && <LockKeyhole size={13} aria-hidden="true" />}
      </button>
      {target && (
        <span
          className="session-target-control__path"
          title={`${target.repositoryId} · ${target.capabilityProfileId} · ${target.head ?? 'HEAD unknown'}`}
        >
          {target.path}
        </span>
      )}
      {open && (
        <SessionTargetDialog
          client={client}
          source={branches}
          selected={target}
          onClose={() => setOpen(false)}
          onSelect={(next) => {
            onChange(next);
            setOpen(false);
          }}
        />
      )}
    </div>
  );
}
