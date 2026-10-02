import type { RepositoryBranchSource } from '../../application/branches';
import type {
  ExecutionTargetClient,
  SessionExecutionSelectionDto,
  TargetWorktreeDto,
} from '../../application/executionTargets/contracts';
import { localExecutionBinding } from '../../application/executionTargets/contracts';
import { resolveDefaultDraftTarget } from './draftTargetResolver';

const selection: SessionExecutionSelectionDto = {
  capabilityProfileId: 'profile',
  capabilityProfileRevision: 2,
  execution: localExecutionBinding,
  workspace: { kind: 'auxiliary' },
};
const source: RepositoryBranchSource = {
  listRepositories: vi.fn(async () => []),
  branchGraph: vi.fn(async () => ({
    snapshotId: 'snapshot',
    referenceTarget: {
      kind: 'branch' as const,
      repositoryId: 'repo',
      branchRef: 'refs/heads/main',
    },
    targets: [
      {
        target: {
          kind: 'branch' as const,
          repositoryId: 'repo',
          branchRef: 'refs/heads/main',
        },
        availableWorktreeCount: 0,
        worktreeIds: [],
        activity: null,
        repositoryId: 'repo',
        branchRef: 'refs/heads/main',
        displayName: 'main',
        tip: {
          objectId: 'abc123',
          abbreviatedObjectId: 'abc123',
          subject: 'tip',
          author: 'test',
          committedAt: '2026-09-23T00:00:00Z',
        },
        aheadOfDefault: 0,
        behindDefault: 0,
        associatedWorktreeCount: 0,
      },
    ],
    anchors: [],
    connections: [],
    hasMore: false,
    loadedCommitCount: 1,
  })),
};

function client(instances: readonly TargetWorktreeDto[]): ExecutionTargetClient {
  return {
    listDevices: vi.fn(async () => []),
    listWorktreeChoices: vi.fn(async () => []),
    listTargets: vi.fn(async () => [
      {
        deviceId: 'local',
        deviceName: 'This laptop',
        profiles: [
          {
            capabilityProfileId: 'profile',
            capabilityProfileRevision: 2,
            capabilityProfileName: 'Profile',
            execution: localExecutionBinding,
            instances,
            error: null,
          },
        ],
      },
    ]),
    loadRuntime: vi.fn(),
    listRepositoryLocations: vi.fn(async () => []),
    saveRepositoryLocation: vi.fn(async () => undefined),
  };
}

const worktree = (id: string): TargetWorktreeDto => ({
  worktreeId: id,
  path: `C:/worktrees/${id}`,
  head: 'abc123',
  branchRef: 'refs/heads/main',
});

it('selects the sole matching worktree', async () => {
  const result = await resolveDefaultDraftTarget({
    source,
    client: client([worktree('only')]),
    repositoryId: 'repo',
    selection,
  });
  expect(result.selection.workspace).toMatchObject({
    kind: 'existing',
    target: { worktreeId: 'only' },
  });
  expect(result.branchChoice).toBeNull();
});
it('uses an explicitly requested branch instead of the repository default', async () => {
  const graph = await source.branchGraph('repo');
  const requestedSource: RepositoryBranchSource = {
    ...source,
    branchGraph: vi.fn(async () => ({
      ...graph,
      targets: [
        ...graph.targets,
        {
          ...graph.targets[0],
          target: {
            kind: 'branch' as const,
            repositoryId: 'repo',
            branchRef: 'refs/heads/feature',
          },
          branchRef: 'refs/heads/feature',
          displayName: 'feature',
        },
      ],
    })),
  };
  const api = client([{ ...worktree('feature-tree'), branchRef: 'refs/heads/feature' }]);
  const result = await resolveDefaultDraftTarget({
    source: requestedSource,
    client: api,
    repositoryId: 'repo',
    selection,
    branchRef: 'refs/heads/feature',
  });
  expect(api.listTargets).toHaveBeenCalledWith('repo', 'refs/heads/feature');
  expect(result.selection.workspace).toMatchObject({
    kind: 'existing',
    target: { branchRef: 'refs/heads/feature' },
  });
});

it('creates the default branch on send when no worktree exists', async () => {
  const result = await resolveDefaultDraftTarget({
    source,
    client: client([]),
    repositoryId: 'repo',
    selection,
  });
  expect(result.selection.workspace).toEqual({
    kind: 'create',
    repositoryId: 'repo',
    branchRef: 'refs/heads/main',
    commit: 'abc123',
    attachment: 'branch',
  });
});

it('displays the default branch but requires a choice when several worktrees match', async () => {
  const result = await resolveDefaultDraftTarget({
    source,
    client: client([worktree('one'), worktree('two')]),
    repositoryId: 'repo',
    selection,
  });
  expect(result.selection.workspace.kind).toBe('auxiliary');
  expect(result.branchChoice).toEqual({
    repositoryId: 'repo',
    branchRef: 'refs/heads/main',
    label: 'main · choose worktree',
    requiresWorktree: true,
  });
});
