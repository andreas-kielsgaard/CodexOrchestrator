import type {
  ExecutionBindingDto,
  SessionExecutionTargetDto,
} from './contracts';

export function toSessionExecutionTarget(
  repositoryId: string,
  profile: {
    readonly capabilityProfileId: string;
    readonly capabilityProfileRevision: number;
    readonly execution: ExecutionBindingDto;
  },
  instance: Pick<SessionExecutionTargetDto, 'worktreeId' | 'path' | 'head' | 'branchRef'>,
): SessionExecutionTargetDto {
  return {
    repositoryId,
    capabilityProfileId: profile.capabilityProfileId,
    capabilityProfileRevision: profile.capabilityProfileRevision,
    execution: profile.execution,
    worktreeId: instance.worktreeId,
    path: instance.path,
    head: instance.head,
    branchRef: instance.branchRef,
  };
}

export function presentExecutionTarget(
  target: SessionExecutionTargetDto,
  repositoryName: string,
  profileName: string,
) {
  const branch = target.branchRef.replace(/^refs\/heads\//, '');
  // A remote path must be rendered without applying the laptop's path rules.
  const directory = target.path.split(/[\\/]/).filter(Boolean).at(-1) ?? target.path;
  return {
    id: JSON.stringify([
      target.repositoryId,
      target.execution.deviceId,
      target.capabilityProfileId,
      target.worktreeId,
    ]),
    label: `${branch} · ${directory}`,
    description: `${repositoryName} · ${profileName} · ${target.path} · #${target.worktreeId.slice(-10)}`,
    keywords: [
      branch,
      target.branchRef,
      directory,
      repositoryName,
      profileName,
      target.path,
      target.worktreeId,
      target.execution.deviceName,
    ],
  };
}
