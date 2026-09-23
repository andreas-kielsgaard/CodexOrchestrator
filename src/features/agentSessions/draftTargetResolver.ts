import type { RepositoryBranchSource } from '../../application/branches';
import type {
  ExecutionTargetClient,
  SessionExecutionSelectionDto,
} from '../../application/executionTargets/contracts';
import { toSessionExecutionTarget } from '../../application/executionTargets/presentation';

export interface DraftBranchChoice {
  readonly repositoryId: string;
  readonly branchRef: string;
  readonly label: string;
  readonly requiresWorktree: boolean;
}

export async function resolveDefaultDraftTarget(input: {
  readonly source: RepositoryBranchSource;
  readonly client: ExecutionTargetClient;
  readonly repositoryId: string;
  readonly selection: SessionExecutionSelectionDto;
  readonly sessionId?: string;
}): Promise<{
  readonly selection: SessionExecutionSelectionDto;
  readonly branchChoice: DraftBranchChoice | null;
}> {
  const graph = await input.source.branchGraph(input.repositoryId);
  if (graph.referenceTarget?.kind !== 'branch')
    throw new Error('The repository does not expose a default branch.');
  const branchRef = graph.referenceTarget.branchRef;
  const branch = graph.targets.find(
    (item) => item.target.kind === 'branch' && item.target.branchRef === branchRef,
  );
  if (!branch) throw new Error('The default branch is absent from the repository graph.');
  const devices = await input.client.listTargets(input.repositoryId, branchRef);
  const profiles = devices
    .filter((device) => device.deviceId === input.selection.execution.deviceId)
    .flatMap((device) => device.profiles)
    .filter(
      (profile) =>
        profile.capabilityProfileId === input.selection.capabilityProfileId && !profile.error,
    );
  const matches = profiles
    .flatMap((profile) => profile.instances.map((instance) => ({ profile, instance })))
    .filter(
      ({ instance }) =>
        !instance.sisterLock || instance.sisterLock.ownerSessionId === input.sessionId,
    );
  if (matches.length === 1) {
    const target = toSessionExecutionTarget(
      input.repositoryId,
      matches[0].profile,
      matches[0].instance,
    );
    return {
      selection: {
        capabilityProfileId: target.capabilityProfileId,
        capabilityProfileRevision: target.capabilityProfileRevision,
        execution: target.execution,
        workspace: { kind: 'existing', target },
      },
      branchChoice: null,
    };
  }
  const label = branchRef.replace(/^refs\/heads\//, '');
  if (matches.length > 1) {
    return {
      selection: input.selection,
      branchChoice: {
        repositoryId: input.repositoryId,
        branchRef,
        label: `${label} · choose worktree`,
        requiresWorktree: true,
      },
    };
  }
  if (profiles.length !== 1)
    throw new Error('The selected Capability Profile has no route to this repository.');
  return {
    selection: {
      ...input.selection,
      workspace: {
        kind: 'create',
        repositoryId: input.repositoryId,
        branchRef,
        commit: branch.tip.objectId,
        attachment: 'branch',
      },
    },
    branchChoice: null,
  };
}
