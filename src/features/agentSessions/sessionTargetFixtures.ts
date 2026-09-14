import type { RepositoryBranchSource } from '../../application/branches';
import type {
  ExecutionBindingDto,
  ExecutionTargetClient,
  ExecutionTargetDeviceDto,
  SessionExecutionTargetDto,
} from '../../application/executionTargets/contracts';
import { localExecutionBinding } from '../../application/executionTargets/contracts';
import { overviewFixture, tipCommit } from '../worktreeReview/WorktreeReviewScreen.fixtures';
import { repairRuntime } from '../workflowAuthoring/testFixtures';
export const remoteExecution: ExecutionBindingDto = {
  deviceId: 'remote',
  deviceName: 'Remote server',
  provider: 'codex',
  configurationRef: 'codex-default',
  connection: {
    kind: 'ssh',
    target: 'orchid-remote',
    hostExecutable: '/root/.local/bin/orchid-host',
  },
};
export const remoteTarget: SessionExecutionTargetDto = {
  capabilityProfileId: 'review',
  capabilityProfileRevision: 1,
  execution: remoteExecution,
  repositoryId: 'repository-one',
  branchRef: 'refs/heads/codex/durable-review',
  worktreeId: 'remote-worktree',
  path: '/root/projects/orchid/worktrees/review',
  head: tipCommit.objectId,
};
export function sessionTargetFixtures() {
  const source: RepositoryBranchSource = {
    listRepositories: vi.fn(async () => overviewFixture.repositories),
    branchGraph: vi.fn(async () => ({
      snapshotId: 'snapshot',
      referenceTarget: null,
      targets: overviewFixture.branches,
      anchors: overviewFixture.branches
        .map((branch) => ({
          objectId: branch.tip.objectId,
          parentIds: [],
          boundary: false,
          merge: false,
        }))
        .filter(
          (anchor, index, array) =>
            array.findIndex((item) => item.objectId === anchor.objectId) === index,
        ),
      connections: [],
      hasMore: false,
      loadedCommitCount: 2,
    })),
  };
  const devices: ExecutionTargetDeviceDto[] = [
    {
      deviceId: 'local',
      deviceName: 'This laptop',
      profiles: [
        {
          capabilityProfileId: 'local-profile',
          capabilityProfileRevision: 1,
          capabilityProfileName: 'Laptop Codex',
          execution: localExecutionBinding,
          instances: [],
          error: null,
        },
      ],
    },
    {
      deviceId: 'remote',
      deviceName: 'Remote server',
      profiles: [
        {
          capabilityProfileId: 'review',
          capabilityProfileRevision: 1,
          capabilityProfileName: 'Remote Codex',
          execution: remoteExecution,
          instances: [
            {
              worktreeId: remoteTarget.worktreeId,
              path: remoteTarget.path,
              head: remoteTarget.head,
              branchRef: remoteTarget.branchRef,
            },
          ],
          error: null,
        },
      ],
    },
  ];
  const client: ExecutionTargetClient = {
    listTargets: vi.fn(async () => devices),
    loadRuntime: vi.fn(async () => ({
      runtimeProfile: repairRuntime,
      nativeInventory: { entries: [], limitations: [] },
    })),
    listRepositoryLocations: vi.fn(async () => []),
    saveRepositoryLocation: vi.fn(async () => {}),
  };
  return { source, client, devices };
}
